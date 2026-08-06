use std::{collections::HashSet, path::Path};

use anyhow::{Context, bail};

use super::{Config, CookieSameSite, RegistrationMode, SegmentFormat};

impl Config {
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        validate_absolute_http_url(&self.server.public_base_url, "server.public_base_url")?;
        if self.server.max_request_body_bytes == 0
            || self.server.max_in_flight_requests == 0
            || self.server.shutdown_grace_seconds == 0
        {
            bail!("server limits and shutdown settings must be greater than zero");
        }
        if self.server.cors_allow_credentials
            && self
                .server
                .cors_allowed_origins
                .iter()
                .any(|origin| origin == "*")
        {
            bail!("credentialed CORS cannot use a wildcard origin");
        }
        for origin in &self.server.cors_allowed_origins {
            if origin != "*" {
                validate_absolute_http_url(origin, "server.cors_allowed_origins")?;
            }
        }

        let route_values = [
            (&self.routes.api, "routes.api"),
            (&self.routes.web, "routes.web"),
            (&self.routes.watch, "routes.watch"),
            (&self.routes.stream, "routes.stream"),
            (&self.routes.health_live, "routes.health_live"),
            (&self.routes.health_ready, "routes.health_ready"),
            (&self.routes.openapi, "routes.openapi"),
        ];
        let mut routes = HashSet::new();
        for (value, field) in route_values {
            validate_route(value, field)?;
            if !routes.insert(value) {
                bail!("configured routes must be unique");
            }
        }

        if self.api.default_page_size == 0
            || self.api.maximum_page_size == 0
            || self.api.default_page_size > self.api.maximum_page_size
        {
            bail!(
                "api page sizes must be positive and default_page_size cannot exceed maximum_page_size"
            );
        }

        if self.web.enabled {
            if self.web.site_name.trim().is_empty()
                || self.web.tagline.trim().is_empty()
                || self.web.date_locale.trim().is_empty()
                || self.web.cache_control.trim().is_empty()
                || self.web.dashboard_reporting_days <= 0
                || self.web.dashboard_refresh_seconds == 0
                || self.web.jobs_refresh_seconds == 0
                || self.web.page_size_options.is_empty()
            {
                bail!(
                    "web text, cache, refresh, pagination, and reporting settings cannot be empty or non-positive"
                );
            }
            validate_hex_color(&self.web.brand_color, "web.brand_color")?;
            let mut page_sizes = HashSet::new();
            for page_size in &self.web.page_size_options {
                if *page_size == 0 || *page_size > self.api.maximum_page_size {
                    bail!(
                        "web.page_size_options values must be positive and cannot exceed api.maximum_page_size"
                    );
                }
                if !page_sizes.insert(*page_size) {
                    bail!("web.page_size_options values must be unique");
                }
            }
            validate_relative_route(&self.web.assets_route, "web.assets_route")?;
            validate_absolute_http_url(&self.web.repository_url, "web.repository_url")?;
            validate_absolute_http_url(&self.web.documentation_url, "web.documentation_url")?;
            if let Some(url) = self.web.support_url.as_deref() {
                validate_absolute_http_url(url, "web.support_url")?;
            }
            if self.web.template_path.as_os_str().is_empty()
                || self.web.assets_directory.as_os_str().is_empty()
            {
                bail!("web template and asset paths cannot be empty");
            }
        }

        if self.database.max_connections == 0 || self.database.busy_timeout_ms == 0 {
            bail!("database connection and timeout settings must be greater than zero");
        }
        validate_component(
            &self.storage.original_directory_name,
            "storage.original_directory_name",
        )?;
        validate_component(
            &self.storage.stream_directory_name,
            "storage.stream_directory_name",
        )?;
        validate_component(
            &self.storage.trash_directory_name,
            "storage.trash_directory_name",
        )?;

        if matches!(self.registration.mode, RegistrationMode::AdminToken)
            && self
                .registration
                .admin_token
                .as_deref()
                .is_none_or(str::is_empty)
        {
            bail!("registration.admin_token is required when registration.mode is admin_token");
        }
        if self.registration.default_storage_quota_bytes == 0
            || self.registration.maximum_email_bytes == 0
            || self.registration.maximum_display_name_bytes == 0
            || self.registration.maximum_api_key_name_bytes == 0
        {
            bail!("registration quotas and field limits must be greater than zero");
        }
        axum::http::HeaderName::from_bytes(self.registration.admin_header_name.as_bytes())
            .context("registration.admin_header_name must be a valid HTTP header name")?;

