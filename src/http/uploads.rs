use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
};

use crate::{
    application::uploads::{self, CreatedMedia, RemoteImportRequest},
    error::AppError,
    state::AppState,
};

pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<(StatusCode, Json<CreatedMedia>), AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    let media = uploads::create_from_multipart(&state, &authenticated, multipart).await?;
    Ok((StatusCode::ACCEPTED, Json(media)))
}

pub async fn remote_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RemoteImportRequest>,
) -> Result<(StatusCode, Json<CreatedMedia>), AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    let media = uploads::create_remote_import(&state, &authenticated, request).await?;
    Ok((StatusCode::ACCEPTED, Json(media)))
}
