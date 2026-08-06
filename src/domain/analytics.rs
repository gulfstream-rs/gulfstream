use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct AnalyticsTotals {
    pub views: i64,
    pub unique_viewers: i64,
    pub play_starts: i64,
    pub completed_views: i64,
    pub watch_time_ms: i64,
    pub bytes_served: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalyticsPoint {
    pub day: String,
    pub views: i64,
    pub unique_viewers: i64,
    pub play_starts: i64,
    pub completed_views: i64,
    pub watch_time_ms: i64,
    pub bytes_served: i64,
}
