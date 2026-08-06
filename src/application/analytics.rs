use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::{
    domain::analytics::{AnalyticsPoint, AnalyticsTotals},
    error::AppError,
    state::AppState,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackEventKind {
    Play,
    Pause,
    Seek,
    Heartbeat,
    Ended,
}

impl PlaybackEventKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Seek => "seek",
            Self::Heartbeat => "heartbeat",
            Self::Ended => "ended",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReportingRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub media_id: Option<String>,
}

pub async fn start_session(
    state: &AppState,
    account_id: &str,
    media_id: &str,
    visitor_identifier: &str,
) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();
    if !state.config.analytics.enabled {
        return Ok(session_id);
    }
    let viewer_hash = hash_viewer(state, visitor_identifier)?;
    let timestamp = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO playback_sessions \
         (id, media_id, account_id, viewer_hash, started_at, last_event_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(media_id)
    .bind(account_id)
    .bind(viewer_hash)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&state.pool)
    .await?;
    Ok(session_id)
}

pub async fn record_playback_event(
    state: &AppState,
    session_id: &str,
    kind: PlaybackEventKind,
    position_ms: i64,
    watched_delta_ms: i64,
) -> Result<(), AppError> {
    if !state.config.analytics.enabled {
        return Ok(());
    }
    if position_ms < 0 || watched_delta_ms < 0 {
        return Err(AppError::BadRequest(
            "playback positions and deltas cannot be negative".to_owned(),
        ));
    }
    let timestamp = Utc::now();
    let timestamp_text = timestamp.to_rfc3339();
    let day = timestamp.date_naive().to_string();
    let mut transaction = state.pool.begin().await?;
    let session_lock = sqlx::query(
        "UPDATE playback_sessions SET last_position_ms = last_position_ms WHERE id = ?",
    )
    .bind(session_id)
    .execute(&mut *transaction)
    .await?;
    if session_lock.rows_affected() == 0 {
        return Err(AppError::NotFound("playback session not found".to_owned()));
    }
    let first_play = if matches!(kind, PlaybackEventKind::Play) {
        sqlx::query(
            "UPDATE playback_sessions SET play_started_at = ? \
             WHERE id = ? AND play_started_at IS NULL",
        )
        .bind(&timestamp_text)
        .bind(session_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1
    } else {
        false
    };
    let first_completion = if matches!(kind, PlaybackEventKind::Ended) {
        sqlx::query(
            "UPDATE playback_sessions SET completed_at = ? \
             WHERE id = ? AND play_started_at IS NOT NULL AND completed_at IS NULL",
        )
        .bind(&timestamp_text)
        .bind(session_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1
    } else {
        false
    };
    let row = sqlx::query(
        "SELECT account_id, media_id, viewer_hash, last_event_at, last_position_ms \
         FROM playback_sessions WHERE id = ?",
    )
    .bind(session_id)
    .fetch_one(&mut *transaction)
    .await?;
    let account_id: String = row.get("account_id");
    let media_id: String = row.get("media_id");
    let viewer_hash: String = row.get("viewer_hash");
    let previous_event =
        chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("last_event_at"))
            .map_err(AppError::internal)?
            .with_timezone(&Utc);
    let elapsed_ms = timestamp
        .signed_duration_since(previous_event)
        .num_milliseconds()
        .max(0);
    let previous_position: i64 = row.get("last_position_ms");
    let progressed_ms = position_ms.saturating_sub(previous_position).max(0);
    let delta = if matches!(
        kind,
        PlaybackEventKind::Heartbeat | PlaybackEventKind::Pause | PlaybackEventKind::Ended
    ) {
        watched_delta_ms
            .min(state.config.analytics.maximum_heartbeat_delta_ms)
            .min(elapsed_ms)
            .min(progressed_ms)
    } else {
        0
    };
    sqlx::query(
        "UPDATE playback_sessions SET last_event_at = ?, last_position_ms = ?, \
                watched_ms = watched_ms + ? WHERE id = ?",
    )
    .bind(&timestamp_text)
    .bind(position_ms)
    .bind(delta)
    .bind(session_id)
    .execute(&mut *transaction)
    .await?;
    if first_play {
        sqlx::query(
            "INSERT INTO analytics_events \
             (account_id, media_id, session_id, kind, occurred_at) \
             VALUES (?, ?, ?, 'view', ?)",
        )
        .bind(&account_id)
        .bind(&media_id)
        .bind(session_id)
        .bind(&timestamp_text)
        .execute(&mut *transaction)
        .await?;
    }
    sqlx::query(
        "INSERT INTO analytics_events \
         (account_id, media_id, session_id, kind, position_ms, watched_delta_ms, occurred_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&account_id)
    .bind(&media_id)
    .bind(session_id)
    .bind(kind.as_str())
    .bind(position_ms)
    .bind(delta)
    .bind(&timestamp_text)
    .execute(&mut *transaction)
    .await?;
    ensure_daily_row(&mut transaction, &day, &account_id, &media_id).await?;
    sqlx::query(
        "UPDATE daily_stats SET \
            views = views + ?, \
            play_starts = play_starts + ?, \
            completed_views = completed_views + ?, \
            watch_time_ms = watch_time_ms + ? \
         WHERE day = ? AND media_id = ?",
    )
    .bind(bool_to_i64(first_play))
    .bind(bool_to_i64(first_play))
    .bind(bool_to_i64(first_completion))
    .bind(delta)
    .bind(&day)
    .bind(&media_id)
    .execute(&mut *transaction)
    .await?;
    if first_play {
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO daily_viewers (day, account_id, media_id, viewer_hash) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&day)
        .bind(&account_id)
        .bind(&media_id)
        .bind(viewer_hash)
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 1 {
            sqlx::query(
                "UPDATE daily_stats SET unique_viewers = unique_viewers + 1 \
                 WHERE day = ? AND media_id = ?",
            )
            .bind(&day)
            .bind(&media_id)
            .execute(&mut *transaction)
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn record_bytes_served(
    state: &AppState,
    account_id: &str,
    media_id: &str,
    session_id: Option<&str>,
    bytes: u64,
) -> Result<(), AppError> {
    if !state.config.analytics.enabled || bytes == 0 {
        return Ok(());
    }
    let bytes = i64::try_from(bytes)
        .map_err(|_| AppError::internal(anyhow::anyhow!("served byte count overflow")))?;
    let timestamp = Utc::now();
    let timestamp_text = timestamp.to_rfc3339();
    let day = timestamp.date_naive().to_string();
    let mut transaction = state.pool.begin().await?;
    let valid_session_id = if let Some(session_id) = session_id {
        let updated = sqlx::query(
            "UPDATE playback_sessions SET bytes_served = bytes_served + ? \
             WHERE id = ? AND media_id = ? AND account_id = ?",
        )
        .bind(bytes)
        .bind(session_id)
        .bind(media_id)
        .bind(account_id)
        .execute(&mut *transaction)
        .await?;
        (updated.rows_affected() == 1).then_some(session_id)
    } else {
        None
    };
    if state.config.analytics.record_byte_events {
        sqlx::query(
            "INSERT INTO analytics_events \
             (account_id, media_id, session_id, kind, bytes, occurred_at) \
             VALUES (?, ?, ?, 'bytes_served', ?, ?)",
        )
        .bind(account_id)
        .bind(media_id)
        .bind(valid_session_id)
        .bind(bytes)
        .bind(&timestamp_text)
        .execute(&mut *transaction)
        .await?;
    }
    ensure_daily_row(&mut transaction, &day, account_id, media_id).await?;
    sqlx::query(
        "UPDATE daily_stats SET bytes_served = bytes_served + ? WHERE day = ? AND media_id = ?",
    )
    .bind(bytes)
    .bind(&day)
    .bind(media_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub fn reporting_range(
    state: &AppState,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    media_id: Option<String>,
) -> Result<ReportingRange, AppError> {
    let to = to.unwrap_or_else(|| Utc::now().date_naive());
    let from = from
        .unwrap_or_else(|| to - Duration::days(state.config.analytics.default_reporting_days - 1));
    if from > to {
        return Err(AppError::BadRequest("from must not be after to".to_owned()));
    }
    let days = (to - from).num_days() + 1;
    if days > state.config.analytics.maximum_reporting_days {
        return Err(AppError::BadRequest(format!(
            "reporting range cannot exceed {} days",
            state.config.analytics.maximum_reporting_days
        )));
    }
    Ok(ReportingRange { from, to, media_id })
}

pub async fn totals(
    state: &AppState,
    account_id: &str,
    range: &ReportingRange,
) -> Result<AnalyticsTotals, AppError> {
    let row = sqlx::query(
        "SELECT \
            COALESCE(SUM(views), 0) AS views, \
            (SELECT COUNT(DISTINCT viewer_hash) FROM daily_viewers \
             WHERE account_id = ? AND day BETWEEN ? AND ? \
               AND (? IS NULL OR media_id = ?)) AS unique_viewers, \
            COALESCE(SUM(play_starts), 0) AS play_starts, \
            COALESCE(SUM(completed_views), 0) AS completed_views, \
            COALESCE(SUM(watch_time_ms), 0) AS watch_time_ms, \
            COALESCE(SUM(bytes_served), 0) AS bytes_served \
         FROM daily_stats \
         WHERE account_id = ? AND day BETWEEN ? AND ? \
           AND (? IS NULL OR media_id = ?)",
    )
    .bind(account_id)
    .bind(range.from.to_string())
    .bind(range.to.to_string())
    .bind(range.media_id.as_deref())
    .bind(range.media_id.as_deref())
    .bind(account_id)
    .bind(range.from.to_string())
    .bind(range.to.to_string())
    .bind(range.media_id.as_deref())
    .bind(range.media_id.as_deref())
    .fetch_one(&state.pool)
    .await?;
    Ok(AnalyticsTotals {
        views: row.get("views"),
        unique_viewers: row.get("unique_viewers"),
        play_starts: row.get("play_starts"),
        completed_views: row.get("completed_views"),
        watch_time_ms: row.get("watch_time_ms"),
        bytes_served: row.get("bytes_served"),
    })
}

pub async fn time_series(
    state: &AppState,
    account_id: &str,
    range: &ReportingRange,
) -> Result<Vec<AnalyticsPoint>, AppError> {
    let rows = sqlx::query(
        "SELECT stats.day, \
            SUM(stats.views) AS views, \
            (SELECT COUNT(DISTINCT viewers.viewer_hash) FROM daily_viewers AS viewers \
             WHERE viewers.account_id = ? AND viewers.day = stats.day \
               AND (? IS NULL OR viewers.media_id = ?)) AS unique_viewers, \
            SUM(stats.play_starts) AS play_starts, \
            SUM(stats.completed_views) AS completed_views, \
            SUM(stats.watch_time_ms) AS watch_time_ms, \
            SUM(stats.bytes_served) AS bytes_served \
         FROM daily_stats AS stats \
         WHERE stats.account_id = ? AND stats.day BETWEEN ? AND ? \
           AND (? IS NULL OR stats.media_id = ?) \
         GROUP BY stats.day ORDER BY stats.day ASC",
    )
    .bind(account_id)
    .bind(range.media_id.as_deref())
    .bind(range.media_id.as_deref())
    .bind(account_id)
    .bind(range.from.to_string())
    .bind(range.to.to_string())
    .bind(range.media_id.as_deref())
    .bind(range.media_id.as_deref())
    .fetch_all(&state.pool)
    .await?;
    let points = rows
        .into_iter()
        .map(|row| AnalyticsPoint {
            day: row.get("day"),
            views: row.get("views"),
            unique_viewers: row.get("unique_viewers"),
            play_starts: row.get("play_starts"),
            completed_views: row.get("completed_views"),
            watch_time_ms: row.get("watch_time_ms"),
            bytes_served: row.get("bytes_served"),
        })
        .collect();
    Ok(fill_missing_points(range, points))
}

fn fill_missing_points(range: &ReportingRange, points: Vec<AnalyticsPoint>) -> Vec<AnalyticsPoint> {
    let mut points = points
        .into_iter()
        .map(|point| (point.day.clone(), point))
        .collect::<BTreeMap<_, _>>();
    let day_count = (range.to - range.from).num_days();
    (0..=day_count)
        .map(|offset| (range.from + Duration::days(offset)).to_string())
        .map(|day| {
            points.remove(&day).unwrap_or(AnalyticsPoint {
                day,
                views: 0,
                unique_viewers: 0,
                play_starts: 0,
                completed_views: 0,
                watch_time_ms: 0,
                bytes_served: 0,
            })
        })
        .collect()
}

fn hash_viewer(state: &AppState, identifier: &str) -> Result<String, AppError> {
    let mut mac =
        <HmacSha256 as KeyInit>::new_from_slice(state.config.analytics.viewer_hash_key.as_bytes())
            .map_err(AppError::internal)?;
    mac.update(identifier.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

async fn ensure_daily_row(
    transaction: &mut Transaction<'_, Sqlite>,
    day: &str,
    account_id: &str,
    media_id: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT OR IGNORE INTO daily_stats (day, account_id, media_id) VALUES (?, ?, ?)")
        .bind(day)
        .bind(account_id)
        .bind(media_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

const fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_series_fills_days_without_activity() {
        let range = ReportingRange {
            from: NaiveDate::from_ymd_opt(2026, 8, 4).expect("valid date"),
            to: NaiveDate::from_ymd_opt(2026, 8, 6).expect("valid date"),
            media_id: None,
        };
        let points = fill_missing_points(
            &range,
            vec![AnalyticsPoint {
                day: "2026-08-05".to_owned(),
                views: 2,
                unique_viewers: 1,
                play_starts: 2,
                completed_views: 1,
                watch_time_ms: 1000,
                bytes_served: 2000,
            }],
        );

        assert_eq!(points.len(), 3);
        assert_eq!(points[0].day, "2026-08-04");
        assert_eq!(points[0].views, 0);
        assert_eq!(points[1].day, "2026-08-05");
        assert_eq!(points[1].views, 2);
        assert_eq!(points[2].day, "2026-08-06");
        assert_eq!(points[2].views, 0);
    }
}
