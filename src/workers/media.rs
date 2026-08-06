use std::time::Duration;

use anyhow::{Context, bail};
use chrono::{Duration as ChronoDuration, Utc};
use futures_util::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::{io::AsyncWriteExt, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    application::media::insert_job,
    domain::media::JobKind,
    infrastructure::{ffmpeg, storage::directory_size},
    state::AppState,
    util::{CleanupPath, clean_filename, validate_remote_url},
};

#[derive(Clone, Debug)]
struct ClaimedJob {
    id: String,
    media_id: String,
    kind: JobKind,
    payload_json: String,
    attempts: i64,
    maximum_attempts: i64,
    locked_by: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemotePayload {
    url: String,
}

pub fn spawn(state: AppState) -> Vec<JoinHandle<()>> {
    (0..state.config.jobs.worker_count)
        .map(|index| {
            let state = state.clone();
            tokio::spawn(async move {
                let worker_id = format!("{}-{index}", Uuid::new_v4());
                run_worker(state, worker_id).await;
            })
        })
        .collect()
}

async fn run_worker(state: AppState, worker_id: String) {
    let poll = Duration::from_millis(state.config.jobs.poll_interval_ms);
    loop {
        if state.cancellation.is_cancelled() {
            return;
        }
        match claim_job(&state, &worker_id).await {
            Ok(Some(job)) => {
                let lease_cancellation = CancellationToken::new();
                let lease_task = spawn_lease_heartbeat(
                    state.clone(),
                    job.id.clone(),
                    worker_id.clone(),
                    lease_cancellation.clone(),
                );
                let result = process_job(&state, &job).await;
                lease_cancellation.cancel();
                if let Err(error) = lease_task.await {
                    tracing::error!(%error, job_id = %job.id, "job lease heartbeat task failed");
                }
                if let Err(error) = finish_job(&state, &job, result).await {
                    tracing::error!(%error, job_id = %job.id, "failed to finalize media job");
                }
            }
            Ok(None) => {
                tokio::select! {
                    () = state.cancellation.cancelled() => return,
                    () = state.jobs_available.notified() => {},
                    () = tokio::time::sleep(poll) => {},
                }
            }
            Err(error) => {
                tracing::error!(%error, %worker_id, "failed to claim media job");
                tokio::select! {
                    () = state.cancellation.cancelled() => return,
                    () = tokio::time::sleep(poll) => {},
                }
            }
        }
    }
}

fn spawn_lease_heartbeat(
    state: AppState,
    job_id: String,
    worker_id: String,
    cancellation: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let interval = Duration::from_secs(state.config.jobs.lease_renewal_interval_seconds);
        loop {
            tokio::select! {
                () = cancellation.cancelled() => return,
                () = tokio::time::sleep(interval) => {
                    let result = sqlx::query(
                        "UPDATE media_jobs SET locked_at = ?, updated_at = ? \
                         WHERE id = ? AND status = 'running' AND locked_by = ?",
                    )
                    .bind(Utc::now().to_rfc3339())
                    .bind(Utc::now().to_rfc3339())
                    .bind(&job_id)
                    .bind(&worker_id)
                    .execute(&state.pool)
                    .await;
                    match result {
                        Ok(result) if result.rows_affected() == 0 => return,
                        Ok(_) => {}
                        Err(error) => tracing::error!(%error, %job_id, "failed to renew job lease"),
                    }
                }
            }
        }
    })
}

