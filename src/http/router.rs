use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{delete, get, post},
};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    services::ServeDir,
    trace::TraceLayer,
};

use crate::{
    http::{
        accounts, analytics, dashboard, health, jobs, media, openapi, playback, sessions, uploads,
        web,
    },
    state::AppState,
};

pub fn build(state: AppState) -> anyhow::Result<Router> {
    let cors = cors_layer(&state)?;
    let max_body = state.config.server.max_request_body_bytes;
    let concurrency = state.config.server.max_in_flight_requests;

    let api = Router::new()
        .route("/accounts", post(accounts::register))
        .route("/auth/login", post(sessions::login))
        .route("/auth/session", get(sessions::current))
        .route("/auth/logout", post(sessions::logout))
        .route("/account", get(accounts::me).patch(accounts::update))
        .route(
            "/account/api-keys",
            get(accounts::list_api_keys).post(accounts::create_api_key),
        )
        .route(
            "/account/api-keys/{api_key_id}",
            delete(accounts::revoke_api_key),
        )
        .route("/dashboard", get(dashboard::get))
        .route("/media", post(uploads::upload).get(media::list))
        .route("/media/imports", post(uploads::remote_import))
        .route(
            "/media/{media_id}",
            get(media::get).patch(media::update).delete(media::delete),
        )
        .route("/media/{media_id}/retry", post(media::retry))
        .route("/media/{media_id}/jobs", get(media::jobs))
        .route(
            "/media/{media_id}/playback-tokens",
            post(media::playback_token),
        )
        .route("/jobs", get(jobs::list))
        .route("/analytics/summary", get(analytics::summary))
        .route("/analytics/time-series", get(analytics::time_series))
        .route(
            "/playback/{session_id}/events",
            post(playback::playback_event),
        );

    let watch = Router::new().route("/{media_id}", get(playback::watch));
    let stream = Router::new()
        .route("/{media_id}/master.m3u8", get(playback::master_playlist))
        .route("/{media_id}/source", get(playback::source))
        .route(
            "/{media_id}/{variant}/{asset}",
            get(playback::variant_asset),
        );

    let mut app = Router::new()
        .route("/", get(web::root))
        .route(&state.config.routes.health_live, get(health::live))
        .route(&state.config.routes.health_ready, get(health::ready))
        .route(&state.config.routes.openapi, get(openapi::serve))
        .nest(&state.config.routes.api, api)
        .nest(&state.config.routes.watch, watch)
        .nest(&state.config.routes.stream, stream);

    if state.config.web.enabled {
        let web_app = Router::new()
            .route("/", get(web::dashboard))
            .route("/register", get(web::register))
            .route("/login", get(web::login))
            .route("/upload", get(web::upload))
            .route("/media", get(web::media))
            .route("/media/{media_id}", get(web::media_detail))
            .route("/jobs", get(web::jobs))
            .route("/analytics", get(web::analytics))
            .route("/account", get(web::account))
            .nest_service(
                &state.config.web.assets_route,
                ServeDir::new(&state.config.web.assets_directory),
            );
        app = app.nest(&state.config.routes.web, web_app);
    }

    Ok(app
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(max_body))
        .layer(ConcurrencyLimitLayer::new(concurrency))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(CatchPanicLayer::new())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                tracing::info_span!(
                    "http.request",
                    method = %request.method(),
                    path = %request.uri().path()
                )
            }),
        )
        .with_state(state))
}

fn cors_layer(state: &AppState) -> anyhow::Result<CorsLayer> {
    let admin_header = state
        .config
        .registration
        .admin_header_name
        .parse::<header::HeaderName>()?;
    let csrf_header = state
        .config
        .browser_auth
        .csrf_header_name
        .parse::<header::HeaderName>()?;
    let allowed_headers = vec![
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::RANGE,
        admin_header,
        csrf_header,
    ];
    let mut layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(allowed_headers);
    if state
        .config
        .server
        .cors_allowed_origins
        .iter()
        .any(|origin| origin == "*")
    {
        layer = layer.allow_origin(Any);
    } else if !state.config.server.cors_allowed_origins.is_empty() {
        let origins = state
            .config
            .server
            .cors_allowed_origins
            .iter()
            .map(|origin| origin.parse::<HeaderValue>())
            .collect::<Result<Vec<_>, _>>()?;
        layer = layer.allow_origin(AllowOrigin::list(origins));
    }
    if state.config.server.cors_allow_credentials {
        layer = layer.allow_credentials(true);
    }
    Ok(layer)
}
