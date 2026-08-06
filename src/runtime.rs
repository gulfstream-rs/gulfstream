use std::{sync::Arc, time::Duration};

use anyhow::Context;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    auth::AuthService,
    config::Config,
    http,
    infrastructure::{database, storage::Storage},
    state::AppState,
    workers,
};

/// Runs Gulfstream until the process receives a shutdown signal.
///
/// The function initializes tracing, storage, the database, HTTP routes, and
/// background workers from the supplied validated configuration.
pub async fn run(config: Config) -> anyhow::Result<()> {
    initialize_tracing(&config)?;
    let bind = config.server.bind;
    let shutdown_grace = Duration::from_secs(config.server.shutdown_grace_seconds);
    let state = build_state(config).await?;
    let app = http::router::build(state.clone())?;
    let worker_handles = workers::spawn(state.clone());
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("bind HTTP listener to {bind}"))?;

    tracing::info!(%bind, "gulfstream started");
    let cancellation = state.cancellation.clone();
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown_signal().await;
        cancellation.cancel();
    });
    let result = server.await.context("serve HTTP application");

    state.cancellation.cancel();
    for worker in worker_handles {
        match tokio::time::timeout(shutdown_grace, worker).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::error!(%error, "media worker task failed"),
            Err(_) => {
                tracing::warn!("media worker did not stop within the configured grace period")
            }
        }
    }
    result
}

async fn build_state(config: Config) -> anyhow::Result<AppState> {
    let pool = database::connect(&config.database).await?;
    let storage = Storage::initialize(config.storage.clone()).await?;
    let player_template = tokio::fs::read_to_string(&config.player.template_path)
        .await
        .with_context(|| {
            format!(
                "read player template {}",
                config.player.template_path.display()
            )
        })?;
    validate_player_template(&player_template)?;
    let web_template = if config.web.enabled {
        let template = tokio::fs::read_to_string(&config.web.template_path)
            .await
            .with_context(|| format!("read web template {}", config.web.template_path.display()))?;
        validate_web_template(&template)?;
        template
    } else {
        String::new()
    };
    let auth = AuthService::new(
        pool.clone(),
        config.security.clone(),
        config.browser_auth.clone(),
    );
    Ok(AppState {
        pool,
        config: Arc::new(config),
        storage,
        auth,
        jobs_available: Arc::new(Notify::new()),
        cancellation: CancellationToken::new(),
        player_template: Arc::new(player_template),
        web_template: Arc::new(web_template),
    })
}

fn initialize_tracing(config: &Config) -> anyhow::Result<()> {
    let filter = EnvFilter::try_new(&config.observability.log_filter)?;
    if config.observability.json_logs {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()?;
    }
    Ok(())
}

async fn shutdown_signal() {
    let control_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl+C signal handler");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to install termination signal handler"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = control_c => {},
        () = terminate => {},
    }
}

fn validate_player_template(template: &str) -> anyhow::Result<()> {
    const PLACEHOLDERS: [&str; 8] = [
        "{{TITLE}}",
        "{{DESCRIPTION}}",
        "{{HLS_SCRIPT}}",
        "{{STREAM_URL_JSON}}",
        "{{EVENT_URL_JSON}}",
        "{{SOURCE_MODE_JSON}}",
        "{{HEARTBEAT_MILLISECONDS}}",
        "<video",
    ];
    for placeholder in PLACEHOLDERS {
        anyhow::ensure!(
            template.contains(placeholder),
            "player template is missing required marker {placeholder}"
        );
    }
    Ok(())
}

fn validate_web_template(template: &str) -> anyhow::Result<()> {
    const PLACEHOLDERS: [&str; 7] = [
        "{{PAGE_TITLE}}",
        "{{PAGE_ID_JSON}}",
        "{{SITE_NAME}}",
        "{{TAGLINE}}",
        "{{ASSET_BASE}}",
        "{{RUNTIME_CONFIG_JSON}}",
        "id=\"app\"",
    ];
    for placeholder in PLACEHOLDERS {
        anyhow::ensure!(
            template.contains(placeholder),
            "web template is missing required marker {placeholder}"
        );
    }
    Ok(())
}
