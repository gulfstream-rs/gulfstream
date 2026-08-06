use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::{
    application::media::{self, ListMediaFilter, MediaPage, UpdateMedia},
    domain::media::{Media, MediaJob, MediaStatus, Visibility},
    error::AppError,
    http::pagination::Pagination,
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListMediaQuery {
    pub status: Option<String>,
    pub visibility: Option<String>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PlaybackTokenResponse {
    pub token: String,
    pub expires_in_seconds: i64,
    pub watch_url: String,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListMediaQuery>,
) -> Result<Json<MediaPage>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    validate_status(query.status.as_deref())?;
    validate_visibility(query.visibility.as_deref())?;
    let pagination = Pagination::from_query(query.page, query.page_size, &state.config.api);
    Ok(Json(
        media::list(
            &state,
            &authenticated,
            ListMediaFilter {
                status: query.status,
                visibility: query.visibility,
                search: query.search,
                page: pagination.page,
                page_size: pagination.page_size,
                offset: pagination.offset(),
            },
        )
        .await?,
    ))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<String>,
) -> Result<Json<Media>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    Ok(Json(
        media::get_owned(&state, &authenticated, &media_id).await?,
    ))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<String>,
    Json(request): Json<UpdateMedia>,
) -> Result<Json<Media>, AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    Ok(Json(
        media::update(&state, &authenticated, &media_id, request).await?,
    ))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    media::delete(&state, &authenticated, &media_id).await?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn retry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    media::retry(&state, &authenticated, &media_id).await?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<String>,
) -> Result<Json<Vec<MediaJob>>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    Ok(Json(media::jobs(&state, &authenticated, &media_id).await?))
}

pub async fn playback_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<String>,
) -> Result<Json<PlaybackTokenResponse>, AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    media::assert_account_owns_media(&state.pool, &authenticated.account_id, &media_id).await?;
    let token = state
        .auth
        .sign_playback_token(&media_id, &authenticated.account_id)?;
    Ok(Json(PlaybackTokenResponse {
        watch_url: format!(
            "{}{}/{media_id}?token={token}",
            state.config.server.public_base_url, state.config.routes.watch
        ),
        expires_in_seconds: state.config.security.playback_token_ttl_seconds,
        token,
    }))
}

fn validate_status(value: Option<&str>) -> Result<(), AppError> {
    if value.is_some_and(|status| status.parse::<MediaStatus>().is_err()) {
        return Err(AppError::BadRequest(
            "invalid media status filter".to_owned(),
        ));
    }
    Ok(())
}

fn validate_visibility(value: Option<&str>) -> Result<(), AppError> {
    if value.is_some_and(|visibility| visibility.parse::<Visibility>().is_err()) {
        return Err(AppError::BadRequest("invalid visibility filter".to_owned()));
    }
    Ok(())
}
