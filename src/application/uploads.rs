use std::path::PathBuf;

use axum::extract::Multipart;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::media::insert_job,
    domain::{
        account::AuthenticatedAccount,
        media::{JobKind, MediaStatus, Visibility},
    },
    error::AppError,
    state::AppState,
    util::{
        CleanupPath, clean_filename, read_text_field, title_from_filename, validate_remote_url,
        write_upload_field,
    },
};

#[derive(Debug, serde::Serialize)]
pub struct CreatedMedia {
    pub id: String,
    pub status: MediaStatus,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteImportRequest {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
}

struct PendingUpload {
    path: PathBuf,
    filename: String,
    mime_type: String,
    size: u64,
    sha256: String,
}

#[derive(Default)]
struct UploadMetadata {
    title: Option<String>,
    description: String,
    visibility: Option<Visibility>,
}

pub async fn create_from_multipart(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    mut multipart: Multipart,
) -> Result<CreatedMedia, AppError> {
    let temporary_id = Uuid::new_v4().to_string();
    let temporary_path = state
        .storage
        .temporary_file(&format!("upload-{temporary_id}"));
    let mut temporary_cleanup = CleanupPath::file(&temporary_path);
    let mut pending: Option<PendingUpload> = None;
    let mut metadata = UploadMetadata::default();
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "file" => {
                if pending.is_some() {
                    return cleanup_error(
                        &temporary_path,
                        AppError::BadRequest(
                            "exactly one file is accepted per media upload".to_owned(),
                        ),
                    )
                    .await;
                }
                let filename = clean_filename(
                    field
                        .file_name()
                        .unwrap_or(state.config.uploads.default_filename.as_str()),
                    &state.config.uploads.default_filename,
                    state.config.uploads.maximum_filename_bytes,
                );
                let mime_type = field.content_type().map_or_else(
                    || state.config.uploads.default_content_type.clone(),
                    |value| value.to_ascii_lowercase(),
                );
                if !state
                    .config
                    .uploads
                    .accepted_content_types
                    .iter()
                    .any(|accepted| accepted == &mime_type)
                {
                    return cleanup_error(
                        &temporary_path,
                        AppError::BadRequest(format!("content type {mime_type} is not accepted")),
                    )
                    .await;
                }
                let (size, sha256) = write_upload_field(
                    field,
                    &temporary_path,
                    state.config.uploads.maximum_source_bytes,
                )
                .await?;
                if size < state.config.uploads.minimum_source_bytes {
                    return Err(AppError::BadRequest(format!(
                        "source must contain at least {} bytes",
                        state.config.uploads.minimum_source_bytes
                    )));
                }
                pending = Some(PendingUpload {
                    path: temporary_path.clone(),
                    filename,
                    mime_type,
                    size,
                    sha256,
                });
            }
            "title" => {
                metadata.title = non_empty(
                    read_text_field(field, state.config.uploads.maximum_text_field_bytes).await?,
                );
            }
            "description" => {
                metadata.description =
                    read_text_field(field, state.config.uploads.maximum_text_field_bytes).await?;
            }
            "visibility" => {
                let value =
                    read_text_field(field, state.config.uploads.maximum_text_field_bytes).await?;
                metadata.visibility = Some(parse_visibility(&value)?);
            }
            _ => {
                return cleanup_error(
                    &temporary_path,
                    AppError::BadRequest(format!("unknown multipart field: {name}")),
                )
                .await;
            }
        }
    }
    let pending = pending.ok_or_else(|| AppError::BadRequest("file is required".to_owned()))?;
    let size = i64::try_from(pending.size)
        .map_err(|_| AppError::PayloadTooLarge("source size exceeds database limits".to_owned()))?;
    let media_id = Uuid::new_v4().to_string();
    let original_directory = state
        .storage
        .original_directory(&authenticated.account_id, &media_id);
    tokio::fs::create_dir_all(&original_directory).await?;
    let destination = original_directory.join(&pending.filename);
    let media_root = state
        .storage
        .media_root(&authenticated.account_id, &media_id);
    let mut media_cleanup = CleanupPath::directory(&media_root);
    tokio::fs::rename(&pending.path, &destination).await?;
    temporary_cleanup.disarm();
    let object_key = state
        .storage
        .object_key(&destination)
        .map_err(AppError::internal)?;
    let title = metadata
        .title
        .unwrap_or_else(|| title_from_filename(&pending.filename));
    let visibility = metadata
        .visibility
        .unwrap_or(state.config.uploads.default_visibility);
    let status = if state.config.transcoding.enabled {
        MediaStatus::Queued
    } else {
        MediaStatus::Ready
    };
    let timestamp = now();
    let mut transaction = state.pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO media_files \
         (id, account_id, source_filename, original_object_key, source_mime_type, source_size_bytes, \
          storage_bytes, sha256, title, description, visibility, status, created_at, updated_at, published_at) \
         SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
         WHERE (SELECT storage_quota_bytes FROM accounts WHERE id = ?) >= \
               ((SELECT COALESCE(SUM(storage_bytes), 0) FROM media_files WHERE account_id = ?) + ?)",
    )
    .bind(&media_id)
    .bind(&authenticated.account_id)
    .bind(&pending.filename)
    .bind(&object_key)
    .bind(&pending.mime_type)
    .bind(size)
    .bind(size)
    .bind(&pending.sha256)
    .bind(&title)
    .bind(&metadata.description)
    .bind(visibility.as_str())
    .bind(status.as_str())
    .bind(&timestamp)
    .bind(&timestamp)
    .bind((status == MediaStatus::Ready).then_some(timestamp.as_str()))
    .bind(&authenticated.account_id)
    .bind(&authenticated.account_id)
    .bind(size)
    .execute(&mut *transaction)
    .await?;
    if inserted.rows_affected() == 0 {
        transaction.rollback().await?;
        return Err(AppError::PayloadTooLarge(
            "account storage quota would be exceeded".to_owned(),
        ));
    }
    if state.config.transcoding.enabled {
        insert_job(
            &mut transaction,
            &media_id,
            JobKind::Transcode,
            "{}",
            state.config.jobs.maximum_attempts,
        )
        .await?;
    }
    transaction.commit().await?;
    media_cleanup.disarm();
    if state.config.transcoding.enabled {
        state.jobs_available.notify_one();
    }
    Ok(CreatedMedia {
        id: media_id,
        status,
    })
}