async fn claim_job(state: &AppState, worker_id: &str) -> anyhow::Result<Option<ClaimedJob>> {
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let stale_before =
        (now - ChronoDuration::seconds(state.config.jobs.lease_seconds)).to_rfc3339();
    let mut transaction = state.pool.begin().await?;

    sqlx::query(
        "UPDATE media_jobs SET status = 'cancelled', locked_at = NULL, locked_by = NULL, \
                error_message = NULL, updated_at = ? \
         WHERE status = 'running' AND locked_at < ? AND kind != 'delete' \
           AND EXISTS (SELECT 1 FROM media_files WHERE media_files.id = media_jobs.media_id \
                       AND media_files.deleted_at IS NOT NULL)",
    )
    .bind(&now_text)
    .bind(&stale_before)
    .execute(&mut *transaction)
    .await?;

    let exhausted = sqlx::query(
        "UPDATE media_jobs SET status = 'failed', locked_at = NULL, locked_by = NULL, \
                error_message = 'worker lease expired after the final attempt', updated_at = ? \
         WHERE status = 'running' AND locked_at < ? AND attempts >= maximum_attempts \
         RETURNING media_id, kind",
    )
    .bind(&now_text)
    .bind(&stale_before)
    .fetch_all(&mut *transaction)
    .await?;
    for row in exhausted {
        let kind: String = row.get("kind");
        if kind != JobKind::Delete.as_str() {
            sqlx::query(
                "UPDATE media_files SET status = 'failed', \
                        error_message = 'worker lease expired after the final attempt', updated_at = ? \
                 WHERE id = ? AND deleted_at IS NULL",
            )
            .bind(&now_text)
            .bind(row.get::<String, _>("media_id"))
            .execute(&mut *transaction)
            .await?;
        }
    }

    sqlx::query(
        "UPDATE media_jobs SET status = 'queued', locked_at = NULL, locked_by = NULL, updated_at = ? \
         WHERE status = 'running' AND locked_at < ? AND attempts < maximum_attempts",
    )
    .bind(&now_text)
    .bind(&stale_before)
    .execute(&mut *transaction)
    .await?;

    let row = sqlx::query(
        "UPDATE media_jobs SET status = 'running', attempts = attempts + 1, locked_at = ?, \
                locked_by = ?, error_message = NULL, updated_at = ? \
         WHERE id = ( \
             SELECT candidate.id FROM media_jobs AS candidate \
             WHERE candidate.status = 'queued' AND candidate.run_after <= ? \
               AND NOT EXISTS ( \
                   SELECT 1 FROM media_jobs AS active \
                   WHERE active.media_id = candidate.media_id AND active.status = 'running' \
               ) \
             ORDER BY candidate.run_after ASC, candidate.created_at ASC LIMIT 1 \
         ) AND status = 'queued' \
         RETURNING id, media_id, kind, payload_json, attempts, maximum_attempts",
    )
    .bind(&now_text)
    .bind(worker_id)
    .bind(&now_text)
    .bind(&now_text)
    .fetch_optional(&mut *transaction)
    .await?;
    transaction.commit().await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let kind = row
        .get::<String, _>("kind")
        .parse::<JobKind>()
        .map_err(|()| anyhow::anyhow!("database contains an unsupported media job kind"))?;
    Ok(Some(ClaimedJob {
        id: row.get("id"),
        media_id: row.get("media_id"),
        kind,
        payload_json: row.get("payload_json"),
        attempts: row.get("attempts"),
        maximum_attempts: row.get("maximum_attempts"),
        locked_by: worker_id.to_owned(),
    }))
}

async fn process_job(state: &AppState, job: &ClaimedJob) -> anyhow::Result<()> {
    match job.kind {
        JobKind::RemoteImport => process_remote_import(state, job).await,
        JobKind::Transcode => process_transcode(state, job).await,
        JobKind::Delete => process_delete(state, job).await,
    }
}

