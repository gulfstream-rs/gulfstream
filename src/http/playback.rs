use std::{
    io::SeekFrom,
    path::{Component, Path as FsPath},
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use futures_util::Stream;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{
    application::{
        analytics,
        media::{self, MediaRecord},
    },
    cookie::{self, SetCookie},
    error::AppError,
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchQuery {
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaybackEventRequest {
    pub kind: analytics::PlaybackEventKind,
    pub position_ms: i64,
    pub watched_delta_ms: i64,
}

pub async fn watch(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    Query(query): Query<WatchQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let record = media::get_record(&state, &media_id).await?;
    ensure_ready(&record)?;
    if record.visibility == "private"
        && let Some(token) = query.token.as_deref()
    {
        state
            .auth
            .verify_playback_token(token, &record.id, &record.account_id)?;
        let mut response = StatusCode::SEE_OTHER.into_response();
        response.headers_mut().insert(
            header::LOCATION,
            format!("{}/{}", state.config.routes.watch, record.id)
                .parse::<HeaderValue>()
                .map_err(AppError::internal)?,
        );
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            state
                .config
                .streaming
                .token_redirect_cache_control
                .parse::<HeaderValue>()
                .map_err(AppError::internal)?,
        );
        append_cookie(
            response.headers_mut(),
            &state,
            &state.config.analytics.playback_cookie_name,
            token,
            &playback_redemption_cookie_path(&state, &record.id),
            Some(state.config.security.playback_token_ttl_seconds),
        )?;
        return Ok(response);
    }
    let playback_token = authorize_watch(&state, &record, &headers).await?;
    let existing_visitor = cookie::read(&headers, &state.config.analytics.visitor_cookie_name);
    let visitor_identifier = existing_visitor
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let session_id =
        analytics::start_session(&state, &record.account_id, &record.id, &visitor_identifier)
            .await?;
    let variant_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM media_variants WHERE media_id = ?")
            .bind(&record.id)
            .fetch_one(&state.pool)
            .await?;
    let (source_mode, source_url) = if variant_count > 0 {
        (
            "hls",
            format!("{}/{}/master.m3u8", state.config.routes.stream, record.id),
        )
    } else if record.original_object_key.is_some() {
        (
            "file",
            format!("{}/{}/source", state.config.routes.stream, record.id),
        )
    } else {
        return Err(AppError::MediaNotReady);
    };
    let hls_script = if source_mode == "hls" {
        state
            .config
            .player
            .hls_javascript_url
            .as_deref()
            .map(|url| format!("<script src=\"{}\"></script>", html_escape(url)))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let event_url = format!("{}/playback/{session_id}/events", state.config.routes.api);
    let heartbeat_milliseconds = state
        .config
        .analytics
        .heartbeat_interval_seconds
        .saturating_mul(1000);
    let html = state
        .player_template
        .replace("{{TITLE}}", &html_escape(&record.title))
        .replace("{{DESCRIPTION}}", &html_escape(&record.description))
        .replace("{{HLS_SCRIPT}}", &hls_script)
        .replace("{{STREAM_URL_JSON}}", &json_string(&source_url)?)
        .replace("{{EVENT_URL_JSON}}", &json_string(&event_url)?)
        .replace("{{SOURCE_MODE_JSON}}", &json_string(source_mode)?)
        .replace(
            "{{HEARTBEAT_MILLISECONDS}}",
            &heartbeat_milliseconds.to_string(),
        );
    let mut response = Html(html).into_response();
    if existing_visitor.is_none() {
        append_cookie(
            response.headers_mut(),
            &state,
            &state.config.analytics.visitor_cookie_name,
            &visitor_identifier,
            &state.config.analytics.visitor_cookie_path,
            Some(state.config.analytics.visitor_cookie_max_age_seconds),
        )?;
    }
    append_cookie(
        response.headers_mut(),
        &state,
        &state.config.analytics.playback_session_cookie_name,
        &session_id,
        &state.config.analytics.playback_session_cookie_path,
        Some(state.config.security.playback_token_ttl_seconds),
    )?;
    if let Some(token) = playback_token {
        append_cookie(
            response.headers_mut(),
            &state,
            &state.config.analytics.playback_cookie_name,
            &token,
            &playback_asset_cookie_path(&state, &record.id),
            Some(state.config.security.playback_token_ttl_seconds),
        )?;
        append_cookie(
            response.headers_mut(),
            &state,
            &state.config.analytics.playback_cookie_name,
            "",
            &playback_redemption_cookie_path(&state, &record.id),
            Some(0),
        )?;
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        state
            .config
            .streaming
            .player_cache_control
            .parse::<HeaderValue>()
            .map_err(AppError::internal)?,
    );
    Ok(response)
}

pub async fn playback_event(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<PlaybackEventRequest>,
) -> Result<StatusCode, AppError> {
    let cookie_session = cookie::read(
        &headers,
        &state.config.analytics.playback_session_cookie_name,
    )
    .ok_or(AppError::Unauthorized)?;
    if cookie_session != session_id {
        return Err(AppError::Unauthorized);
    }
    analytics::record_playback_event(
        &state,
        &session_id,
        request.kind,
        request.position_ms,
        request.watched_delta_ms,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn master_playlist(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
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
        false,
    )
    .await
}

pub async fn source(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let record = media::get_record(&state, &media_id).await?;
    ensure_ready(&record)?;
    authorize_stream(&state, &record, &headers).await?;
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
        state.config.streaming.enable_range_requests,
    )
    .await
}

pub async fn variant_asset(
    State(state): State<AppState>,
    Path((media_id, variant, asset)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    validate_component(&variant)?;
    validate_component(&asset)?;
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
    let configured_cache = if asset.ends_with(".m3u8") {
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
        state.config.streaming.enable_range_requests && !asset.ends_with(".m3u8"),
    )
    .await
}

async fn authorize_watch(
    state: &AppState,
    record: &MediaRecord,
    headers: &HeaderMap,
) -> Result<Option<String>, AppError> {
    if record.visibility != "private" {
        return Ok(None);
    }
    if headers.contains_key(header::AUTHORIZATION) {
        let authenticated = state.auth.authenticate(headers).await?;
        if authenticated.account_id != record.account_id {
            return Err(AppError::Forbidden);
        }
        return Ok(Some(
            state
                .auth
                .sign_playback_token(&record.id, &record.account_id)?,
        ));
    }
    if let Some(token) = cookie::read(headers, &state.config.analytics.playback_cookie_name) {
        state
            .auth
            .verify_playback_token(&token, &record.id, &record.account_id)?;
        return Ok(Some(token));
    }
    Err(AppError::Unauthorized)
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
    allow_ranges: bool,
) -> Result<Response, AppError> {
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
        ReaderStream::new(file.take(length)),
        state.clone(),
        record.account_id.clone(),
        record.id.clone(),
        session_id,
    );
    let body = Body::from_stream(stream);
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, length.to_string())
        .header(header::CACHE_CONTROL, cache_control);
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

struct AnalyticsStream {
    inner: ReaderStream<tokio::io::Take<tokio::fs::File>>,
    state: AppState,
    account_id: String,
    media_id: String,
    session_id: Option<String>,
    bytes_emitted: u64,
    recording_scheduled: bool,
}

impl AnalyticsStream {
    fn new(
        inner: ReaderStream<tokio::io::Take<tokio::fs::File>>,
        state: AppState,
        account_id: String,
        media_id: String,
        session_id: Option<String>,
    ) -> Self {
        Self {
            inner,
            state,
            account_id,
            media_id,
            session_id,
            bytes_emitted: 0,
            recording_scheduled: false,
        }
    }

    fn schedule_recording(&mut self) {
        if self.recording_scheduled || self.bytes_emitted == 0 {
            return;
        }
        self.recording_scheduled = true;
        let state = self.state.clone();
        let account_id = self.account_id.clone();
        let media_id = self.media_id.clone();
        let session_id = self.session_id.clone();
        let bytes = self.bytes_emitted;
        tokio::spawn(async move {
            if let Err(error) = analytics::record_bytes_served(
                &state,
                &account_id,
                &media_id,
                session_id.as_deref(),
                bytes,
            )
            .await
            {
                tracing::error!(%error, %media_id, "failed to persist streamed byte analytics");
            }
        });
    }
}

impl Stream for AnalyticsStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(context) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.bytes_emitted = self.bytes_emitted.saturating_add(bytes.len() as u64);
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(None) => {
                self.schedule_recording();
                Poll::Ready(None)
            }
            other => other,
        }
    }
}

