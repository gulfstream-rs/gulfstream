use sqlx::{Row, sqlite::SqlitePoolOptions};

#[tokio::test]
async fn migrations_create_the_expected_core_tables() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("migration-test.db");
    let url = format!("sqlite://{}?mode=rwc", database.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::migrate!().run(&pool).await?;

    let rows = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&pool)
    .await?;
    let names: std::collections::HashSet<String> =
        rows.into_iter().map(|row| row.get("name")).collect();
    for expected in [
        "accounts",
        "api_keys",
        "browser_sessions",
        "media_files",
        "media_variants",
        "media_jobs",
        "playback_sessions",
        "analytics_events",
        "daily_viewers",
        "daily_stats",
    ] {
        assert!(names.contains(expected), "missing table {expected}");
    }
    Ok(())
}
