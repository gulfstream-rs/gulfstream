use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    application::analytics,
    domain::analytics::{AnalyticsPoint, AnalyticsTotals, CountryStats},
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

#[derive(Debug, Serialize)]
pub struct AnalyticsCountries {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub media_id: Option<String>,
    /// Whether a country source (database or CDN header) is configured.
    pub geolocation_enabled: bool,
    pub countries: Vec<CountryStats>,
}

pub async fn countries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<AnalyticsCountries>, AppError> {
    let account_id = authorize(&state, &headers, query.media_id.as_deref()).await?;
    let range = analytics::reporting_range(&state, query.from, query.to, query.media_id)?;
    let countries = analytics::countries(&state, &account_id, &range).await?;
    Ok(Json(AnalyticsCountries {
        from: range.from,
        to: range.to,
        media_id: range.media_id,
        geolocation_enabled: state.geolocation.is_enabled(),
        countries,
    }))
}

/// Authenticates the caller and checks ownership of the filtered media.
async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    media_id: Option<&str>,
) -> Result<String, AppError> {
    let authenticated = state.auth.authenticate(headers).await?;
    if let Some(media_id) = media_id {
        crate::application::media::assert_account_owns_media(
            &state.pool,
            &authenticated.account_id,
            media_id,
        )
        .await?;
    }
    Ok(authenticated.account_id)
}

pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<AnalyticsSummary>, AppError> {
    let account_id = authorize(&state, &headers, query.media_id.as_deref()).await?;
    let range = analytics::reporting_range(&state, query.from, query.to, query.media_id)?;
    let totals = analytics::totals(&state, &account_id, &range).await?;
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
    let account_id = authorize(&state, &headers, query.media_id.as_deref()).await?;
    let range = analytics::reporting_range(&state, query.from, query.to, query.media_id)?;
    let points = analytics::time_series(&state, &account_id, &range).await?;
    Ok(Json(AnalyticsTimeSeries {
        from: range.from,
        to: range.to,
        media_id: range.media_id,
        points,
    }))
}
