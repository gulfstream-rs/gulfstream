use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;

use super::Visibility;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub routes: RoutesConfig,
    pub api: ApiConfig,
    pub web: WebConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub streaming: StreamingConfig,
    pub registration: RegistrationConfig,
    pub browser_auth: BrowserAuthConfig,
    pub security: SecurityConfig,
    pub uploads: UploadConfig,
    pub remote_imports: RemoteImportConfig,
    pub jobs: JobConfig,
    pub transcoding: TranscodingConfig,
    pub analytics: AnalyticsConfig,
    pub player: PlayerConfig,
    pub observability: ObservabilityConfig,
    /// Abuse and bandwidth protection for playback routes.
    pub protection: ProtectionConfig,
    /// Third-party embedding of the player (`routes.embed`).
    pub embed: EmbedConfig,
    /// Country attribution for playback analytics.
    pub geolocation: GeolocationConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    pub public_base_url: String,
    pub max_request_body_bytes: usize,
    pub max_in_flight_requests: usize,
    pub shutdown_grace_seconds: u64,
    pub cors_allowed_origins: Vec<String>,
    pub cors_allow_credentials: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutesConfig {
    pub api: String,
    pub web: String,
    pub watch: String,
    pub stream: String,
    pub health_live: String,
    pub health_ready: String,
    pub openapi: String,
    pub embed: String,
    pub oembed: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    pub default_page_size: u32,
    pub maximum_page_size: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebConfig {
    pub enabled: bool,
    pub template_path: PathBuf,
    pub assets_directory: PathBuf,
    pub assets_route: String,
    pub site_name: String,
    pub tagline: String,
    pub repository_url: String,
    pub documentation_url: String,
    pub support_url: Option<String>,
    pub date_locale: String,
    pub time_zone: Option<String>,
    pub brand_color: String,
    pub dashboard_reporting_days: i64,
    pub dashboard_refresh_seconds: u64,
    pub jobs_refresh_seconds: u64,
    pub page_size_options: Vec<u32>,
    pub cache_control: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub busy_timeout_ms: u64,
    pub journal_mode: SqliteJournalMode,
    pub synchronous: SqliteSynchronous,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqliteJournalMode {
    Delete,
    Truncate,
    Persist,
    Memory,
    Wal,
    Off,
}

impl SqliteJournalMode {
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
            Self::Persist => "PERSIST",
            Self::Memory => "MEMORY",
            Self::Wal => "WAL",
            Self::Off => "OFF",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqliteSynchronous {
    Off,
    Normal,
    Full,
    Extra,
}

impl SqliteSynchronous {
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
            Self::Extra => "EXTRA",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    pub root: PathBuf,
    pub temporary_root: PathBuf,
    pub original_directory_name: String,
    pub stream_directory_name: String,
    pub trash_directory_name: String,
    pub preserve_deleted_media: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamingConfig {
    pub player_cache_control: String,
    pub token_redirect_cache_control: String,
    pub playlist_cache_control: String,
    pub segment_cache_control: String,
    pub source_cache_control: String,
    pub private_asset_cache_control: String,
    pub enable_range_requests: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationConfig {
    pub mode: RegistrationMode,
    pub admin_token: Option<String>,
    pub admin_header_name: String,
    pub default_storage_quota_bytes: u64,
    pub maximum_email_bytes: usize,
    pub maximum_display_name_bytes: usize,
    pub maximum_api_key_name_bytes: usize,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationMode {
    Open,
    AdminToken,
    Disabled,
}

impl RegistrationMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::AdminToken => "admin_token",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserAuthConfig {
    pub password_pepper: String,
    pub password_argon2_memory_kib: u32,
    pub password_argon2_iterations: u32,
    pub password_argon2_parallelism: u32,
    pub minimum_password_bytes: usize,
    pub maximum_password_bytes: usize,
    pub session_signing_key: String,
    pub session_ttl_seconds: i64,
    pub session_idle_timeout_seconds: i64,
    pub maximum_sessions_per_account: u32,
    pub session_cookie_name: String,
    pub session_cookie_path: String,
    pub session_cookie_domain: Option<String>,
    pub session_cookie_secure: bool,
    pub session_cookie_http_only: bool,
    pub session_cookie_same_site: CookieSameSite,
    pub csrf_header_name: String,
    pub maintenance_interval_seconds: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    pub api_key_prefix: String,
    pub api_key_pepper: String,
    pub api_key_argon2_memory_kib: u32,
    pub api_key_argon2_iterations: u32,
    pub api_key_argon2_parallelism: u32,
    pub playback_signing_key: String,
    pub playback_token_ttl_seconds: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadConfig {
    pub minimum_source_bytes: u64,
    pub maximum_source_bytes: u64,
    pub maximum_text_field_bytes: usize,
    pub maximum_filename_bytes: usize,
    pub default_filename: String,
    pub default_remote_filename: String,
    pub default_content_type: String,
    pub default_visibility: Visibility,
    pub accepted_content_types: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteImportConfig {
    pub enabled: bool,
    pub allow_private_networks: bool,
    pub maximum_source_bytes: u64,
    pub request_timeout_seconds: u64,
    pub maximum_redirects: usize,
    pub user_agent: String,
    pub use_system_proxy: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobConfig {
    pub worker_count: usize,
    pub poll_interval_ms: u64,
    pub lease_seconds: i64,
    pub lease_renewal_interval_seconds: u64,
    pub maximum_attempts: i64,
    pub retry_base_delay_seconds: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscodingConfig {
    pub enabled: bool,
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: PathBuf,
    pub command_timeout_seconds: u64,
    pub segment_duration_seconds: u32,
    pub segment_format: SegmentFormat,
    pub hls_version: u32,
    pub master_playlist_filename: String,
    pub variant_playlist_filename: String,
    pub fmp4_init_filename: String,
    pub fmp4_segment_filename_pattern: String,
    pub mpegts_segment_filename_pattern: String,
    pub hls_playlist_type: String,
    pub hls_flags: Vec<String>,
    pub scaling_flags: String,
    pub audio_sample_rate_hz: u32,
    pub audio_channels: u32,
    pub ffmpeg_threads: u32,
    pub retain_original: bool,
    pub allow_upscale: bool,
    pub extra_input_arguments: Vec<String>,
    pub extra_output_arguments: Vec<String>,
    pub profiles: Vec<TranscodingProfile>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentFormat {
    Fmp4,
    MpegTs,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscodingProfile {
    pub name: String,
    pub maximum_height: u32,
    pub video_codec: String,
    pub video_bitrate_kbps: u32,
    pub maximum_video_bitrate_kbps: u32,
    pub video_buffer_kbps: u32,
    pub audio_codec: String,
    pub audio_bitrate_kbps: u32,
    pub preset: String,
    pub profile: String,
    pub pixel_format: String,
    pub frame_rate: u32,
    pub hls_video_codec: String,
    pub hls_audio_codec: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticsConfig {
    pub enabled: bool,
    pub viewer_hash_key: String,
    pub visitor_cookie_name: String,
    pub visitor_cookie_path: String,
    pub playback_cookie_name: String,
    pub playback_session_cookie_name: String,
    pub playback_session_cookie_path: String,
    pub cookie_secure: bool,
    pub cookie_same_site: CookieSameSite,
    pub visitor_cookie_max_age_seconds: i64,
    pub heartbeat_interval_seconds: u64,
    pub maximum_heartbeat_delta_ms: i64,
    pub default_reporting_days: i64,
    pub maximum_reporting_days: i64,
    pub raw_event_retention_days: u64,
    pub playback_session_retention_days: u64,
    pub maintenance_interval_seconds: u64,
    pub record_byte_events: bool,
    /// Server-measured watch time after which a view counts as qualified
    /// (the unit used for monetization). Shorter videos qualify after
    /// `qualified_view_minimum_fraction` of their duration instead.
    pub qualified_view_seconds: u64,
    pub qualified_view_minimum_fraction: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

impl CookieSameSite {
    #[must_use]
    pub const fn as_cookie_value(self) -> &'static str {
        match self {
            Self::Strict => "Strict",
            Self::Lax => "Lax",
            Self::None => "None",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerConfig {
    pub template_path: PathBuf,
    pub hls_javascript_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityConfig {
    pub log_filter: String,
    pub json_logs: bool,
}

/// Playback abuse protection. Limits apply per client address; IPv6 clients
/// are grouped by their /64 prefix. A value of `0` disables a limit.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectionConfig {
    /// Header carrying the client address when Gulfstream runs behind a
    /// trusted reverse proxy or CDN (for example `x-forwarded-for` or
    /// `cf-connecting-ip`). Leave unset when clients connect directly:
    /// otherwise clients can spoof their address.
    pub client_ip_header: Option<String>,
    /// Sustained stream requests (playlists, segments, source ranges) per
    /// second and client, and the burst allowed above that rate.
    pub stream_requests_per_second: u32,
    pub stream_request_burst: u32,
    /// Watch/embed page loads (new playback sessions) per minute and client.
    pub sessions_per_minute: u32,
    /// Simultaneous open stream responses per client.
    pub max_concurrent_streams_per_client: u32,
    /// Per-response pacing of HLS segments after `segment_burst_bytes`.
    pub segment_bytes_per_second: u64,
    pub segment_burst_bytes: u64,
    /// Per-response pacing of the original file after `source_burst_bytes`.
    pub source_bytes_per_second: u64,
    pub source_burst_bytes: u64,
    /// Total egress cap shared by every stream response.
    pub global_bytes_per_second: u64,
    /// Reject browser media requests initiated by other sites
    /// (`Sec-Fetch-Site: cross-site`), which stops hotlinking of streams
    /// outside the official player and embed.
    pub block_cross_site_requests: bool,
    /// Serve the original upload to viewers when HLS renditions exist.
    /// Disable to stop full-quality source downloads; owners can still
    /// fetch it with their API credentials.
    pub expose_original_with_hls: bool,
    /// Upper bound of tracked clients before idle entries are pruned.
    pub maximum_tracked_clients: usize,
}

/// Recommended values, matching `config/gulfstream.example.toml`.
impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            client_ip_header: None,
            stream_requests_per_second: 30,
            stream_request_burst: 300,
            sessions_per_minute: 60,
            max_concurrent_streams_per_client: 16,
            segment_bytes_per_second: 0,
            segment_burst_bytes: 0,
            source_bytes_per_second: 2_500_000,
            source_burst_bytes: 16_000_000,
            global_bytes_per_second: 0,
            block_cross_site_requests: true,
            expose_original_with_hls: false,
            maximum_tracked_clients: 100_000,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedConfig {
    pub enabled: bool,
    /// Origins allowed to frame the embed player (`frame-ancestors`), for
    /// example `["https://blog.example.com"]`. `["*"]` allows any site.
    pub allowed_origins: Vec<String>,
    /// Default iframe size offered by oEmbed and the embed-code builder.
    pub default_width: u32,
    pub default_height: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GeolocationConfig {
    /// MaxMind DB (`.mmdb`) country database: DB-IP IP-to-Country Lite,
    /// MaxMind GeoLite2 Country or IPinfo Lite.
    pub database_path: Option<PathBuf>,
    /// Trusted CDN header with an ISO 3166-1 alpha-2 country code (for
    /// example `cf-ipcountry`). Takes precedence over the database.
    pub country_header: Option<String>,
}
