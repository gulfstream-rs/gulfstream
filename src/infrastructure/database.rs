use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::Context;
use sqlx::{
    AssertSqlSafe, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::config::DatabaseConfig;

pub async fn connect(config: &DatabaseConfig) -> anyhow::Result<SqlitePool> {
    if let Some(database_path) = sqlite_file_path(&config.url)
        && let Some(parent) = database_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
    {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create database directory {}", parent.display()))?;
    }
    let options = SqliteConnectOptions::from_str(&config.url)
        .context("parse database.url")?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(config.busy_timeout_ms));
    let journal_mode = config.journal_mode.as_sql().to_owned();
    let synchronous = config.synchronous.as_sql().to_owned();
    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .after_connect(move |connection, _metadata| {
            let journal_mode = journal_mode.clone();
            let synchronous = synchronous.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(format!(
                    "PRAGMA journal_mode = {journal_mode}"
                )))
                .execute(&mut *connection)
                .await?;
                sqlx::query(AssertSqlSafe(format!("PRAGMA synchronous = {synchronous}")))
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .context("connect to SQLite")?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("run database migrations")?;
    Ok(pool)
}

fn sqlite_file_path(url: &str) -> Option<PathBuf> {
    let value = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?
        .split('?')
        .next()?;
    if value.is_empty() || value == ":memory:" {
        return None;
    }
    Some(Path::new(value).to_owned())
}
