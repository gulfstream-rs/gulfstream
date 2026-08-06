use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

use crate::{
    application::jobs::{self, JobFilter, JobPage},
    domain::media::{JobKind, JobStatus},
    error::AppError,
    http::pagination::Pagination,
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListJobsQuery {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<JobPage>, AppError> {
    validate_status(query.status.as_deref())?;
    validate_kind(query.kind.as_deref())?;
    let authenticated = state.auth.authenticate(&headers).await?;
    let pagination = Pagination::from_query(query.page, query.page_size, &state.config.api);
    Ok(Json(
        jobs::list(
            &state,
            &authenticated,
            JobFilter {
                status: query.status,
                kind: query.kind,
                page: pagination.page,
                page_size: pagination.page_size,
                offset: pagination.offset(),
            },
        )
        .await?,
    ))
}

fn validate_status(value: Option<&str>) -> Result<(), AppError> {
    if value.is_some_and(|status| status.parse::<JobStatus>().is_err()) {
        return Err(AppError::BadRequest("invalid job status filter".to_owned()));
    }
    Ok(())
}

fn validate_kind(value: Option<&str>) -> Result<(), AppError> {
    if value.is_some_and(|kind| kind.parse::<JobKind>().is_err()) {
        return Err(AppError::BadRequest("invalid job kind filter".to_owned()));
    }
    Ok(())
}
