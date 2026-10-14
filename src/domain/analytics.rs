use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct AnalyticsTotals {
    pub views: i64,
    pub unique_viewers: i64,
    pub play_starts: i64,
    pub completed_views: i64,
    pub watch_time_ms: i64,
    pub bytes_served: i64,
    /// Views with enough server-measured watch time to be monetizable.
    pub qualified_views: i64,
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
    pub qualified_views: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CountryStats {
    /// ISO 3166-1 alpha-2 code, or `ZZ` when unknown.
    pub country_code: String,
    pub views: i64,
    pub qualified_views: i64,
    pub watch_time_ms: i64,
}