        validate_secret(
            &self.browser_auth.password_pepper,
            "browser_auth.password_pepper",
        )?;
        validate_secret(
            &self.browser_auth.session_signing_key,
            "browser_auth.session_signing_key",
        )?;
        if self.browser_auth.password_argon2_memory_kib == 0
            || self.browser_auth.password_argon2_iterations == 0
            || self.browser_auth.password_argon2_parallelism == 0
            || self.browser_auth.minimum_password_bytes == 0
            || self.browser_auth.maximum_password_bytes < self.browser_auth.minimum_password_bytes
            || self.browser_auth.session_ttl_seconds <= 0
            || self.browser_auth.session_idle_timeout_seconds <= 0
            || self.browser_auth.session_idle_timeout_seconds
                > self.browser_auth.session_ttl_seconds
            || self.browser_auth.maximum_sessions_per_account == 0
            || self.browser_auth.maintenance_interval_seconds == 0
        {
            bail!("browser authentication limits, Argon2 parameters, and intervals are invalid");
        }
        validate_cookie_name(
            &self.browser_auth.session_cookie_name,
            "browser_auth.session_cookie_name",
        )?;
        validate_cookie_path(
            &self.browser_auth.session_cookie_path,
            "browser_auth.session_cookie_path",
        )?;
        if let Some(domain) = self.browser_auth.session_cookie_domain.as_deref() {
            validate_cookie_domain(domain, "browser_auth.session_cookie_domain")?;
        }
        axum::http::HeaderName::from_bytes(self.browser_auth.csrf_header_name.as_bytes())
            .context("browser_auth.csrf_header_name must be a valid HTTP header name")?;
        if matches!(
            self.browser_auth.session_cookie_same_site,
            CookieSameSite::None
        ) && !self.browser_auth.session_cookie_secure
        {
            bail!("browser_auth.session_cookie_secure must be true when SameSite is none");
        }

        if self.security.api_key_prefix.is_empty()
            || !self
                .security
                .api_key_prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
        {
            bail!("security.api_key_prefix must contain only ASCII letters and digits");
        }
        validate_secret(&self.security.api_key_pepper, "security.api_key_pepper")?;
        validate_secret(
            &self.security.playback_signing_key,
            "security.playback_signing_key",
        )?;
        if self.security.playback_token_ttl_seconds <= 0
            || self.security.api_key_argon2_memory_kib == 0
            || self.security.api_key_argon2_iterations == 0
            || self.security.api_key_argon2_parallelism == 0
        {
            bail!("security token and Argon2 settings must be greater than zero");
        }

        validate_secret(&self.analytics.viewer_hash_key, "analytics.viewer_hash_key")?;
        if self.analytics.default_reporting_days <= 0
            || self.analytics.maximum_reporting_days <= 0
            || self.analytics.default_reporting_days > self.analytics.maximum_reporting_days
            || self.analytics.heartbeat_interval_seconds == 0
            || self.analytics.maximum_heartbeat_delta_ms <= 0
            || self.analytics.visitor_cookie_max_age_seconds <= 0
        {
            bail!("analytics reporting and interval settings are invalid");
        }
        if (self.analytics.raw_event_retention_days > 0
            || self.analytics.playback_session_retention_days > 0)
            && self.analytics.maintenance_interval_seconds == 0
        {
            bail!(
                "analytics.maintenance_interval_seconds must be positive when retention is enabled"
            );
        }
        validate_cookie_name(
            &self.analytics.visitor_cookie_name,
            "analytics.visitor_cookie_name",
        )?;
        validate_cookie_path(
            &self.analytics.visitor_cookie_path,
            "analytics.visitor_cookie_path",
        )?;
        validate_cookie_name(
            &self.analytics.playback_cookie_name,
            "analytics.playback_cookie_name",
        )?;
        validate_cookie_name(
            &self.analytics.playback_session_cookie_name,
            "analytics.playback_session_cookie_name",
        )?;
        validate_cookie_path(
            &self.analytics.playback_session_cookie_path,
            "analytics.playback_session_cookie_path",
        )?;
        let cookie_names = [
            &self.browser_auth.session_cookie_name,
            &self.analytics.visitor_cookie_name,
            &self.analytics.playback_cookie_name,
            &self.analytics.playback_session_cookie_name,
        ];
        let unique_cookie_names = cookie_names.iter().copied().collect::<HashSet<_>>();
        if unique_cookie_names.len() != cookie_names.len() {
            bail!("all configured cookie names must be unique");
        }
        if matches!(self.analytics.cookie_same_site, CookieSameSite::None)
            && !self.analytics.cookie_secure
        {
            bail!("analytics.cookie_secure must be true when SameSite is none");
        }

