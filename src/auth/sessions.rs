use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    cookie,
    domain::account::{AuthenticatedAccount, AuthenticationMethod},
    error::AppError,
};

use super::{AuthService, BrowserSessionIssue, api_keys::has_bearer_header, keyed_hash, now};

impl AuthService {
    pub async fn authenticate(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthenticatedAccount, AppError> {
        if has_bearer_header(headers) {
            self.authenticate_api_key(headers).await
        } else {
            self.authenticate_session(headers, false).await
        }
    }

    pub async fn authenticate_for_write(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthenticatedAccount, AppError> {
        if has_bearer_header(headers) {
            self.authenticate_api_key(headers).await
        } else {
            self.authenticate_session(headers, true).await
        }
    }

    pub async fn refresh_browser_session(
        &self,
        headers: &HeaderMap,
    ) -> Result<BrowserSessionIssue, AppError> {
        let authenticated = self.authenticate_session(headers, false).await?;
        let AuthenticationMethod::BrowserSession { session_id } = authenticated.method else {
            return Err(AppError::Unauthorized);
        };
        let csrf_token = random_token();
        let csrf_hash = keyed_hash(
            self.browser.session_signing_key.as_bytes(),
            b"csrf",
            &csrf_token,
        )
        .map_err(AppError::internal)?;
        let row =
            sqlx::query("SELECT expires_at FROM browser_sessions WHERE id = ? AND account_id = ?")
                .bind(&session_id)
                .bind(&authenticated.account_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(AppError::Unauthorized)?;
        let expires_at: String = row.get("expires_at");
        let updated =
            sqlx::query("UPDATE browser_sessions SET csrf_hash = ?, last_used_at = ? WHERE id = ?")
                .bind(csrf_hash)
                .bind(now())
                .bind(&session_id)
                .execute(&self.pool)
                .await?;
        if updated.rows_affected() != 1 {
            return Err(AppError::Unauthorized);
        }
        Ok(BrowserSessionIssue {
            account_id: authenticated.account_id,
            session_token: String::new(),
            csrf_token,
            expires_at,
        })
    }

    pub async fn issue_browser_session(
        &self,
        account_id: &str,
    ) -> Result<BrowserSessionIssue, AppError> {
        let session_id = Uuid::new_v4().to_string();
        let session_token = random_token();
        let csrf_token = random_token();
        let token_hash = keyed_hash(
            self.browser.session_signing_key.as_bytes(),
            b"session",
            &session_token,
        )
        .map_err(AppError::internal)?;
        let csrf_hash = keyed_hash(
            self.browser.session_signing_key.as_bytes(),
            b"csrf",
            &csrf_token,
        )
        .map_err(AppError::internal)?;
        let timestamp = Utc::now();
        let expires_at = timestamp + Duration::seconds(self.browser.session_ttl_seconds);
        let mut transaction = self.pool.begin().await?;
        sqlx::query("DELETE FROM browser_sessions WHERE expires_at <= ?")
            .bind(timestamp.to_rfc3339())
            .execute(&mut *transaction)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM browser_sessions WHERE account_id = ?")
                .bind(account_id)
                .fetch_one(&mut *transaction)
                .await?;
        let maximum = i64::from(self.browser.maximum_sessions_per_account);
        if count >= maximum {
            let remove_count = count - maximum + 1;
            sqlx::query(
                "DELETE FROM browser_sessions WHERE id IN (\
                    SELECT id FROM browser_sessions WHERE account_id = ? ORDER BY created_at ASC LIMIT ?\
                 )",
            )
            .bind(account_id)
            .bind(remove_count)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query(
            "INSERT INTO browser_sessions \
             (id, account_id, token_hash, csrf_hash, created_at, last_used_at, expires_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session_id)
        .bind(account_id)
        .bind(token_hash)
        .bind(csrf_hash)
        .bind(timestamp.to_rfc3339())
        .bind(timestamp.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(BrowserSessionIssue {
            account_id: account_id.to_owned(),
            session_token,
            csrf_token,
            expires_at: expires_at.to_rfc3339(),
        })
    }

    pub async fn revoke_browser_session(
        &self,
        session_id: &str,
        account_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM browser_sessions WHERE id = ? AND account_id = ?")
            .bind(session_id)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn purge_expired_browser_sessions(&self) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM browser_sessions WHERE expires_at <= ?")
            .bind(now())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn authenticate_session(
        &self,
        headers: &HeaderMap,
        require_csrf: bool,
    ) -> Result<AuthenticatedAccount, AppError> {
        let token = cookie::read(headers, &self.browser.session_cookie_name)
            .ok_or(AppError::Unauthorized)?;
        let token_hash = keyed_hash(
            self.browser.session_signing_key.as_bytes(),
            b"session",
            &token,
        )
        .map_err(AppError::internal)?;
        let row = sqlx::query(
            "SELECT browser_sessions.id, browser_sessions.account_id, browser_sessions.csrf_hash, \
                    browser_sessions.last_used_at, browser_sessions.expires_at, accounts.status \
             FROM browser_sessions JOIN accounts ON accounts.id = browser_sessions.account_id \
             WHERE browser_sessions.token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
        let session_id: String = row.get("id");
        let account_id: String = row.get("account_id");
        if row.get::<String, _>("status") != "active" {
            return Err(AppError::Forbidden);
        }
        let current = Utc::now();
        let expires_at = parse_timestamp(row.get::<String, _>("expires_at"))?;
        let last_used_at = parse_timestamp(row.get::<String, _>("last_used_at"))?;
        let idle_deadline =
            last_used_at + Duration::seconds(self.browser.session_idle_timeout_seconds);
        if current >= expires_at || current >= idle_deadline {
            self.revoke_browser_session(&session_id, &account_id)
                .await?;
            return Err(AppError::Unauthorized);
        }
        if require_csrf {
            let supplied = headers
                .get(self.browser.csrf_header_name.as_str())
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
                .ok_or(AppError::Forbidden)?;
            let supplied_hash = keyed_hash(
                self.browser.session_signing_key.as_bytes(),
                b"csrf",
                supplied,
            )
            .map_err(AppError::internal)?;
            let expected_hash: String = row.get("csrf_hash");
            if !bool::from(supplied_hash.as_bytes().ct_eq(expected_hash.as_bytes())) {
                return Err(AppError::Forbidden);
            }
        }
        sqlx::query("UPDATE browser_sessions SET last_used_at = ? WHERE id = ?")
            .bind(current.to_rfc3339())
            .bind(&session_id)
            .execute(&self.pool)
            .await?;
        Ok(AuthenticatedAccount {
            account_id,
            method: AuthenticationMethod::BrowserSession { session_id },
        })
    }
}

fn random_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn parse_timestamp(value: String) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(AppError::internal)
}