impl Drop for AnalyticsStream {
    fn drop(&mut self) {
        self.schedule_recording();
    }
}

fn parse_range(value: Option<&HeaderValue>, total: u64) -> Result<Option<(u64, u64)>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if total == 0 {
        return Err(AppError::RangeNotSatisfiable);
    }
    let text = value.to_str().map_err(|_| AppError::RangeNotSatisfiable)?;
    let range = text
        .strip_prefix("bytes=")
        .ok_or(AppError::RangeNotSatisfiable)?;
    if range.contains(',') {
        return Err(AppError::RangeNotSatisfiable);
    }
    let (start_text, end_text) = range.split_once('-').ok_or(AppError::RangeNotSatisfiable)?;
    let (start, end) = if start_text.is_empty() {
        let suffix = end_text
            .parse::<u64>()
            .map_err(|_| AppError::RangeNotSatisfiable)?;
        if suffix == 0 {
            return Err(AppError::RangeNotSatisfiable);
        }
        (total.saturating_sub(suffix.min(total)), total - 1)
    } else {
        let start = start_text
            .parse::<u64>()
            .map_err(|_| AppError::RangeNotSatisfiable)?;
        let end = if end_text.is_empty() {
            total - 1
        } else {
            end_text
                .parse::<u64>()
                .map_err(|_| AppError::RangeNotSatisfiable)?
                .min(total - 1)
        };
        (start, end)
    };
    if start >= total || start > end {
        return Err(AppError::RangeNotSatisfiable);
    }
    Ok(Some((start, end)))
}

