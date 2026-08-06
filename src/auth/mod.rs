mod api_keys;
mod passwords;
mod playback;
mod sessions;

use std::sync::Arc;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;

use crate::config::{BrowserAuthConfig, SecurityConfig};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct AuthService {
    pool: SqlitePool,
    security: Arc<SecurityConfig>,
    browser: Arc<BrowserAuthConfig>,
}

#[derive(Debug)]
pub(crate) struct PreparedApiKey {
    pub(crate) database_id: String,
    pub(crate) key_id: String,
    pub(crate) name: String,
    pub(crate) api_key: String,
    pub(crate) secret_hash: String,
    pub(crate) created_at: String,
}

#[derive(Clone, Debug)]
pub struct BrowserSessionIssue {
    pub account_id: String,
    pub session_token: String,
    pub csrf_token: String,
    pub expires_at: String,
}

impl AuthService {
    #[must_use]
    pub fn new(pool: SqlitePool, security: SecurityConfig, browser: BrowserAuthConfig) -> Self {
        Self {
            pool,
            security: Arc::new(security),
            browser: Arc::new(browser),
        }
    }

    #[must_use]
    pub fn admin_token_matches(&self, expected: &str, supplied: &str) -> bool {
        expected.as_bytes().ct_eq(supplied.as_bytes()).into()
    }
}

fn keyed_hash(key: &[u8], purpose: &[u8], value: &str) -> anyhow::Result<String> {
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(key)?;
    mac.update(purpose);
    mac.update(&[0]);
    mac.update(value.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
