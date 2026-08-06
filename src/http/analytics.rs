use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    application::analytics,
    domain::analytics::{AnalyticsPoint, AnalyticsTotals},
    error::AppError,
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticsQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub media_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummary {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub media_id: Option<String>,
    pub totals: AnalyticsTotals,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsTimeSeries {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub media_id: Option<String>,
    pub points: Vec<AnalyticsPoint>,
}

pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<AnalyticsSummary>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    if let Some(media_id) = query.media_id.as_deref() {
        crate::application::media::assert_account_owns_media(
            &state.pool,
            &authenticated.account_id,
            media_id,
        )
        .await?;
    }
    let range = analytics::reporting_range(&state, query.from, query.to, query.media_id)?;
    let totals = analytics::totals(&state, &authenticated.account_id, &range).await?;
    Ok(Json(AnalyticsSummary {
        from: range.from,
        to: range.to,
        media_id: range.media_id,
        totals,
    }))
}

pub async fn time_series(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<AnalyticsTimeSeries>, AppError> {
    let authenticated = state.auth.authenticate(&headers).await?;
    if let Some(media_id) = query.media_id.as_deref() {
        crate::application::media::assert_account_owns_media(
            &state.pool,
            &authenticated.account_id,
            media_id,
        )
        .await?;
    }
    let range = analytics::reporting_range(&state, query.from, query.to, query.media_id)?;
    let points = analytics::time_series(&state, &authenticated.account_id, &range).await?;
    Ok(Json(AnalyticsTimeSeries {
        from: range.from,
        to: range.to,
        media_id: range.media_id,
        points,
    }))
}