fn ensure_ready(record: &MediaRecord) -> Result<(), AppError> {
    if record.status == "ready" {
        Ok(())
    } else {
        Err(AppError::MediaNotReady)
    }
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

fn playback_redemption_cookie_path(state: &AppState, media_id: &str) -> String {
    format!("{}/{}", state.config.routes.watch, media_id)
}

fn playback_asset_cookie_path(state: &AppState, media_id: &str) -> String {
    format!("{}/{media_id}/", state.config.routes.stream)
}

fn append_cookie(
    headers: &mut HeaderMap,
    state: &AppState,
    name: &str,
    value: &str,
    path: &str,
    max_age: Option<i64>,
) -> Result<(), AppError> {
    cookie::append(
        headers,
        SetCookie {
            name,
            value,
            path,
            domain: None,
            max_age_seconds: max_age,
            http_only: true,
            secure: state.config.analytics.cookie_secure,
            same_site: state.config.analytics.cookie_same_site.as_cookie_value(),
        },
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn json_string(value: &str) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(AppError::internal)
}

#[cfg(test)]
mod tests {
    use super::parse_range;
    use axum::http::HeaderValue;

    #[test]
    fn range_parser_supports_standard_and_suffix_ranges() {
        let standard = HeaderValue::from_static("bytes=10-19");
        assert_eq!(
            parse_range(Some(&standard), 100).expect("valid range"),
            Some((10, 19))
        );
        let open = HeaderValue::from_static("bytes=90-");
        assert_eq!(
            parse_range(Some(&open), 100).expect("valid range"),
            Some((90, 99))
        );
        let suffix = HeaderValue::from_static("bytes=-10");
        assert_eq!(
            parse_range(Some(&suffix), 100).expect("valid range"),
            Some((90, 99))
        );
    }

    #[test]
    fn range_parser_rejects_multiple_or_out_of_bounds_ranges() {
        let multiple = HeaderValue::from_static("bytes=0-1,4-5");
        assert!(parse_range(Some(&multiple), 100).is_err());
        let outside = HeaderValue::from_static("bytes=100-101");
        assert!(parse_range(Some(&outside), 100).is_err());
    }
}
