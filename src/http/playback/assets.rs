use std::{
    io::SeekFrom,
    path::{Component, Path as FsPath},
};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use super::{ensure_ready, original_exposed, range::parse_range, stream::AnalyticsStream};
use crate::{
    application::media::{self, MediaRecord},
    cookie,
    error::AppError,
    http::client::ClientAddress,
    infrastructure::protection::{AssetClass, PacedStream, StreamPermit},
    state::AppState,
};

/// Read size for stream bodies; also the pacing granularity.
const STREAM_CHUNK_BYTES: usize = 64 * 1024;

pub async fn master_playlist(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    client: ClientAddress,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let permit = admit(&state, client, &headers)?;
    let record = media::get_record(&state, &media_id).await?;
    ensure_ready(&record)?;
    authorize_stream(&state, &record, &headers).await?;
    let path = state
        .storage
        .stream_directory(&record.account_id, &record.id)
        .join(&state.config.transcoding.master_playlist_filename);
    serve_path(
        &state,
        &record,
        &headers,
        &path,
        "application/vnd.apple.mpegurl",
        &state.config.streaming.playlist_cache_control,
        Delivery {
            class: AssetClass::Playlist,
            allow_ranges: false,
            permit,
        },
    )
    .await
}

pub async fn source(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    client: ClientAddress,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let permit = admit(&state, client, &headers)?;
    let record = media::get_record(&state, &media_id).await?;
    ensure_ready(&record)?;
    authorize_stream(&state, &record, &headers).await?;
    if !state.config.protection.expose_original_with_hls {
        let variants: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM media_variants WHERE media_id = ?")
                .bind(&record.id)
                .fetch_one(&state.pool)
                .await?;
        if !original_exposed(&state, variants > 0) && !is_owner(&state, &record, &headers).await {
            return Err(AppError::NotFound(
                "the original file is not available for playback".to_owned(),
            ));
        }
    }
    let object_key = record
        .original_object_key
        .as_deref()
        .ok_or(AppError::MediaNotReady)?;
    let path = state
        .storage
        .object_path(object_key)
        .map_err(AppError::internal)?;
    let mime = record
        .source_mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let cache_control = cache_control(
        &state,
        &record,
        &state.config.streaming.source_cache_control,
    );
    serve_path(
        &state,
        &record,
        &headers,
        &path,
        mime,
        cache_control,
        Delivery {
            class: AssetClass::Source,
            allow_ranges: state.config.streaming.enable_range_requests,
            permit,
        },
    )
    .await
}

pub async fn variant_asset(
    State(state): State<AppState>,
    Path((media_id, variant, asset)): Path<(String, String, String)>,
    client: ClientAddress,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    validate_component(&variant)?;
    validate_component(&asset)?;
    let permit = admit(&state, client, &headers)?;
    let record = media::get_record(&state, &media_id).await?;
    ensure_ready(&record)?;
    authorize_stream(&state, &record, &headers).await?;
    let playlist_object_key: String = sqlx::query_scalar(
        "SELECT playlist_object_key FROM media_variants WHERE media_id = ? AND name = ?",
    )
    .bind(&media_id)
    .bind(&variant)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("media variant not found".to_owned()))?;
    let playlist_path = state
        .storage
        .object_path(&playlist_object_key)
        .map_err(AppError::internal)?;
    let directory = playlist_path.parent().ok_or_else(|| {
        AppError::internal(anyhow::anyhow!("variant playlist has no parent directory"))
    })?;
    let path = directory.join(&asset);
    let mime = content_type_for_asset(&asset);
    let is_playlist = asset.ends_with(".m3u8");
    let configured_cache = if is_playlist {
        &state.config.streaming.playlist_cache_control
    } else {
        &state.config.streaming.segment_cache_control
    };
    let cache_control = cache_control(&state, &record, configured_cache);
    serve_path(
        &state,
        &record,
        &headers,
        &path,
        mime,
        cache_control,
        Delivery {
            class: if is_playlist {
                AssetClass::Playlist
            } else {
                AssetClass::Segment
            },
            allow_ranges: state.config.streaming.enable_range_requests && !is_playlist,
            permit,
        },
    )
    .await
}

