use axum::http::HeaderMap;
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    config::RegistrationMode,
    domain::account::{Account, ApiKeySummary, AuthenticatedAccount, IssuedApiKey},
    error::AppError,
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterAccount {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAccount {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateApiKey {
    pub name: String,
}

pub async fn register(
    state: &AppState,
    headers: &HeaderMap,
    request: RegisterAccount,
) -> Result<Account, AppError> {
    authorize_registration(state, headers)?;
    let email = normalized_email(
        &request.email,
        state.config.registration.maximum_email_bytes,
    )?;
    let display_name = normalized_required(
        &request.display_name,
        "display_name",
        state.config.registration.maximum_display_name_bytes,
    )?;
    validate_password(state, &request.password)?;
    let password_hash = state.auth.hash_account_password(&request.password).await?;
    let id = Uuid::new_v4().to_string();
    let timestamp = now();
    let quota =
        i64::try_from(state.config.registration.default_storage_quota_bytes).map_err(|_| {
            AppError::BadRequest("configured account quota exceeds SQLite integer range".to_owned())
        })?;
    let account_insert = sqlx::query(
        "INSERT INTO accounts \
         (id, email, display_name, password_hash, status, storage_quota_bytes, created_at, updated_at) \
         VALUES (?, ?, ?, ?, 'active', ?, ?, ?)",
    )
    .bind(&id)
    .bind(&email)
    .bind(&display_name)
    .bind(password_hash)
    .bind(quota)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&state.pool)
    .await;
    if let Err(error) = account_insert {
        if error
            .as_database_error()
            .is_some_and(|database_error| database_error.is_unique_violation())
        {
            return Err(AppError::Conflict(
                "an account with this email already exists".to_owned(),
            ));
        }
        return Err(error.into());
    }
    Ok(Account {
        id,
        email,
        display_name,
        status: "active".to_owned(),
        storage_quota_bytes: quota,
        storage_used_bytes: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    })
}

pub async fn get_account(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
) -> Result<Account, AppError> {
    get_account_by_id(state, &authenticated.account_id).await
}

pub async fn get_account_by_id(state: &AppState, account_id: &str) -> Result<Account, AppError> {
    let row = sqlx::query(
        "SELECT accounts.id, accounts.email, accounts.display_name, accounts.status, \
                accounts.storage_quota_bytes, accounts.created_at, accounts.updated_at, \
                COALESCE(SUM(CASE WHEN media_files.deleted_at IS NULL THEN media_files.storage_bytes ELSE 0 END), 0) \
                    AS storage_used_bytes \
         FROM accounts LEFT JOIN media_files ON media_files.account_id = accounts.id \
         WHERE accounts.id = ? GROUP BY accounts.id",
    )
    .bind(account_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("account not found".to_owned()))?;
    Ok(Account {
        id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        status: row.get("status"),
        storage_quota_bytes: row.get("storage_quota_bytes"),
        storage_used_bytes: row.get("storage_used_bytes"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn update(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    request: UpdateAccount,
) -> Result<Account, AppError> {
    let display_name = normalized_required(
        &request.display_name,
        "display_name",
        state.config.registration.maximum_display_name_bytes,
    )?;
    let result = sqlx::query("UPDATE accounts SET display_name = ?, updated_at = ? WHERE id = ?")
        .bind(display_name)
        .bind(now())
        .bind(&authenticated.account_id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::NotFound("account not found".to_owned()));
    }
    get_account(state, authenticated).await
}

pub async fn list_api_keys(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
) -> Result<Vec<ApiKeySummary>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, created_at, last_used_at, revoked_at \
         FROM api_keys WHERE account_id = ? ORDER BY created_at DESC",
    )
    .bind(&authenticated.account_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ApiKeySummary {
            id: row.get("id"),
            name: row.get("name"),
            created_at: row.get("created_at"),
            last_used_at: row.get("last_used_at"),
            revoked_at: row.get("revoked_at"),
        })
        .collect())
}

pub async fn create_api_key(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    request: CreateApiKey,
) -> Result<IssuedApiKey, AppError> {
    let name = normalized_required(
        &request.name,
        "name",
        state.config.registration.maximum_api_key_name_bytes,
    )?;
    let (id, api_key, created_at) = state
        .auth
        .issue_api_key(&authenticated.account_id, &name)
        .await?;
    Ok(IssuedApiKey {
        id,
        name,
        api_key,
        created_at,
    })
}

pub async fn revoke_api_key(
    state: &AppState,
    authenticated: &AuthenticatedAccount,
    api_key_id: &str,
) -> Result<(), AppError> {
    let result = sqlx::query(
        "UPDATE api_keys SET revoked_at = ? WHERE id = ? AND account_id = ? AND revoked_at IS NULL",
    )
    .bind(now())
    .bind(api_key_id)
    .bind(&authenticated.account_id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("API key not found".to_owned()));
    }
    Ok(())
}

pub(crate) fn normalized_email(value: &str, maximum_bytes: usize) -> Result<String, AppError> {
    let email = normalized_required(value, "email", maximum_bytes)?.to_lowercase();
    if !is_valid_email(&email) {
        return Err(AppError::BadRequest(
            "email must be syntactically valid".to_owned(),
        ));
    }
    Ok(email)
}

fn authorize_registration(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    match state.config.registration.mode {
        RegistrationMode::Open => Ok(()),
        RegistrationMode::Disabled => Err(AppError::Forbidden),
        RegistrationMode::AdminToken => {
            let supplied = headers
                .get(state.config.registration.admin_header_name.as_str())
                .and_then(|value| value.to_str().ok())
                .ok_or(AppError::Forbidden)?;
            let expected = state
                .config
                .registration
                .admin_token
                .as_deref()
                .ok_or(AppError::Forbidden)?;
            if state.auth.admin_token_matches(expected, supplied) {
                Ok(())
            } else {
                Err(AppError::Forbidden)
            }
        }
    }
}

fn validate_password(state: &AppState, password: &str) -> Result<(), AppError> {
    let length = password.len();
    if length < state.config.browser_auth.minimum_password_bytes
        || length > state.config.browser_auth.maximum_password_bytes
    {
        return Err(AppError::BadRequest(format!(
            "password must contain between {} and {} bytes",
            state.config.browser_auth.minimum_password_bytes,
            state.config.browser_auth.maximum_password_bytes
        )));
    }
    Ok(())
}

fn normalized_required(value: &str, field: &str, maximum_bytes: usize) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if normalized.len() > maximum_bytes {
        return Err(AppError::BadRequest(format!(
            "{field} exceeds the configured limit of {maximum_bytes} bytes"
        )));
    }
    Ok(normalized.to_owned())
}

fn is_valid_email(value: &str) -> bool {
    if value
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return false;
    }
    let Some((local, domain)) = value.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !local.contains('@')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