async fn process_remote_import(state: &AppState, job: &ClaimedJob) -> anyhow::Result<()> {
    let payload: RemotePayload =
        serde_json::from_str(&job.payload_json).context("parse remote import job")?;
    let mut url = reqwest::Url::parse(&payload.url).context("parse remote import URL")?;
    let row = sqlx::query(
        "SELECT account_id, source_filename, original_object_key, status \
         FROM media_files WHERE id = ?",
    )
    .bind(&job.media_id)
    .fetch_optional(&state.pool)
    .await?
    .context("remote import media is missing")?;
    let account_id: String = row.get("account_id");
    let source_filename: String = row.get("source_filename");
    let original_object_key: Option<String> = row.get("original_object_key");
    let media_status: String = row.get("status");
    if original_object_key.is_some()
        && matches!(media_status.as_str(), "queued" | "processing" | "ready")
    {
        return Ok(());
    }
    if media_status != "importing" {
        bail!("remote import media has invalid status {media_status}");
    }
    let temporary_path = state
        .storage
        .temporary_file(&format!("remote-{}-{}", job.id, job.locked_by));
    let mut temporary_cleanup = CleanupPath::file(&temporary_path);
    let mut redirects = 0_usize;
    let response = loop {
        let addresses =
            validate_remote_url(&url, state.config.remote_imports.allow_private_networks).await?;
        let host = url.host_str().context("remote URL has no host")?;
        let mut client_builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(&state.config.remote_imports.user_agent)
            .timeout(Duration::from_secs(
                state.config.remote_imports.request_timeout_seconds,
            ))
            .resolve_to_addrs(host, &addresses);
        if !state.config.remote_imports.use_system_proxy {
            client_builder = client_builder.no_proxy();
        }
        let response = client_builder.build()?.get(url.clone()).send().await?;
        if response.status().is_redirection() {
            if redirects >= state.config.remote_imports.maximum_redirects {
                bail!("remote import exceeded the configured redirect limit");
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .context("remote redirect has no valid Location header")?;
            url = url.join(location).context("resolve remote redirect")?;
            redirects += 1;
            continue;
        }
        if !response.status().is_success() {
            bail!("remote server returned HTTP {}", response.status());
        }
        break response;
    };
    if response
        .content_length()
        .is_some_and(|length| length > state.config.remote_imports.maximum_source_bytes)
    {
        bail!("remote source exceeds the configured size limit");
    }
    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(state.config.uploads.default_content_type.as_str())
        .to_owned();
    if !state
        .config
        .uploads
        .accepted_content_types
        .iter()
        .any(|accepted| accepted == &mime_type)
    {
        bail!("remote source content type {mime_type} is not accepted");
    }
    let mut output = tokio::fs::File::create(&temporary_path).await?;
    let mut stream = response.bytes_stream();
    let mut size = 0_u64;
    let mut digest = Sha256::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        size = size.saturating_add(chunk.len() as u64);
        if size > state.config.remote_imports.maximum_source_bytes {
            let _ = tokio::fs::remove_file(&temporary_path).await;
            bail!("remote source exceeds the configured size limit");
        }
        digest.update(&chunk);
        output.write_all(&chunk).await?;
    }
    output.flush().await?;
    output.sync_all().await?;
    if size < state.config.uploads.minimum_source_bytes {
        bail!(
            "remote source must contain at least {} bytes",
            state.config.uploads.minimum_source_bytes
        );
    }
    let source_size = i64::try_from(size).context("remote source size exceeds SQLite limits")?;
    assert_job_lease(state, job).await?;
    let filename = clean_filename(
        &source_filename,
        &state.config.uploads.default_remote_filename,
        state.config.uploads.maximum_filename_bytes,
    );
    let media_root = state.storage.media_root(&account_id, &job.media_id);
    let mut media_cleanup = CleanupPath::directory(&media_root);
    let original_directory = state.storage.original_directory(&account_id, &job.media_id);
    tokio::fs::create_dir_all(&original_directory).await?;
    let destination = original_directory.join(filename);
    tokio::fs::rename(&temporary_path, &destination).await?;
    temporary_cleanup.disarm();
    let object_key = state.storage.object_key(&destination)?;
    let sha256 = hex::encode(digest.finalize());
    let timestamp = Utc::now().to_rfc3339();
    let next_status = if state.config.transcoding.enabled {
        "queued"
    } else {
        "ready"
    };
    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE media_files SET original_object_key = ?, source_url = ?, source_mime_type = ?, \
                source_size_bytes = ?, storage_bytes = ?, sha256 = ?, status = ?, error_message = NULL, \
                updated_at = ?, published_at = ? \
         WHERE id = ? AND status = 'importing' AND deleted_at IS NULL
           AND EXISTS (SELECT 1 FROM media_jobs
                       WHERE media_jobs.id = ? AND media_jobs.status = 'running'
                         AND media_jobs.locked_by = ?)
           AND (SELECT storage_quota_bytes FROM accounts WHERE id = media_files.account_id) >=
               ((SELECT COALESCE(SUM(storage_bytes), 0) FROM media_files AS quota_media
                 WHERE quota_media.account_id = media_files.account_id) + ?)",
    )
    .bind(&object_key)
    .bind(url.as_str())
    .bind(mime_type)
    .bind(source_size)
    .bind(source_size)
    .bind(sha256)
    .bind(next_status)
    .bind(&timestamp)
    .bind((next_status == "ready").then_some(timestamp.as_str()))
    .bind(&job.media_id)
    .bind(&job.id)
    .bind(&job.locked_by)
    .bind(source_size)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        transaction.rollback().await?;
        let still_importing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM media_files WHERE id = ? AND status = 'importing' AND deleted_at IS NULL)",
        )
        .bind(&job.media_id)
        .fetch_one(&state.pool)
        .await?;
        if still_importing {
            bail!("account storage quota would be exceeded");
        }
        return Ok(());
    }
    if state.config.transcoding.enabled {
        insert_job(
            &mut transaction,
            &job.media_id,
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
    Ok(())
}

async fn process_transcode(state: &AppState, job: &ClaimedJob) -> anyhow::Result<()> {
    let row = sqlx::query(
        "SELECT account_id, original_object_key, source_size_bytes, status \
         FROM media_files WHERE id = ?",
    )
    .bind(&job.media_id)
    .fetch_optional(&state.pool)
    .await?
    .context("transcode media is missing")?;
    let account_id: String = row.get("account_id");
    let status: String = row.get("status");
    if status == "ready" {
        let variant_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM media_variants WHERE media_id = ?")
                .bind(&job.media_id)
                .fetch_one(&state.pool)
                .await?;
        if variant_count > 0 {
            return Ok(());
        }
    } else if !matches!(status.as_str(), "queued" | "failed" | "processing") {
        bail!("transcode media has invalid status {status}");
    }
    let object_key: String = row
        .get::<Option<String>, _>("original_object_key")
        .context("media has no source object")?;
    let source_size: i64 = row.get("source_size_bytes");
    let claimed_media = sqlx::query(
        "UPDATE media_files SET status = 'processing', error_message = NULL, updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL AND status IN ('queued', 'failed', 'processing', 'ready')",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(&job.media_id)
    .execute(&state.pool)
    .await?;
    if claimed_media.rows_affected() == 0 {
        return Ok(());
    }
    let source_path = state.storage.object_path(&object_key)?;
    let staging = state
        .storage
        .stream_staging_directory(&format!("{}-{}", job.id, job.locked_by));
    if tokio::fs::metadata(&staging).await.is_ok() {
        tokio::fs::remove_dir_all(&staging).await?;
    }
    let mut staging_cleanup = CleanupPath::directory(&staging);
    let probe = ffmpeg::probe(&source_path, &state.config.transcoding).await?;
    let variants =
        match ffmpeg::generate_hls(&source_path, &staging, &probe, &state.config.transcoding).await
        {
            Ok(variants) => variants,
            Err(error) => {
                let _ = tokio::fs::remove_dir_all(&staging).await;
                return Err(error);
            }
        };
    let generated_size_u64 = directory_size(&staging).await?;
    let generated_size =
        i64::try_from(generated_size_u64).context("generated media size exceeds SQLite limits")?;
    assert_job_lease(state, job).await?;
    let retained_source = if state.config.transcoding.retain_original {
        source_size
    } else {
        0
    };
    let target_storage = retained_source.saturating_add(generated_size);
    let final_stream = state.storage.stream_directory(&account_id, &job.media_id);
    if tokio::fs::metadata(&final_stream).await.is_ok() {
        tokio::fs::remove_dir_all(&final_stream).await?;
    }
    if let Some(parent) = final_stream.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&staging, &final_stream).await?;
    staging_cleanup.disarm();
    let mut final_stream_cleanup = CleanupPath::directory(&final_stream);
    let timestamp = Utc::now().to_rfc3339();
    let mut transaction = state.pool.begin().await?;
    sqlx::query("DELETE FROM media_variants WHERE media_id = ?")
        .bind(&job.media_id)
        .execute(&mut *transaction)
        .await?;
    for variant in variants {
        let relative_playlist = final_stream.join(&variant.name).join(
            variant
                .playlist_path
                .file_name()
                .context("variant playlist has no filename")?,
        );
        let playlist_object_key = state.storage.object_key(&relative_playlist)?;
        let variant_directory_size = directory_size(&final_stream.join(&variant.name)).await?;
        sqlx::query(
            "INSERT INTO media_variants \
             (id, media_id, name, width, height, bandwidth_bps, codecs, playlist_object_key, storage_bytes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&job.media_id)
        .bind(&variant.name)
        .bind(i64::from(variant.width))
        .bind(i64::from(variant.height))
        .bind(i64::try_from(variant.bandwidth_bps).context("variant bandwidth overflow")?)
        .bind(&variant.codecs)
        .bind(playlist_object_key)
        .bind(i64::try_from(variant_directory_size).context("variant storage size overflow")?)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await?;
    }
    let updated = sqlx::query(
        "UPDATE media_files SET status = 'ready', duration_ms = ?, width = ?, height = ?, \
                video_codec = ?, audio_codec = ?, storage_bytes = ?, error_message = NULL, \
                updated_at = ?, published_at = COALESCE(published_at, ?), \
                original_object_key = CASE WHEN ? = 1 THEN original_object_key ELSE NULL END \
         WHERE id = ? AND status = 'processing' AND deleted_at IS NULL
           AND EXISTS (SELECT 1 FROM media_jobs
                       WHERE media_jobs.id = ? AND media_jobs.status = 'running'
                         AND media_jobs.locked_by = ?)
           AND (SELECT storage_quota_bytes FROM accounts WHERE id = media_files.account_id) >=
               ((SELECT COALESCE(SUM(storage_bytes), 0) FROM media_files AS quota_media
                 WHERE quota_media.account_id = media_files.account_id)
                - media_files.storage_bytes + ?)",
    )
    .bind(probe.duration_ms)
    .bind(probe.width.map(i64::from))
    .bind(probe.height.map(i64::from))
    .bind(probe.video_codec)
    .bind(probe.audio_codec)
    .bind(target_storage)
    .bind(&timestamp)
    .bind(&timestamp)
    .bind(if state.config.transcoding.retain_original {
        1_i64
    } else {
        0_i64
    })
    .bind(&job.media_id)
    .bind(&job.id)
    .bind(&job.locked_by)
    .bind(target_storage)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        transaction.rollback().await?;
        let still_processing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM media_files WHERE id = ? AND status = 'processing' AND deleted_at IS NULL)",
        )
        .bind(&job.media_id)
        .fetch_one(&state.pool)
        .await?;
        if still_processing {
            bail!("transcoded media would exceed the account storage quota");
        }
        return Ok(());
    }
    transaction.commit().await?;
    final_stream_cleanup.disarm();
    if !state.config.transcoding.retain_original {
        if let Some(parent) = source_path.parent() {
            let _ = tokio::fs::remove_dir_all(parent).await;
        } else {
            let _ = tokio::fs::remove_file(&source_path).await;
        }
    }
    Ok(())
}

