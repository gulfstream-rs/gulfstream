use serde::Serialize;
use sqlx::Row;

use crate::{
    application::page::Page, domain::account::AuthenticatedAccount, error::AppError,
    state::AppState,
};

#[derive(Clone, Debug)]
pub struct JobFilter {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub page: u32,
    pub page_size: u32,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct JobListItem {
    pub id: String,
    pub media_id: String,
    pub media_title: String,
    pub kind: String,
    pub status: String,
    pub attempts: i64,
    pub maximum_attempts: i64,
    pub run_after: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub type JobPage = Page<JobListItem>;

pub async fn list(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    filter: JobFilter,
) -> Result<JobPage, AppError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM media_jobs \
         JOIN media_files ON media_files.id = media_jobs.media_id \
         WHERE media_files.account_id = ? \
           AND (? IS NULL OR media_jobs.status = ?) \
           AND (? IS NULL OR media_jobs.kind = ?)",
    )
    .bind(&authenticated.account_id)
    .bind(filter.status.as_deref())
    .bind(filter.status.as_deref())
    .bind(filter.kind.as_deref())
    .bind(filter.kind.as_deref())
    .fetch_one(&state.pool)
    .await?;
    let rows = sqlx::query(
        "SELECT media_jobs.id, media_jobs.media_id, media_files.title AS media_title, \
                media_jobs.kind, media_jobs.status, media_jobs.attempts, media_jobs.maximum_attempts, \
                media_jobs.run_after, media_jobs.error_message, media_jobs.created_at, media_jobs.updated_at \
         FROM media_jobs JOIN media_files ON media_files.id = media_jobs.media_id \
         WHERE media_files.account_id = ? \
           AND (? IS NULL OR media_jobs.status = ?) \
           AND (? IS NULL OR media_jobs.kind = ?) \
         ORDER BY media_jobs.created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&authenticated.account_id)
    .bind(filter.status.as_deref())
    .bind(filter.status.as_deref())
    .bind(filter.kind.as_deref())
    .bind(filter.kind.as_deref())
    .bind(i64::from(filter.page_size))
    .bind(filter.offset)
    .fetch_all(&state.pool)
    .await?;
    let items = rows
        .into_iter()
        .map(|row| JobListItem {
            id: row.get("id"),
            media_id: row.get("media_id"),
            media_title: row.get("media_title"),
            kind: row.get("kind"),
            status: row.get("status"),
            attempts: row.get("attempts"),
            maximum_attempts: row.get("maximum_attempts"),
            run_after: row.get("run_after"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();
    Ok(Page::new(items, filter.page, filter.page_size, total))
}