pub async fn create_remote_import(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    request: RemoteImportRequest,
) -> Result<CreatedMedia, AppError> {
    if !state.config.remote_imports.enabled {
        return Err(AppError::Forbidden);
    }
    let url = reqwest::Url::parse(request.url.trim())
        .map_err(|error| AppError::BadRequest(format!("invalid URL: {error}")))?;
    validate_remote_url(&url, state.config.remote_imports.allow_private_networks).await?;
    let filename = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.is_empty())
        .map(|value| {
            clean_filename(
                value,
                &state.config.uploads.default_remote_filename,
                state.config.uploads.maximum_filename_bytes,
            )
        })
        .unwrap_or_else(|| state.config.uploads.default_remote_filename.clone());
    let title = request
        .title
        .and_then(non_empty)
        .unwrap_or_else(|| title_from_filename(&filename));
    validate_text_size(
        &title,
        state.config.uploads.maximum_text_field_bytes,
        "title",
    )?;
    let description = request.description.unwrap_or_default();
    validate_text_size(
        &description,
        state.config.uploads.maximum_text_field_bytes,
        "description",
    )?;
    let visibility = request
        .visibility
        .unwrap_or(state.config.uploads.default_visibility);
    let media_id = Uuid::new_v4().to_string();
    let timestamp = now();
    let payload = serde_json::to_string(&serde_json::json!({ "url": url.as_str() }))
        .map_err(AppError::internal)?;
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO media_files \
         (id, account_id, source_filename, source_url, title, description, visibility, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&media_id)
    .bind(&authenticated.account_id)
    .bind(&filename)
    .bind(url.as_str())
    .bind(title)
    .bind(description)
    .bind(visibility.as_str())
    .bind(MediaStatus::Importing.as_str())
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&mut *transaction)
    .await?;
    insert_job(
        &mut transaction,
        &media_id,
        JobKind::RemoteImport,
        &payload,
        state.config.jobs.maximum_attempts,
    )
    .await?;
    transaction.commit().await?;
    state.jobs_available.notify_one();
    Ok(CreatedMedia {
        id: media_id,
        status: MediaStatus::Importing,
    })
}

async fn cleanup_error<T>(path: &std::path::Path, error: AppError) -> Result<T, AppError> {
    let _ = tokio::fs::remove_file(path).await;
    Err(error)
}

fn validate_text_size(value: &str, maximum_bytes: usize, field: &str) -> Result<(), AppError> {
    if value.len() > maximum_bytes {
        return Err(AppError::PayloadTooLarge(format!(
            "{field} exceeds the configured limit of {maximum_bytes} bytes"
        )));
    }
    Ok(())
}

fn parse_visibility(value: &str) -> Result<Visibility, AppError> {
    value.trim().to_ascii_lowercase().parse().map_err(|()| {
        AppError::BadRequest("visibility must be private, unlisted, or public".to_owned())
    })
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