async fn process_delete(state: &AppState, job: &ClaimedJob) -> anyhow::Result<()> {
    assert_job_lease(state, job).await?;
    let row = sqlx::query("SELECT account_id FROM media_files WHERE id = ?")
        .bind(&job.media_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(row) = row else {
        return Ok(());
    };
    let account_id: String = row.get("account_id");
    let storage_bytes = if state.config.storage.preserve_deleted_media {
        state
            .storage
            .move_to_trash(&account_id, &job.media_id)
            .await?;
        sqlx::query_scalar::<_, i64>("SELECT storage_bytes FROM media_files WHERE id = ?")
            .bind(&job.media_id)
            .fetch_one(&state.pool)
            .await?
    } else {
        let path = state.storage.media_root(&account_id, &job.media_id);
        if tokio::fs::metadata(&path).await.is_ok() {
            tokio::fs::remove_dir_all(path).await?;
        }
        0
    };
    sqlx::query(
        "UPDATE media_files SET status = 'deleted', storage_bytes = ?, updated_at = ? \
         WHERE id = ? AND EXISTS (SELECT 1 FROM media_jobs \
             WHERE media_jobs.id = ? AND media_jobs.status = 'running' AND media_jobs.locked_by = ?)",
    )
    .bind(storage_bytes)
    .bind(Utc::now().to_rfc3339())
    .bind(&job.media_id)
    .bind(&job.id)
    .bind(&job.locked_by)
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn assert_job_lease(state: &AppState, job: &ClaimedJob) -> anyhow::Result<()> {
    let fresh_after =
        (Utc::now() - ChronoDuration::seconds(state.config.jobs.lease_seconds)).to_rfc3339();
    let lease_is_current: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
            SELECT 1 FROM media_jobs \
            WHERE id = ? AND status = 'running' AND locked_by = ? AND locked_at >= ? \
              AND (kind = 'delete' OR EXISTS ( \
                  SELECT 1 FROM media_files \
                  WHERE media_files.id = media_jobs.media_id AND media_files.deleted_at IS NULL \
              )) \
        )",
    )
    .bind(&job.id)
    .bind(&job.locked_by)
    .bind(fresh_after)
    .fetch_one(&state.pool)
    .await?;
    if !lease_is_current {
        bail!("media job lease is no longer current");
    }
    Ok(())
}

