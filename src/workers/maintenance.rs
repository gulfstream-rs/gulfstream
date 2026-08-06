use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use tokio::{task::JoinHandle, time::MissedTickBehavior};

use crate::state::AppState;

pub fn spawn(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        run_browser_session_cleanup(&state).await;
        run_analytics_cleanup(&state).await;

        let mut auth_tick = tokio::time::interval(Duration::from_secs(
            state.config.browser_auth.maintenance_interval_seconds,
        ));
        let mut analytics_tick = tokio::time::interval(Duration::from_secs(
            state.config.analytics.maintenance_interval_seconds,
        ));
        auth_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        analytics_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        auth_tick.tick().await;
        analytics_tick.tick().await;

        loop {
            tokio::select! {
                () = state.cancellation.cancelled() => return,
                _ = auth_tick.tick() => run_browser_session_cleanup(&state).await,
                _ = analytics_tick.tick() => run_analytics_cleanup(&state).await,
            }
        }
    })
}

async fn run_browser_session_cleanup(state: &AppState) {
    if let Err(error) = state.auth.purge_expired_browser_sessions().await {
        tracing::error!(%error, "browser session cleanup failed");
    }
}

async fn run_analytics_cleanup(state: &AppState) {
    if let Err(error) = clean_analytics(state).await {
        tracing::error!(%error, "analytics retention cleanup failed");
    }
}

async fn clean_analytics(state: &AppState) -> anyhow::Result<()> {
    let now = Utc::now();
    let mut transaction = state.pool.begin().await?;
    if state.config.analytics.raw_event_retention_days > 0 {
        let days = i64::try_from(state.config.analytics.raw_event_retention_days)?;
        let cutoff = (now - ChronoDuration::days(days)).to_rfc3339();
        sqlx::query("DELETE FROM analytics_events WHERE occurred_at < ?")
            .bind(cutoff)
            .execute(&mut *transaction)
            .await?;
    }
    if state.config.analytics.playback_session_retention_days > 0 {
        let days = i64::try_from(state.config.analytics.playback_session_retention_days)?;
        let cutoff = (now - ChronoDuration::days(days)).to_rfc3339();
        sqlx::query("DELETE FROM playback_sessions WHERE last_event_at < ?")
            .bind(cutoff)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(())
}
