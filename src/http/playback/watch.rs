use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_ready, original_exposed};
use crate::{
    application::{
        analytics::{self, PlaybackSurface, SessionContext},
        media::{self, MediaRecord},
    },
    cookie::{self, SetCookie},
    error::AppError,
    http::{
        client::ClientAddress,
        template::{html_escape, script_json, substitute},
    },
    state::AppState,
};

/// Query string of the watch page.
///
/// Unknown parameters are tolerated on purpose: shared links routinely carry
/// tracking parameters (for example `utm_*`) and must not fail with 400. Only
/// `token` and the start offset (`t` or `start`) are read.
#[derive(Debug, Deserialize)]
pub struct WatchQuery {
    pub token: Option<String>,
    pub t: Option<String>,
    pub start: Option<String>,
}

/// Query string of the embed player. Values are parsed leniently so that a
/// malformed option never breaks an embed on a third-party page.
#[derive(Debug, Default, Deserialize)]
pub struct EmbedQuery {
    pub autoplay: Option<String>,
    pub muted: Option<String>,
    #[serde(rename = "loop")]
    pub loop_playback: Option<String>,
    pub start: Option<String>,
    pub t: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaybackEventRequest {
    pub kind: analytics::PlaybackEventKind,
    pub position_ms: i64,
    pub watched_delta_ms: i64,
}

#[derive(Debug, Default, Deserialize)]
pub struct PlaybackEventQuery {
    pub signature: Option<String>,
}

/// Client-side player options (`{{PLAYER_OPTIONS_JSON}}`).
#[derive(Debug, Serialize)]
struct PlayerOptions {
    surface: &'static str,
    autoplay: bool,
    muted: bool,
    #[serde(rename = "loop")]
    loop_playback: bool,
    start_seconds: f64,
    /// Canonical public watch link; `null` for private media.
    watch_url: Option<String>,
    /// Public embed URL; `null` when embedding is unavailable.
    embed_url: Option<String>,
    title: String,
    site_name: String,
}

#[derive(Debug, Default)]
struct EmbedOptions {
    autoplay: bool,
    muted: bool,
    loop_playback: bool,
    start_seconds: f64,
}

impl EmbedOptions {
    fn from_query(query: &EmbedQuery) -> Self {
        let autoplay = flag(query.autoplay.as_deref());
        Self {
            autoplay,
            // Browsers only allow unmuted autoplay after user engagement, so
            // autoplaying embeds start muted unless `muted=0` is explicit.
            muted: query
                .muted
                .as_deref()
                .map_or(autoplay, |value| flag(Some(value))),
            loop_playback: flag(query.loop_playback.as_deref()),
            start_seconds: query
                .start
                .as_deref()
                .or(query.t.as_deref())
                .and_then(parse_timestamp)
                .unwrap_or(0.0),
        }
    }
}

pub async fn watch(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    Query(query): Query<WatchQuery>,
    client: ClientAddress,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let record = media::get_record(&state, &media_id).await?;
    ensure_ready(&record)?;
    if record.visibility == "private"
        && let Some(token) = query.token.as_deref()
    {
        state
            .auth
            .verify_playback_token(token, &record.id, &record.account_id)?;
        let mut response = StatusCode::SEE_OTHER.into_response();
        response.headers_mut().insert(
            header::LOCATION,
            format!("{}/{}", state.config.routes.watch, record.id)
                .parse::<HeaderValue>()
                .map_err(AppError::internal)?,
        );
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            state
                .config
                .streaming
                .token_redirect_cache_control
                .parse::<HeaderValue>()
                .map_err(AppError::internal)?,
        );
        append_cookie(
            response.headers_mut(),
            &state,
            &state.config.analytics.playback_cookie_name,
            token,
            &playback_redemption_cookie_path(&state, &record.id),
            Some(state.config.security.playback_token_ttl_seconds),
        )?;
        return Ok(response);
    }
    let playback_token = authorize_watch(&state, &record, &headers).await?;
    let options = EmbedOptions {
        start_seconds: query
            .start
            .as_deref()
            .or(query.t.as_deref())
            .and_then(parse_timestamp)
            .unwrap_or(0.0),
        ..EmbedOptions::default()
    };
    let mut response = render_player(
        &state,
        &record,
        &headers,
        client,
        PlaybackSurface::Watch,
        &options,
    )
    .await?;
    if let Some(token) = playback_token {
        append_cookie(
            response.headers_mut(),
            &state,
            &state.config.analytics.playback_cookie_name,
            &token,
            &playback_asset_cookie_path(&state, &record.id),
            Some(state.config.security.playback_token_ttl_seconds),
        )?;
        // The redemption cookie on the watch path is intentionally kept until
        // it expires: it carries the same token as the asset cookie, and
        // keeping it lets viewers reload the player page while the signed
        // authorization is still valid.
    }
    // The watch page itself may only be framed by this site (the dashboard
    // preview); third-party sites use the dedicated embed route.
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("frame-ancestors 'self'"),
    );
    Ok(response)
}

