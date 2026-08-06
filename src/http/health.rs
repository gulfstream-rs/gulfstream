use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
}

pub async fn live() -> Json<Health> {
    Json(Health { status: "ok" })
}

pub async fn ready(State(state): State<AppState>) -> Result<(StatusCode, Json<Health>), AppError> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;
    tokio::fs::metadata(&state.config.storage.root).await?;
    tokio::fs::metadata(&state.config.storage.temporary_root).await?;
    if state.config.transcoding.enabled {
        tokio::fs::metadata(&state.config.transcoding.ffmpeg_path).await?;
        tokio::fs::metadata(&state.config.transcoding.ffprobe_path).await?;
    }
    Ok((StatusCode::OK, Json(Health { status: "ready" })))
}
