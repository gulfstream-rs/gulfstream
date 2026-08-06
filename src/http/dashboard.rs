use axum::{Json, extract::State, http::HeaderMap};

use crate::{
    application::dashboard::{self, Dashboard},
    error::AppError,
    state::AppState,
};

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Dashboard>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    Ok(Json(dashboard::load(&state, &authenticated).await?))
}