/// Applies hotlink protection and per-client limits before any work.
fn admit(
    state: &AppState,
    client: ClientAddress,
    headers: &HeaderMap,
) -> Result<Option<StreamPermit>, AppError> {
    if state.config.protection.block_cross_site_requests
        && headers
            .get("sec-fetch-site")
            .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"cross-site"))
    {
        return Err(AppError::Forbidden);
    }
    state.protection.admit_stream(client.key())
}

async fn is_owner(state: &AppState, record: &MediaRecord, headers: &HeaderMap) -> bool {
    headers.contains_key(header::AUTHORIZATION)
        && state
            .auth
            .authenticate(headers)
            .await
            .is_ok_and(|authenticated| authenticated.account_id == record.account_id)
}

struct Delivery {
    class: AssetClass,
    allow_ranges: bool,
    permit: Option<StreamPermit>,
}

async fn authorize_stream(
    state: &AppState,
    record: &MediaRecord,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    if record.visibility != "private" {
        return Ok(());
    }
    if headers.contains_key(header::AUTHORIZATION) {
        let authenticated = state.auth.authenticate(headers).await?;
        if authenticated.account_id == record.account_id {
            return Ok(());
        }
        return Err(AppError::Forbidden);
    }
    let token = cookie::read(headers, &state.config.analytics.playback_cookie_name)
        .ok_or(AppError::Unauthorized)?;
    state
        .auth
        .verify_playback_token(&token, &record.id, &record.account_id)
}

async fn serve_path(
    state: &AppState,
    record: &MediaRecord,
    headers: &HeaderMap,
    path: &FsPath,
    mime: &str,
    cache_control: &str,
    delivery: Delivery,
) -> Result<Response, AppError> {
    let allow_ranges = delivery.allow_ranges;
    let metadata = tokio::fs::metadata(path).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound("media asset not found".to_owned())
        } else {
            AppError::Io(error)
        }
    })?;
    if !metadata.is_file() {
        return Err(AppError::NotFound("media asset not found".to_owned()));
    }
    let total = metadata.len();
    let selected_range = if allow_ranges {
        parse_range(headers.get(header::RANGE), total)?
    } else {
        None
    };
    let (start, end, status) = selected_range.map_or_else(
        || (0, total.saturating_sub(1), StatusCode::OK),
        |(start, end)| (start, end, StatusCode::PARTIAL_CONTENT),
    );
    let length = if total == 0 {
        0
    } else {
        end.saturating_sub(start).saturating_add(1)
    };
    let mut file = tokio::fs::File::open(path).await?;
    file.seek(SeekFrom::Start(start)).await?;
    let session_id = cookie::read(
        headers,
        &state.config.analytics.playback_session_cookie_name,
    );
    let stream = AnalyticsStream::new(
        ReaderStream::with_capacity(file.take(length), STREAM_CHUNK_BYTES),
        state.clone(),
        record.account_id.clone(),
        record.id.clone(),
        session_id,
    );
    let body = Body::from_stream(PacedStream::new(
        stream,
        state.protection.clone(),
        state.protection.pacing(delivery.class),
        delivery.permit,
    ));
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, length.to_string())
        .header(header::CACHE_CONTROL, cache_control)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if allow_ranges {
        builder = builder.header(header::ACCEPT_RANGES, "bytes");
    }
    if status == StatusCode::PARTIAL_CONTENT {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total}"),
        );
    }
    builder
        .body(body)
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))
}

fn validate_component(value: &str) -> Result<(), AppError> {
    let path = FsPath::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().count() != 1
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(AppError::BadRequest("invalid media asset path".to_owned()));
    }
    Ok(())
}

fn content_type_for_asset(asset: &str) -> &'static str {
    if asset.ends_with(".m3u8") {
        "application/vnd.apple.mpegurl"
    } else if asset.ends_with(".m4s") {
        "video/iso.segment"
    } else if asset.ends_with(".ts") {
        "video/mp2t"
    } else if asset.ends_with(".mp4") {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}

fn cache_control<'a>(state: &'a AppState, record: &MediaRecord, configured: &'a str) -> &'a str {
    if record.visibility == "private" {
        &state.config.streaming.private_asset_cache_control
    } else {
        configured
    }
}
