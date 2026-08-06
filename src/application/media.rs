use chrono::Utc;
use serde::Deserialize;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    application::page::Page,
    domain::{
        account::AuthenticatedAccount,
        media::{JobKind, JobStatus, Media, MediaJob, MediaStatus, MediaVariant, Visibility},
    },
    error::AppError,
    state::AppState,
};

#[derive(Clone, Debug)]
pub struct MediaRecord {
    pub id: String,
    pub account_id: String,
    pub original_object_key: Option<String>,
    pub source_filename: String,
    pub source_mime_type: Option<String>,
    pub source_size_bytes: i64,
    pub storage_bytes: i64,
    pub title: String,
    pub description: String,
    pub visibility: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMedia {
    pub title: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
}

#[derive(Clone, Debug)]
pub struct ListMediaFilter {
    pub status: Option<String>,
    pub visibility: Option<String>,
    pub search: Option<String>,
    pub page: u32,
    pub page_size: u32,
    pub offset: i64,
}

pub type MediaPage = Page<Media>;

pub async fn get_owned(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    media_id: &str,
) -> Result<Media, AppError> {
    let record = owned_record(&state.pool, &authenticated.account_id, media_id).await?;
    hydrate_media(state, record).await
}

pub async fn get_record(state: &AppState, media_id: &str) -> Result<MediaRecord, AppError> {
    let row = sqlx::query(
        "SELECT id, account_id, original_object_key, source_filename, source_mime_type, \
                source_size_bytes, storage_bytes, title, description, visibility, status \
         FROM media_files WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(media_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("media not found".to_owned()))?;
    Ok(record_from_row(&row))
}

pub async fn list(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    filter: ListMediaFilter,
) -> Result<MediaPage, AppError> {
    let search = filter
        .search
        .as_ref()
        .map(|value| format!("%{}%", escape_like_pattern(value.trim())));
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM media_files \
         WHERE account_id = ? AND deleted_at IS NULL \
           AND (? IS NULL OR status = ?) \
           AND (? IS NULL OR visibility = ?) \
           AND (? IS NULL OR title LIKE ? ESCAPE '\\')",
    )
    .bind(&authenticated.account_id)
    .bind(filter.status.as_deref())
    .bind(filter.status.as_deref())
    .bind(filter.visibility.as_deref())
    .bind(filter.visibility.as_deref())
    .bind(search.as_deref())
    .bind(search.as_deref())
    .fetch_one(&state.pool)
    .await?;
    let rows = sqlx::query(
        "SELECT id, account_id, original_object_key, source_filename, source_mime_type, \
                source_size_bytes, storage_bytes, title, description, visibility, status \
         FROM media_files \
         WHERE account_id = ? AND deleted_at IS NULL \
           AND (? IS NULL OR status = ?) \
           AND (? IS NULL OR visibility = ?) \
           AND (? IS NULL OR title LIKE ? ESCAPE '\\') \
         ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&authenticated.account_id)
    .bind(filter.status.as_deref())
    .bind(filter.status.as_deref())
    .bind(filter.visibility.as_deref())
    .bind(filter.visibility.as_deref())
    .bind(search.as_deref())
    .bind(search.as_deref())
    .bind(i64::from(filter.page_size))
    .bind(filter.offset)
    .fetch_all(&state.pool)
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(hydrate_media(state, record_from_row(&row)).await?);
    }
    Ok(Page::new(items, filter.page, filter.page_size, total))
}

pub async fn update(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    media_id: &str,
    request: UpdateMedia,
) -> Result<Media, AppError> {
    if request.title.is_none() && request.description.is_none() && request.visibility.is_none() {
        return Err(AppError::BadRequest(
            "at least one field must be provided".to_owned(),
        ));
    }
    if request
        .title
        .as_ref()
        .is_some_and(|value| value.len() > state.config.uploads.maximum_text_field_bytes)
        || request
            .description
            .as_ref()
            .is_some_and(|value| value.len() > state.config.uploads.maximum_text_field_bytes)
    {
        return Err(AppError::PayloadTooLarge(format!(
            "media metadata exceeds the configured limit of {} bytes",
            state.config.uploads.maximum_text_field_bytes
        )));
    }
    let title = request
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if request.title.is_some() && title.is_none() {
        return Err(AppError::BadRequest("title cannot be empty".to_owned()));
    }
    let result = sqlx::query(
        "UPDATE media_files SET \
            title = COALESCE(?, title), \
            description = COALESCE(?, description), \
            visibility = COALESCE(?, visibility), \
            updated_at = ? \
         WHERE id = ? AND account_id = ? AND deleted_at IS NULL",
    )
    .bind(title)
    .bind(request.description.as_deref())
    .bind(request.visibility.map(Visibility::as_str))
    .bind(now())
    .bind(media_id)
    .bind(&authenticated.account_id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("media not found".to_owned()));
    }
    get_owned(state, authenticated, media_id).await
}