/// Chromeless player for `<iframe>` embedding on other sites.
///
/// Only public and unlisted media can be embedded: private media relies on
/// cookies scoped to this site, which browsers withhold in third-party frames.
pub async fn embed(
    State(state): State<AppState>,
    Path(media_id): Path<String>,
    Query(query): Query<EmbedQuery>,
    client: ClientAddress,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if !state.config.embed.enabled {
        return Err(AppError::NotFound("embedding is disabled".to_owned()));
    }
    let record = media::get_record(&state, &media_id).await?;
    if record.visibility == "private" {
        return Err(AppError::Forbidden);
    }
    ensure_ready(&record)?;
    let options = EmbedOptions::from_query(&query);
    let mut response = render_player(
        &state,
        &record,
        &headers,
        client,
        PlaybackSurface::Embed,
        &options,
    )
    .await?;
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        frame_ancestors(&state.config.embed.allowed_origins)
            .parse::<HeaderValue>()
            .map_err(AppError::internal)?,
    );
    Ok(response)
}

async fn render_player(
    state: &AppState,
    record: &MediaRecord,
    headers: &HeaderMap,
    client: ClientAddress,
    surface: PlaybackSurface,
    options: &EmbedOptions,
) -> Result<Response, AppError> {
    state.protection.admit_session(client.key())?;
    let existing_visitor = cookie::read(headers, &state.config.analytics.visitor_cookie_name);
    let visitor_identifier = existing_visitor.clone().unwrap_or_else(|| {
        // Third-party frames frequently drop cookies; a daily-rotating
        // address/agent fingerprint keeps unique-viewer counts meaningful
        // without storing either value (it is hashed with a secret key).
        match (surface, client.0) {
            (PlaybackSurface::Embed, Some(ip)) => format!(
                "embed:{}:{ip}:{}",
                Utc::now().date_naive(),
                headers
                    .get(header::USER_AGENT)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
            ),
            _ => Uuid::new_v4().to_string(),
        }
    });
    let country_code = state.geolocation.country(headers, client.0);
    let session_id = analytics::start_session(
        state,
        &record.account_id,
        &record.id,
        &SessionContext {
            visitor_identifier: &visitor_identifier,
            country_code: &country_code,
            surface,
        },
    )
    .await?;
    let variant_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM media_variants WHERE media_id = ?")
            .bind(&record.id)
            .fetch_one(&state.pool)
            .await?;
    let (source_mode, source_url) = if variant_count > 0 {
        (
            "hls",
            format!("{}/{}/master.m3u8", state.config.routes.stream, record.id),
        )
    } else if record.original_object_key.is_some() {
        (
            "file",
            format!("{}/{}/source", state.config.routes.stream, record.id),
        )
    } else {
        return Err(AppError::MediaNotReady);
    };
    let hls_script = if source_mode == "hls" {
        state
            .config
            .player
            .hls_javascript_url
            .as_deref()
            .map(|url| format!("<script src=\"{}\"></script>", html_escape(url)))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let signature = state.auth.sign_playback_session(&session_id)?;
    let event_url = format!(
        "{}/playback/{session_id}/events?signature={signature}",
        state.config.routes.api
    );
    let heartbeat_milliseconds = state
        .config
        .analytics
        .heartbeat_interval_seconds
        .saturating_mul(1000);
    let fallback_url_json = if source_mode == "hls"
        && record.original_object_key.is_some()
        && original_exposed(state, true)
    {
        script_json(&format!(
            "{}/{}/source",
            state.config.routes.stream, record.id
        ))?
    } else {
        "null".to_owned()
    };
    let public = record.visibility != "private";
    let base = state.config.server.public_base_url.trim_end_matches('/');
    let watch_url = public.then(|| format!("{base}{}/{}", state.config.routes.watch, record.id));
    let embed_url = (public && state.config.embed.enabled)
        .then(|| format!("{base}{}/{}", state.config.routes.embed, record.id));
    let player_options = script_json(&PlayerOptions {
        surface: surface.as_str(),
        autoplay: options.autoplay,
        muted: options.muted,
        loop_playback: options.loop_playback,
        start_seconds: options.start_seconds,
        watch_url: watch_url.clone(),
        embed_url,
        title: record.title.clone(),
        site_name: state.config.web.site_name.clone(),
    })?;
    let mut head_links = String::new();
    if let Some(url) = &watch_url {
        head_links.push_str(&format!(
            "<link rel=\"canonical\" href=\"{}\">",
            html_escape(url)
        ));
        if state.config.embed.enabled {
            head_links.push_str(&format!(
                "<link rel=\"alternate\" type=\"application/json+oembed\" href=\"{}\" title=\"{}\">",
                html_escape(&format!(
                    "{base}{}?format=json&url={}",
                    state.config.routes.oembed,
                    percent_encode(url)
                )),
                html_escape(&record.title)
            ));
        }
    }
    if surface == PlaybackSurface::Embed || record.visibility != "public" {
        // Embeds duplicate the watch page; unlisted/private media must not
        // be discoverable through search engines.
        head_links.push_str("<meta name=\"robots\" content=\"noindex\">");
    }
    let stream_url_json = script_json(&source_url)?;
    let event_url_json = script_json(&event_url)?;
    let source_mode_json = script_json(source_mode)?;
    let heartbeat = heartbeat_milliseconds.to_string();
    let site_name = html_escape(&state.config.web.site_name);
    let title = html_escape(&record.title);
    let description = html_escape(&record.description);
    // Single-pass substitution: user-controlled title/description text that
    // contains `{{...}}` is never rescanned for other markers.
    let html = substitute(
        &state.player_template,
        &[
            ("{{TITLE}}", &title),
            ("{{DESCRIPTION}}", &description),
            ("{{SITE_NAME}}", &site_name),
            ("{{HLS_SCRIPT}}", &hls_script),
            ("{{STREAM_URL_JSON}}", &stream_url_json),
            ("{{EVENT_URL_JSON}}", &event_url_json),
            ("{{SOURCE_MODE_JSON}}", &source_mode_json),
            ("{{FALLBACK_URL_JSON}}", &fallback_url_json),
            ("{{HEARTBEAT_MILLISECONDS}}", &heartbeat),
            ("{{PLAYER_OPTIONS_JSON}}", &player_options),
            ("{{HEAD_LINKS}}", &head_links),
        ],
    );
    let mut response = Html(html).into_response();
    if existing_visitor.is_none() && surface == PlaybackSurface::Watch {
        append_cookie(
            response.headers_mut(),
            state,
            &state.config.analytics.visitor_cookie_name,
            &visitor_identifier,
            &state.config.analytics.visitor_cookie_path,
            Some(state.config.analytics.visitor_cookie_max_age_seconds),
        )?;
    }
    append_cookie(
        response.headers_mut(),
        state,
        &state.config.analytics.playback_session_cookie_name,
        &session_id,
        &state.config.analytics.playback_session_cookie_path,
        Some(state.config.security.playback_token_ttl_seconds),
    )?;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        state
            .config
            .streaming
            .player_cache_control
            .parse::<HeaderValue>()
            .map_err(AppError::internal)?,
    );
    Ok(response)
}