        if self.uploads.minimum_source_bytes == 0
            || self.uploads.maximum_source_bytes == 0
            || self.uploads.maximum_text_field_bytes == 0
            || self.uploads.maximum_filename_bytes == 0
            || self.remote_imports.maximum_source_bytes == 0
            || self.remote_imports.request_timeout_seconds == 0
            || self.remote_imports.user_agent.trim().is_empty()
        {
            bail!("upload and remote import sizes, fields, and timeouts must be positive");
        }
        for (value, field) in [
            (&self.uploads.default_filename, "uploads.default_filename"),
            (
                &self.uploads.default_remote_filename,
                "uploads.default_remote_filename",
            ),
        ] {
            validate_component(value, field)?;
            if value.len() > self.uploads.maximum_filename_bytes {
                bail!("{field} exceeds uploads.maximum_filename_bytes");
            }
        }
        if self.uploads.default_content_type.trim().is_empty()
            || self.uploads.accepted_content_types.is_empty()
            || self
                .uploads
                .accepted_content_types
                .iter()
                .any(|value| value.trim().is_empty())
        {
            bail!("upload content type settings cannot be empty");
        }
        if !self
            .uploads
            .accepted_content_types
            .iter()
            .any(|value| value == &self.uploads.default_content_type)
        {
            bail!(
                "uploads.default_content_type must be included in uploads.accepted_content_types"
            );
        }
        if self.uploads.minimum_source_bytes > self.uploads.maximum_source_bytes
            || self.uploads.minimum_source_bytes > self.remote_imports.maximum_source_bytes
            || self.uploads.maximum_source_bytes > self.server.max_request_body_bytes as u64
        {
            bail!("configured upload size relationships are invalid");
        }

        for (value, field) in [
            (
                &self.streaming.player_cache_control,
                "streaming.player_cache_control",
            ),
            (
                &self.streaming.token_redirect_cache_control,
                "streaming.token_redirect_cache_control",
            ),
            (
                &self.streaming.playlist_cache_control,
                "streaming.playlist_cache_control",
            ),
            (
                &self.streaming.segment_cache_control,
                "streaming.segment_cache_control",
            ),
            (
                &self.streaming.source_cache_control,
                "streaming.source_cache_control",
            ),
            (
                &self.streaming.private_asset_cache_control,
                "streaming.private_asset_cache_control",
            ),
        ] {
            if value.trim().is_empty() {
                bail!("{field} cannot be empty");
            }
        }

        if self.jobs.worker_count == 0
            || self.jobs.poll_interval_ms == 0
            || self.jobs.lease_seconds <= 0
            || self.jobs.lease_renewal_interval_seconds == 0
            || self.jobs.maximum_attempts <= 0
            || self.jobs.retry_base_delay_seconds <= 0
        {
            bail!("job worker settings must be greater than zero");
        }
        match i64::try_from(self.jobs.lease_renewal_interval_seconds) {
            Ok(interval) if interval < self.jobs.lease_seconds => {}
            _ => bail!("jobs.lease_renewal_interval_seconds must be below jobs.lease_seconds"),
        }