pub async fn delete(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    media_id: &str,
) -> Result<(), AppError> {
    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE media_files SET status = ?, deleted_at = ?, updated_at = ? \
         WHERE id = ? AND account_id = ? AND deleted_at IS NULL",
    )
    .bind(MediaStatus::Deleting.as_str())
    .bind(now())
    .bind(now())
    .bind(media_id)
    .bind(&authenticated.account_id)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("media not found".to_owned()));
    }
    sqlx::query(
        "UPDATE media_jobs SET status = ?, locked_at = NULL, locked_by = NULL, updated_at = ? \
         WHERE media_id = ? AND kind != ? AND status = ?",
    )
    .bind(JobStatus::Cancelled.as_str())
    .bind(now())
    .bind(media_id)
    .bind(JobKind::Delete.as_str())
    .bind(JobStatus::Queued.as_str())
    .execute(&mut *transaction)
    .await?;
    insert_job(
        &mut transaction,
        media_id,
        JobKind::Delete,
        "{}",
        state.config.jobs.maximum_attempts,
    )
    .await?;
    transaction.commit().await?;
    state.jobs_available.notify_one();
    Ok(())
}

pub async fn retry(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    media_id: &str,
) -> Result<(), AppError> {
    let record = owned_record(&state.pool, &authenticated.account_id, media_id).await?;
    if record.status != MediaStatus::Failed.as_str() {
        return Err(AppError::Conflict(
            "only failed media can be retried".to_owned(),
        ));
    }
    let kind = if record.original_object_key.is_some() {
        JobKind::Transcode
    } else {
        JobKind::RemoteImport
    };
    let payload = if kind == JobKind::RemoteImport {
        let source_url: Option<String> =
            sqlx::query_scalar("SELECT source_url FROM media_files WHERE id = ?")
                .bind(media_id)
                .fetch_one(&state.pool)
                .await?;
        let source_url = source_url.ok_or_else(|| {
            AppError::Conflict("failed remote import has no source URL".to_owned())
        })?;
        serde_json::to_string(&serde_json::json!({ "url": source_url }))
            .map_err(AppError::internal)?
    } else {
        "{}".to_owned()
    };
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "UPDATE media_files SET status = ?, error_message = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(if kind == JobKind::RemoteImport {
        MediaStatus::Importing.as_str()
    } else {
        MediaStatus::Queued.as_str()
    })
    .bind(now())
    .bind(media_id)
    .execute(&mut *transaction)
    .await?;
    insert_job(
        &mut transaction,
        media_id,
        kind,
        &payload,
        state.config.jobs.maximum_attempts,
    )
    .await?;
    transaction.commit().await?;
    state.jobs_available.notify_one();
    Ok(())
}

