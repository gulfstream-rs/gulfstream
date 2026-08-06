use sqlx::sqlite::SqlitePoolOptions;

async fn migrated_pool() -> anyhow::Result<(tempfile::TempDir, sqlx::SqlitePool)> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("invariants.db");
    let url = format!("sqlite://{}?mode=rwc", database.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok((directory, pool))
}

async fn insert_account_and_media(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO accounts \
         (id, email, display_name, password_hash, status, storage_quota_bytes, created_at, updated_at) \
         VALUES ('account', 'owner@example.com', 'Owner', 'test-password-hash', 'active', 1000000, 'now', 'now')",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO media_files \
         (id, account_id, source_filename, title, description, visibility, status, created_at, updated_at) \
         VALUES ('media', 'account', 'video.mp4', 'Video', '', 'private', 'ready', 'now', 'now')",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn first_play_and_daily_viewer_are_counted_once() -> anyhow::Result<()> {
    let (_directory, pool) = migrated_pool().await?;
    insert_account_and_media(&pool).await?;
    sqlx::query(
        "INSERT INTO playback_sessions \
         (id, media_id, account_id, viewer_hash, started_at, last_event_at) \
         VALUES ('session', 'media', 'account', 'viewer', 'now', 'now')",
    )
    .execute(&pool)
    .await?;

    let first = sqlx::query(
        "UPDATE playback_sessions SET play_started_at = 'first' \
         WHERE id = 'session' AND play_started_at IS NULL",
    )
    .execute(&pool)
    .await?;
    let duplicate = sqlx::query(
        "UPDATE playback_sessions SET play_started_at = 'duplicate' \
         WHERE id = 'session' AND play_started_at IS NULL",
    )
    .execute(&pool)
    .await?;
    assert_eq!(first.rows_affected(), 1);
    assert_eq!(duplicate.rows_affected(), 0);

    let first_viewer = sqlx::query(
        "INSERT OR IGNORE INTO daily_viewers \
         (day, account_id, media_id, viewer_hash) \
         VALUES ('2026-08-06', 'account', 'media', 'viewer')",
    )
    .execute(&pool)
    .await?;
    let duplicate_viewer = sqlx::query(
        "INSERT OR IGNORE INTO daily_viewers \
         (day, account_id, media_id, viewer_hash) \
         VALUES ('2026-08-06', 'account', 'media', 'viewer')",
    )
    .execute(&pool)
    .await?;
    assert_eq!(first_viewer.rows_affected(), 1);
    assert_eq!(duplicate_viewer.rows_affected(), 0);
    Ok(())
}

#[tokio::test]
async fn only_one_active_job_of_each_kind_exists_per_media() -> anyhow::Result<()> {
    let (_directory, pool) = migrated_pool().await?;
    insert_account_and_media(&pool).await?;
    sqlx::query(
        "INSERT INTO media_jobs \
         (id, media_id, kind, status, payload_json, maximum_attempts, run_after, created_at, updated_at) \
         VALUES ('job-1', 'media', 'transcode', 'queued', '{}', 3, 'now', 'now', 'now')",
    )
    .execute(&pool)
    .await?;
    let duplicate = sqlx::query(
        "INSERT INTO media_jobs \
         (id, media_id, kind, status, payload_json, maximum_attempts, run_after, created_at, updated_at) \
         VALUES ('job-2', 'media', 'transcode', 'queued', '{}', 3, 'now', 'now', 'now')",
    )
    .execute(&pool)
    .await;
    assert!(duplicate.is_err());

    sqlx::query("UPDATE media_jobs SET status = 'succeeded' WHERE id = 'job-1'")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO media_jobs \
         (id, media_id, kind, status, payload_json, maximum_attempts, run_after, created_at, updated_at) \
         VALUES ('job-3', 'media', 'transcode', 'queued', '{}', 3, 'now', 'now', 'now')",
    )
    .execute(&pool)
    .await?;
    Ok(())
}