        if self.transcoding.enabled {
            if self.transcoding.ffmpeg_path.as_os_str().is_empty()
                || self.transcoding.ffprobe_path.as_os_str().is_empty()
                || self.transcoding.segment_duration_seconds == 0
                || self.transcoding.command_timeout_seconds == 0
                || self.transcoding.hls_version == 0
            {
                bail!(
                    "transcoding command paths, durations, timeout, and HLS version must be configured"
                );
            }
            if matches!(self.transcoding.segment_format, SegmentFormat::Fmp4)
                && self.transcoding.hls_version < 7
            {
                bail!("transcoding.hls_version must be at least 7 for fMP4 segments");
            }
            validate_component(
                &self.transcoding.master_playlist_filename,
                "transcoding.master_playlist_filename",
            )?;
            validate_component(
                &self.transcoding.variant_playlist_filename,
                "transcoding.variant_playlist_filename",
            )?;
            validate_component(
                &self.transcoding.fmp4_init_filename,
                "transcoding.fmp4_init_filename",
            )?;
            validate_segment_pattern(
                &self.transcoding.fmp4_segment_filename_pattern,
                "transcoding.fmp4_segment_filename_pattern",
            )?;
            validate_segment_pattern(
                &self.transcoding.mpegts_segment_filename_pattern,
                "transcoding.mpegts_segment_filename_pattern",
            )?;
            if self.transcoding.audio_sample_rate_hz == 0
                || self.transcoding.audio_channels == 0
                || self.transcoding.hls_playlist_type.trim().is_empty()
                || self.transcoding.scaling_flags.trim().is_empty()
                || self.transcoding.profiles.is_empty()
            {
                bail!("transcoding audio, HLS, scaling, and profile settings must be configured");
            }
            let mut names = HashSet::new();
            for profile in &self.transcoding.profiles {
                validate_component(&profile.name, "transcoding.profiles.name")?;
                if profile.maximum_height == 0
                    || profile.video_bitrate_kbps == 0
                    || profile.maximum_video_bitrate_kbps == 0
                    || profile.video_buffer_kbps == 0
                    || profile.audio_bitrate_kbps == 0
                    || profile.frame_rate == 0
                {
                    bail!(
                        "transcoding profile dimensions, frame rates, and bitrates must be positive"
                    );
                }
                if profile.maximum_video_bitrate_kbps < profile.video_bitrate_kbps {
                    bail!("transcoding profile maximum bitrate cannot be below its target bitrate");
                }
                for value in [
                    &profile.video_codec,
                    &profile.audio_codec,
                    &profile.preset,
                    &profile.profile,
                    &profile.pixel_format,
                    &profile.hls_video_codec,
                    &profile.hls_audio_codec,
                ] {
                    if value.trim().is_empty() {
                        bail!("transcoding profile string values cannot be empty");
                    }
                }
                if !names.insert(profile.name.as_str()) {
                    bail!("transcoding profile names must be unique");
                }
            }
        }

        if let Some(url) = self.player.hls_javascript_url.as_deref() {
            validate_absolute_http_url(url, "player.hls_javascript_url")?;
        }
        Ok(())
    }
}

fn validate_hex_color(value: &str, field: &str) -> anyhow::Result<()> {
    let digits = value.strip_prefix('#').unwrap_or_default();
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{field} must be a six-digit hexadecimal color such as #2563eb");
    }
    Ok(())
}

fn validate_absolute_http_url(value: &str, field: &str) -> anyhow::Result<()> {
    let parsed =
        reqwest::Url::parse(value).with_context(|| format!("{field} must be an absolute URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        bail!("{field} must use HTTP or HTTPS and include a host");
    }
    Ok(())
}

fn validate_secret(value: &str, field: &str) -> anyhow::Result<()> {
    if value.len() < 32 {
        bail!("{field} must contain at least 32 bytes");
    }
    Ok(())
}

fn validate_route(value: &str, field: &str) -> anyhow::Result<()> {
    if value == "/"
        || !value.starts_with('/')
        || value.ends_with('/')
        || value.contains('?')
        || value.contains('#')
    {
        bail!("{field} must start with '/', must not be '/', and must not end with '/'");
    }
    if value
        .split('/')
        .skip(1)
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        bail!("{field} contains an invalid path component");
    }
    Ok(())
}

fn validate_relative_route(value: &str, field: &str) -> anyhow::Result<()> {
    validate_route(value, field)
}

fn validate_component(value: &str, field: &str) -> anyhow::Result<()> {
    let candidate = Path::new(value);
    let valid = candidate.components().count() == 1
        && candidate.file_name().and_then(|part| part.to_str()) == Some(value)
        && value != "."
        && value != "..";
    if !valid {
        bail!("{field} must be a single safe path component");
    }
    Ok(())
}

fn validate_cookie_name(value: &str, field: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || !value.bytes().all(
            |byte| matches!(byte, 0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E),
        )
    {
        bail!("{field} is not a valid cookie name");
    }
    Ok(())
}

fn validate_cookie_path(value: &str, field: &str) -> anyhow::Result<()> {
    if !value.starts_with('/')
        || value
            .bytes()
            .any(|byte| byte == b';' || byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        bail!("{field} is not a valid cookie path");
    }
    Ok(())
}

fn validate_cookie_domain(value: &str, field: &str) -> anyhow::Result<()> {
    let candidate = value.strip_prefix('.').unwrap_or(value);
    if candidate.is_empty()
        || candidate.starts_with('-')
        || candidate.ends_with('-')
        || !candidate
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
    {
        bail!("{field} is not a valid cookie domain");
    }
    Ok(())
}

fn validate_segment_pattern(value: &str, field: &str) -> anyhow::Result<()> {
    validate_component(value, field)?;
    if !value.contains("%d") && !value.contains("%0") {
        bail!("{field} must contain an FFmpeg numeric placeholder such as %06d");
    }
    Ok(())
}