pub async fn playback_event(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<PlaybackEventQuery>,
    headers: HeaderMap,
    Json(request): Json<PlaybackEventRequest>,
) -> Result<StatusCode, AppError> {
    // Either the first-party session cookie or the signature issued with the
    // player page (used by third-party embeds without cookies).
    let cookie_matches = cookie::read(
        &headers,
        &state.config.analytics.playback_session_cookie_name,
    )
    .is_some_and(|cookie_session| cookie_session == session_id);
    let signature_matches = query
        .signature
        .as_deref()
        .is_some_and(|signature| state.auth.verify_playback_session(&session_id, signature));
    if !cookie_matches && !signature_matches {
        return Err(AppError::Unauthorized);
    }
    analytics::record_playback_event(
        &state,
        &session_id,
        request.kind,
        request.position_ms,
        request.watched_delta_ms,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn authorize_watch(
    state: &AppState,
    record: &MediaRecord,
    headers: &HeaderMap,
) -> Result<Option<String>, AppError> {
    if record.visibility != "private" {
        return Ok(None);
    }
    if headers.contains_key(header::AUTHORIZATION) {
        let authenticated = state.auth.authenticate(headers).await?;
        if authenticated.account_id != record.account_id {
            return Err(AppError::Forbidden);
        }
        return Ok(Some(
            state
                .auth
                .sign_playback_token(&record.id, &record.account_id)?,
        ));
    }
    if let Some(token) = cookie::read(headers, &state.config.analytics.playback_cookie_name) {
        state
            .auth
            .verify_playback_token(&token, &record.id, &record.account_id)?;
        return Ok(Some(token));
    }
    Err(AppError::Unauthorized)
}

fn playback_redemption_cookie_path(state: &AppState, media_id: &str) -> String {
    format!("{}/{}", state.config.routes.watch, media_id)
}

fn playback_asset_cookie_path(state: &AppState, media_id: &str) -> String {
    format!("{}/{media_id}/", state.config.routes.stream)
}

fn append_cookie(
    headers: &mut HeaderMap,
    state: &AppState,
    name: &str,
    value: &str,
    path: &str,
    max_age: Option<i64>,
) -> Result<(), AppError> {
    cookie::append(
        headers,
        SetCookie {
            name,
            value,
            path,
            domain: None,
            max_age_seconds: max_age,
            http_only: true,
            secure: state.config.analytics.cookie_secure,
            same_site: state.config.analytics.cookie_same_site.as_cookie_value(),
        },
    )
}

/// `Content-Security-Policy` value restricting who may frame the embed.
fn frame_ancestors(origins: &[String]) -> String {
    if origins.iter().any(|origin| origin == "*") {
        // Any web origin; `*` alone would not match `https:` pages in all
        // browsers' CSP implementations.
        "frame-ancestors * https: http:".to_owned()
    } else {
        format!("frame-ancestors 'self' {}", origins.join(" "))
    }
}

fn flag(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("1" | "true" | "yes" | "on" | "")
    )
}

/// Parses `90`, `90.5`, `1m30s`, `1h2m3s` or `2:30` into seconds.
fn parse_timestamp(value: &str) -> Option<f64> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<f64>() {
        return (seconds.is_finite() && seconds >= 0.0).then_some(seconds);
    }
    if value.contains(':') {
        let mut total = 0.0;
        for part in value.split(':') {
            let number = part.parse::<u32>().ok()?;
            total = total * 60.0 + f64::from(number);
        }
        return Some(total);
    }
    let mut total = 0.0;
    let mut number = String::new();
    for character in value.chars() {
        if character.is_ascii_digit() {
            number.push(character);
            continue;
        }
        let amount = f64::from(number.parse::<u32>().ok()?);
        number.clear();
        total += amount
            * match character.to_ascii_lowercase() {
                'h' => 3600.0,
                'm' => 60.0,
                's' => 1.0,
                _ => return None,
            };
    }
    if !number.is_empty() {
        total += f64::from(number.parse::<u32>().ok()?);
    }
    Some(total)
}

