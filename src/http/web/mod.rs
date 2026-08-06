mod runtime;

use axum::{
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};

use crate::{error::AppError, state::AppState};

pub async fn root(State(state): State<AppState>) -> Result<Response, AppError> {
    let destination = if state.config.web.enabled {
        &state.config.routes.web
    } else {
        &state.config.routes.openapi
    };
    redirect(destination)
}

macro_rules! page_handler {
    ($name:ident, $title:literal, $page_id:literal) => {
        pub async fn $name(State(state): State<AppState>) -> Result<Response, AppError> {
            render(&state, $title, $page_id)
        }
    };
}

page_handler!(dashboard, "Dashboard", "dashboard");
page_handler!(register, "Create account", "register");
page_handler!(login, "Login", "login");
page_handler!(upload, "Upload", "upload");
page_handler!(media, "Media", "media");
page_handler!(media_detail, "Media details", "media_detail");
page_handler!(jobs, "Processing", "jobs");
page_handler!(analytics, "Analytics", "analytics");
page_handler!(account, "Account", "account");

fn render(state: &AppState, title: &str, page_id: &str) -> Result<Response, AppError> {
    if !state.config.web.enabled {
        return Err(AppError::NotFound("web interface is disabled".to_owned()));
    }

    let runtime_json = serde_json::to_string(&runtime::build(state)).map_err(AppError::internal)?;
    let page_id_json = serde_json::to_string(page_id).map_err(AppError::internal)?;
    let asset_base = format!(
        "{}{}",
        state.config.routes.web, state.config.web.assets_route
    );
    let html = state
        .web_template
        .replace("{{PAGE_TITLE}}", &html_escape(title))
        .replace("{{PAGE_ID_JSON}}", &page_id_json)
        .replace("{{SITE_NAME}}", &html_escape(&state.config.web.site_name))
        .replace("{{TAGLINE}}", &html_escape(&state.config.web.tagline))
        .replace("{{ASSET_BASE}}", &html_escape(&asset_base))
        .replace(
            "{{RUNTIME_CONFIG_JSON}}",
            &runtime_json.replace('<', "\\u003c"),
        );

    let mut response = Html(html).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        state
            .config
            .web
            .cache_control
            .parse::<HeaderValue>()
            .map_err(AppError::internal)?,
    );
    Ok(response)
}

fn redirect(location: &str) -> Result<Response, AppError> {
    let mut response = StatusCode::SEE_OTHER.into_response();
    response.headers_mut().insert(
        header::LOCATION,
        location
            .parse::<HeaderValue>()
            .map_err(AppError::internal)?,
    );
    Ok(response)
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