pub async fn jobs(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    media_id: &str,
) -> Result<Vec<MediaJob>, AppError> {
    owned_record(&state.pool, &authenticated.account_id, media_id).await?;
    let rows = sqlx::query(
        "SELECT id, kind, status, attempts, maximum_attempts, run_after, error_message, created_at, updated_at \
         FROM media_jobs WHERE media_id = ? ORDER BY created_at DESC",
    )
    .bind(media_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| MediaJob {
            id: row.get("id"),
            kind: row.get("kind"),
            status: row.get("status"),
            attempts: row.get("attempts"),
            maximum_attempts: row.get("maximum_attempts"),
            run_after: row.get("run_after"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn assert_account_owns_media(
    pool: &SqlitePool,
    account_id: &str,
    media_id: &str,
) -> Result<(), AppError> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM media_files WHERE id = ? AND account_id = ? AND deleted_at IS NULL",
    )
    .bind(media_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    if exists.is_some() {
        Ok(())
    } else {
        Err(AppError::NotFound("media not found".to_owned()))
    }
}

pub async fn insert_job(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    media_id: &str,
    kind: JobKind,
    payload_json: &str,
    maximum_attempts: i64,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    sqlx::query(
        "INSERT INTO media_jobs \
         (id, media_id, kind, status, payload_json, maximum_attempts, run_after, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(media_id)
    .bind(kind.as_str())
    .bind(JobStatus::Queued.as_str())
    .bind(payload_json)
    .bind(maximum_attempts)
    .bind(&timestamp)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&mut **transaction)
    .await?;
    Ok(id)
}

async fn owned_record(
    pool: &SqlitePool,
    account_id: &str,
    media_id: &str,
) -> Result<MediaRecord, AppError> {
    let row = sqlx::query(
        "SELECT id, account_id, original_object_key, source_filename, source_mime_type, \
                source_size_bytes, storage_bytes, title, description, visibility, status \
         FROM media_files WHERE id = ? AND account_id = ? AND deleted_at IS NULL",
    )
    .bind(media_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("media not found".to_owned()))?;
    Ok(record_from_row(&row))
}

fn record_from_row(row: &sqlx::sqlite::SqliteRow) -> MediaRecord {
    MediaRecord {
        id: row.get("id"),
        account_id: row.get("account_id"),
        original_object_key: row.get("original_object_key"),
        source_filename: row.get("source_filename"),
        source_mime_type: row.get("source_mime_type"),
        source_size_bytes: row.get("source_size_bytes"),
        storage_bytes: row.get("storage_bytes"),
        title: row.get("title"),
        description: row.get("description"),
        visibility: row.get("visibility"),
        status: row.get("status"),
    }
}

async fn hydrate_media(state: &AppState, record: MediaRecord) -> Result<Media, AppError> {
    let row = sqlx::query(
        "SELECT sha256, duration_ms, width, height, video_codec, audio_codec, error_message, \
                created_at, updated_at, published_at \
         FROM media_files WHERE id = ?",
    )
    .bind(&record.id)
    .fetch_one(&state.pool)
    .await?;
    let variant_rows = sqlx::query(
        "SELECT name, width, height, bandwidth_bps, codecs \
         FROM media_variants WHERE media_id = ? ORDER BY height ASC",
    )
    .bind(&record.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Media {
        id: record.id,
        source_filename: record.source_filename,
        source_mime_type: record.source_mime_type,
        source_size_bytes: record.source_size_bytes,
        storage_bytes: record.storage_bytes,
        sha256: row.get("sha256"),
        title: record.title,
        description: record.description,
        visibility: record.visibility,
        status: record.status,
        duration_ms: row.get("duration_ms"),
        width: row.get("width"),
        height: row.get("height"),
        video_codec: row.get("video_codec"),
        audio_codec: row.get("audio_codec"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        published_at: row.get("published_at"),
        variants: variant_rows
            .into_iter()
            .map(|variant| MediaVariant {
                name: variant.get("name"),
                width: variant.get("width"),
                height: variant.get("height"),
                bandwidth_bps: variant.get("bandwidth_bps"),
                codecs: variant.get("codecs"),
            })
            .collect(),
    })
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