async fn finish_job(
    state: &AppState,
    job: &ClaimedJob,
    result: anyhow::Result<()>,
) -> anyhow::Result<()> {
    let timestamp = Utc::now();
    match result {
        Ok(()) => {
            sqlx::query(
                "UPDATE media_jobs SET status = 'succeeded', locked_at = NULL, locked_by = NULL, \
                        error_message = NULL, updated_at = ? \
                 WHERE id = ? AND status = 'running' AND locked_by = ?",
            )
            .bind(timestamp.to_rfc3339())
            .bind(&job.id)
            .bind(&job.locked_by)
            .execute(&state.pool)
            .await?;
        }
        Err(error) => {
            let message = format!("{error:#}");
            if job.kind != JobKind::Delete {
                let deletion_requested: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM media_files WHERE id = ? AND deleted_at IS NOT NULL)",
                )
                .bind(&job.media_id)
                .fetch_one(&state.pool)
                .await?;
                if deletion_requested {
                    sqlx::query(
                        "UPDATE media_jobs SET status = 'cancelled', locked_at = NULL, locked_by = NULL, \
                                error_message = NULL, updated_at = ? \
                         WHERE id = ? AND status = 'running' AND locked_by = ?",
                    )
                    .bind(timestamp.to_rfc3339())
                    .bind(&job.id)
                    .bind(&job.locked_by)
                    .execute(&state.pool)
                    .await?;
                    return Ok(());
                }
            }
            if job.attempts < job.maximum_attempts {
                let exponent = u32::try_from(job.attempts.saturating_sub(1))
                    .unwrap_or(u32::MAX)
                    .min(30);
                let multiplier = 2_i64.saturating_pow(exponent);
                let delay = state
                    .config
                    .jobs
                    .retry_base_delay_seconds
                    .saturating_mul(multiplier);
                let run_after = timestamp + ChronoDuration::seconds(delay);
                let updated = sqlx::query(
                    "UPDATE media_jobs SET status = 'queued', run_after = ?, locked_at = NULL, locked_by = NULL, \
                            error_message = ?, updated_at = ? \
                     WHERE id = ? AND status = 'running' AND locked_by = ?",
                )
                .bind(run_after.to_rfc3339())
                .bind(&message)
                .bind(timestamp.to_rfc3339())
                .bind(&job.id)
                .bind(&job.locked_by)
                .execute(&state.pool)
                .await?;
                if updated.rows_affected() == 1 {
                    state.jobs_available.notify_one();
                }
            } else {
                let updated = sqlx::query(
                    "UPDATE media_jobs SET status = 'failed', locked_at = NULL, locked_by = NULL, \
                            error_message = ?, updated_at = ? \
                     WHERE id = ? AND status = 'running' AND locked_by = ?",
                )
                .bind(&message)
                .bind(timestamp.to_rfc3339())
                .bind(&job.id)
                .bind(&job.locked_by)
                .execute(&state.pool)
                .await?;
                if updated.rows_affected() == 1 && job.kind != JobKind::Delete {
                    sqlx::query(
                        "UPDATE media_files SET status = 'failed', error_message = ?, updated_at = ? \
                         WHERE id = ? AND deleted_at IS NULL",
                    )
                    .bind(&message)
                    .bind(timestamp.to_rfc3339())
                    .bind(&job.media_id)
                    .execute(&state.pool)
                    .await?;
                }
            }
            tracing::warn!(job_id = %job.id, media_id = %job.media_id, error = %message, "media job failed");
        }
    }
    Ok(())
}
