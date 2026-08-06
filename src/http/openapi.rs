use axum::{extract::State, http::header};

use crate::{error::AppError, state::AppState};

pub async fn serve(
    State(state): State<AppState>,
) -> Result<([(&'static str, &'static str); 1], String), AppError> {
    let document = include_str!("../../openapi.yaml")
        .replace(
            "{{PACKAGE_VERSION_JSON}}",
            &serde_json::to_string(env!("CARGO_PKG_VERSION")).map_err(AppError::internal)?,
        )
        .replace(
            "{{PUBLIC_BASE_URL_JSON}}",
            &serde_json::to_string(&state.config.server.public_base_url)
                .map_err(AppError::internal)?,
        )
        .replace("{{API_ROUTE}}", &state.config.routes.api)
        .replace("{{WATCH_ROUTE}}", &state.config.routes.watch)
        .replace("{{STREAM_ROUTE}}", &state.config.routes.stream)
        .replace("{{HEALTH_LIVE_ROUTE}}", &state.config.routes.health_live)
        .replace("{{HEALTH_READY_ROUTE}}", &state.config.routes.health_ready)
        .replace("{{OPENAPI_ROUTE}}", &state.config.routes.openapi)
        .replace(
            "{{REGISTRATION_ADMIN_HEADER_JSON}}",
            &serde_json::to_string(&state.config.registration.admin_header_name)
                .map_err(AppError::internal)?,
        )
        .replace(
            "{{CSRF_HEADER_NAME_JSON}}",
            &serde_json::to_string(&state.config.browser_auth.csrf_header_name)
                .map_err(AppError::internal)?,
        )
        .replace(
            "{{REPOSITORY_URL_JSON}}",
            &serde_json::to_string(&state.config.web.repository_url).map_err(AppError::internal)?,
        )
        .replace(
            "{{DOCUMENTATION_URL_JSON}}",
            &serde_json::to_string(&state.config.web.documentation_url)
                .map_err(AppError::internal)?,
        );

    Ok((
        [(header::CONTENT_TYPE.as_str(), "application/yaml")],
        document,
    ))
}
