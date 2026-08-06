use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::{auth::AuthService, config::Config, infrastructure::storage::Storage};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub storage: Storage,
    pub auth: AuthService,
    pub jobs_available: Arc<Notify>,
    pub cancellation: CancellationToken,
    pub player_template: Arc<String>,
    pub web_template: Arc<String>,
}