/// Percent-encodes a URL for use as a query parameter value.
fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_options_parse_leniently() {
        let options = EmbedOptions::from_query(&EmbedQuery {
            autoplay: Some("1".to_owned()),
            muted: None,
            loop_playback: Some("true".to_owned()),
            start: None,
            t: Some("1m30s".to_owned()),
        });
        assert!(options.autoplay);
        assert!(options.muted, "autoplay implies muted by default");
        assert!(options.loop_playback);
        assert!((options.start_seconds - 90.0).abs() < f64::EPSILON);

        let explicit = EmbedOptions::from_query(&EmbedQuery {
            autoplay: Some("1".to_owned()),
            muted: Some("0".to_owned()),
            start: Some("bogus".to_owned()),
            ..EmbedQuery::default()
        });
        assert!(!explicit.muted);
        assert!(explicit.start_seconds.abs() < f64::EPSILON);
    }

    #[test]
    fn timestamps_accept_common_formats() {
        assert_eq!(parse_timestamp("90"), Some(90.0));
        assert_eq!(parse_timestamp("2:30"), Some(150.0));
        assert_eq!(parse_timestamp("1h2m3s"), Some(3723.0));
        assert_eq!(parse_timestamp("-4"), None);
        assert_eq!(parse_timestamp("1x"), None);
    }

    #[test]
    fn frame_ancestors_reflect_configuration() {
        assert_eq!(
            frame_ancestors(&["*".to_owned()]),
            "frame-ancestors * https: http:"
        );
        assert_eq!(
            frame_ancestors(&["https://a.example".to_owned()]),
            "frame-ancestors 'self' https://a.example"
        );
    }

    #[test]
    fn urls_are_percent_encoded() {
        assert_eq!(
            percent_encode("https://x.test/watch/a b?c=d"),
            "https%3A%2F%2Fx.test%2Fwatch%2Fa%20b%3Fc%3Dd"
        );
    }
}
