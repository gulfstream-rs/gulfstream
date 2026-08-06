use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::Row;

use crate::{
    application::{accounts, analytics},
    domain::{
        account::{Account, AuthenticatedAccount},
        analytics::AnalyticsTotals,
    },
    error::AppError,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct Dashboard {
    pub account: Account,
    pub media_total: i64,
    pub media_by_status: Vec<StatusCount>,
    pub jobs_total: i64,
    pub jobs_by_status: Vec<StatusCount>,
    pub analytics_from: String,
    pub analytics_to: String,
    pub analytics: AnalyticsTotals,
}

pub async fn load(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
) -> Result<Dashboard, AppError> {
    let account = accounts::get_account(state, authenticated).await?;
    let media_rows = sqlx::query(
        "SELECT status, COUNT(*) AS count FROM media_files \
         WHERE account_id = ? AND deleted_at IS NULL GROUP BY status ORDER BY status",
    )
    .bind(&authenticated.account_id)
    .fetch_all(&state.pool)
    .await?;
    let media_by_status = media_rows
        .into_iter()
        .map(|row| StatusCount {
            status: row.get("status"),
            count: row.get("count"),
        })
        .collect::<Vec<_>>();
    let media_total = media_by_status.iter().map(|item| item.count).sum();

    let job_rows = sqlx::query(
        "SELECT media_jobs.status, COUNT(*) AS count FROM media_jobs \
         JOIN media_files ON media_files.id = media_jobs.media_id \
         WHERE media_files.account_id = ? GROUP BY media_jobs.status ORDER BY media_jobs.status",
    )
    .bind(&authenticated.account_id)
    .fetch_all(&state.pool)
    .await?;
    let jobs_by_status = job_rows
        .into_iter()
        .map(|row| StatusCount {
            status: row.get("status"),
            count: row.get("count"),
        })
        .collect::<Vec<_>>();
    let jobs_total = jobs_by_status.iter().map(|item| item.count).sum();

    let to = Utc::now().date_naive();
    let from = to - Duration::days(state.config.web.dashboard_reporting_days - 1);
    let range = analytics::reporting_range(state, Some(from), Some(to), None)?;
    let totals = analytics::totals(state, &authenticated.account_id, &range).await?;
    Ok(Dashboard {
        account,
        media_total,
        media_by_status,
        jobs_total,
        jobs_by_status,
        analytics_from: range.from.to_string(),
        analytics_to: range.to.to_string(),
        analytics: totals,
    })
}
