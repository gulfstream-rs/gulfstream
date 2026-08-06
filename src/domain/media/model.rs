use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MediaVariant {
    pub name: String,
    pub width: i64,
    pub height: i64,
    pub bandwidth_bps: i64,
    pub codecs: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Media {
    pub id: String,
    pub source_filename: String,
    pub source_mime_type: Option<String>,
    pub source_size_bytes: i64,
    pub storage_bytes: i64,
    pub sha256: Option<String>,
    pub title: String,
    pub description: String,
    pub visibility: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
    pub variants: Vec<MediaVariant>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MediaJob {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub attempts: i64,
    pub maximum_attempts: i64,
    pub run_after: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
