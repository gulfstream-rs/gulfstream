use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};

use crate::{
    application::accounts::{self, CreateApiKey, RegisterAccount, UpdateAccount},
    domain::account::{Account, ApiKeySummary, IssuedApiKey},
    error::AppError,
    state::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterAccount>,
) -> Result<(StatusCode, Json<Account>), AppError> {
    let account = accounts::register(&state, &headers, request).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Account>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    Ok(Json(accounts::get_account(&state, &authenticated).await?))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateAccount>,
) -> Result<Json<Account>, AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    Ok(Json(
        accounts::update(&state, &authenticated, request).await?,
    ))
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKeySummary>>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    Ok(Json(accounts::list_api_keys(&state, &authenticated).await?))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateApiKey>,
) -> Result<(StatusCode, Json<IssuedApiKey>), AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    let key = accounts::create_api_key(&state, &authenticated, request).await?;
    Ok((StatusCode::CREATED, Json(key)))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(api_key_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let authenticated = state.auth.authenticate_for_write(&headers).await?;
    accounts::revoke_api_key(&state, &authenticated, &api_key_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
