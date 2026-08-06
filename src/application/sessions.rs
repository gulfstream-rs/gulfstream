use serde::Deserialize;
use sqlx::Row;

use crate::{
    auth::BrowserSessionIssue, domain::account::Account, error::AppError, state::AppState,
};

use super::accounts;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    state: &AppState,
    request: LoginRequest,
) -> Result<(Account, BrowserSessionIssue), AppError> {
    if request.password.is_empty()
        || request.password.len() > state.config.browser_auth.maximum_password_bytes
    {
        return Err(AppError::Unauthorized);
    }
    let email = accounts::normalized_email(
        &request.email,
        state.config.registration.maximum_email_bytes,
    )
    .map_err(|_| AppError::Unauthorized)?;
    let row = sqlx::query("SELECT id, password_hash, status FROM accounts WHERE email = ?")
        .bind(email)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if row.get::<String, _>("status") != "active" {
        return Err(AppError::Forbidden);
    }
    let password_hash: String = row.get("password_hash");
    if !state
        .auth
        .verify_account_password(&request.password, &password_hash)
        .await?
    {
        return Err(AppError::Unauthorized);
    }
    let account_id: String = row.get("id");
    let session = state.auth.issue_browser_session(&account_id).await?;
    let account = accounts::get_account_by_id(state, &account_id).await?;
    Ok((account, session))
}
