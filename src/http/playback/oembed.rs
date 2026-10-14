//! oEmbed provider (<https://oembed.com>) so CMSs such as WordPress, Ghost or
//! Notion can turn a pasted watch link into an embedded player.

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use super::ensure_ready;
use crate::{application::media, error::AppError, http::template::html_escape, state::AppState};

#[derive(Debug, Deserialize)]
pub struct OembedQuery {
    pub url: String,
    pub format: Option<String>,
    pub maxwidth: Option<u32>,
    pub maxheight: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct OembedResponse {
    version: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    title: String,
    provider_name: String,
    provider_url: String,
    width: u32,
    height: u32,
    html: String,
}

pub async fn oembed(
    State(state): State<AppState>,
    Query(query): Query<OembedQuery>,
) -> Result<Response, AppError> {
    if !state.config.embed.enabled {
        return Err(AppError::NotFound("embedding is disabled".to_owned()));
    }
    if query
        .format
        .as_deref()
        .is_some_and(|format| !format.eq_ignore_ascii_case("json"))
    {
        // Required by the oEmbed specification for unsupported formats.
        return Ok(StatusCode::NOT_IMPLEMENTED.into_response());
    }
    let media_id = media_id_from_url(&state, &query.url)
        .ok_or_else(|| AppError::NotFound("URL is not a media link of this site".to_owned()))?;
    let record = media::get_record(&state, &media_id).await?;
    // Do not reveal private media; the spec maps this to 401 or 404.
    if record.visibility == "private" {
        return Err(AppError::NotFound("media not found".to_owned()));
    }
    ensure_ready(&record)?;
    let dimensions: Option<(Option<i64>, Option<i64>)> =
        sqlx::query_as("SELECT width, height FROM media_files WHERE id = ?")
            .bind(&record.id)
            .fetch_optional(&state.pool)
            .await?;
    let (width, height) = fit(
        &state,
        dimensions.and_then(|(width, height)| Some((width?, height?))),
        query.maxwidth,
        query.maxheight,
    );
    let base = state.config.server.public_base_url.trim_end_matches('/');
    let src = format!("{base}{}/{}", state.config.routes.embed, record.id);
    let title = html_escape(&record.title);
    let html = format!(
        "<iframe src=\"{}\" width=\"{width}\" height=\"{height}\" title=\"{title}\" \
         frameborder=\"0\" allow=\"autoplay; fullscreen; picture-in-picture; encrypted-media; clipboard-write\" \
         allowfullscreen referrerpolicy=\"strict-origin-when-cross-origin\"></iframe>",
        html_escape(&src)
    );
    Ok(Json(OembedResponse {
        version: "1.0",
        kind: "video",
        title: record.title,
        provider_name: state.config.web.site_name.clone(),
        provider_url: format!("{base}/"),
        width,
        height,
        html,
    })
    .into_response())
}

/// Extracts the media identifier from a watch or embed URL of this site.
fn media_id_from_url(state: &AppState, url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let base = reqwest::Url::parse(&state.config.server.public_base_url).ok()?;
    if parsed.host_str() != base.host_str()
        || parsed.port_or_known_default() != base.port_or_known_default()
    {
        return None;
    }
    let path = parsed.path();
    [&state.config.routes.watch, &state.config.routes.embed]
        .into_iter()
        .find_map(|route| path.strip_prefix(route.as_str())?.strip_prefix('/'))
        .map(|rest| rest.trim_end_matches('/'))
        .filter(|id| !id.is_empty() && !id.contains('/'))
        .map(str::to_owned)
}

/// Default embed size, adjusted to the video's aspect ratio and the
/// consumer's maximum dimensions.
fn fit(
    state: &AppState,
    source: Option<(i64, i64)>,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> (u32, u32) {
    let mut width = f64::from(state.config.embed.default_width);
    let ratio = source.filter(|(w, h)| *w > 0 && *h > 0).map_or_else(
        || f64::from(state.config.embed.default_height) / width,
        |(w, h)| h as f64 / w as f64,
    );
    if let Some(max) = max_width.filter(|max| *max > 0) {
        width = width.min(f64::from(max));
    }
    let mut height = width * ratio;
    if let Some(max) = max_height.filter(|max| *max > 0)
        && height > f64::from(max)
    {
        height = f64::from(max);
        width = height / ratio;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (
        width.round().max(1.0) as u32,
        height.round().max(1.0) as u32,
    )
}
