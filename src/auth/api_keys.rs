use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString,
};
use axum::http::HeaderMap;
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::{
    domain::account::{AuthenticatedAccount, AuthenticationMethod},
    error::AppError,
};

use super::{AuthService, PreparedApiKey, now};

impl AuthService {
    pub(crate) async fn authenticate_api_key(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthenticatedAccount, AppError> {
        let token = bearer_token(headers).ok_or(AppError::Unauthorized)?;
        let (key_id, secret) = parse_api_key(token, &self.security.api_key_prefix)?;
        let row = sqlx::query(
            "SELECT api_keys.id, api_keys.account_id, api_keys.secret_hash, accounts.status \
             FROM api_keys JOIN accounts ON accounts.id = api_keys.account_id \
             WHERE api_keys.key_id = ? AND api_keys.revoked_at IS NULL",
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
        if row.get::<String, _>("status") != "active" {
            return Err(AppError::Forbidden);
        }
        let expected_hash: String = row.get("secret_hash");
        let peppered = format!("{secret}{}", self.security.api_key_pepper);
        let params = Params::new(
            self.security.api_key_argon2_memory_kib,
            self.security.api_key_argon2_iterations,
            self.security.api_key_argon2_parallelism,
            None,
        )
        .map_err(AppError::internal)?;
        let verified = tokio::task::spawn_blocking(move || {
            let hash = PasswordHash::new(&expected_hash)?;
            Ok::<bool, anyhow::Error>(
                Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
                    .verify_password(peppered.as_bytes(), &hash)
                    .is_ok(),
            )
        })
        .await
        .map_err(AppError::internal)?
        .map_err(AppError::internal)?;
        if !verified {
            return Err(AppError::Unauthorized);
        }
        let api_key_id: String = row.get("id");
        let account_id: String = row.get("account_id");
        sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
            .bind(now())
            .bind(&api_key_id)
            .execute(&self.pool)
            .await?;
        Ok(AuthenticatedAccount {
            account_id,
            method: AuthenticationMethod::ApiKey,
        })
    }

    pub(crate) async fn prepare_api_key(&self, name: &str) -> Result<PreparedApiKey, AppError> {
        let database_id = Uuid::new_v4().to_string();
        let key_id = Uuid::new_v4().simple().to_string()[..16].to_owned();
        let secret = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let peppered = format!("{secret}{}", self.security.api_key_pepper);
        let params = Params::new(
            self.security.api_key_argon2_memory_kib,
            self.security.api_key_argon2_iterations,
            self.security.api_key_argon2_parallelism,
            None,
        )
        .map_err(AppError::internal)?;
        let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(AppError::internal)?;
        let secret_hash = tokio::task::spawn_blocking(move || {
            Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
                .hash_password(peppered.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(anyhow::Error::from)
        })
        .await
        .map_err(AppError::internal)?
        .map_err(AppError::internal)?;
        Ok(PreparedApiKey {
            database_id,
            api_key: format!("{}_{key_id}_{secret}", self.security.api_key_prefix),
            key_id,
            name: name.to_owned(),
            secret_hash,
            created_at: now(),
        })
    }

    pub(crate) async fn insert_prepared_api_key(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        account_id: &str,
        prepared: &PreparedApiKey,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO api_keys (id, account_id, key_id, name, secret_hash, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&prepared.database_id)
        .bind(account_id)
        .bind(&prepared.key_id)
        .bind(&prepared.name)
        .bind(&prepared.secret_hash)
        .bind(&prepared.created_at)
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn issue_api_key(
        &self,
        account_id: &str,
        name: &str,
    ) -> Result<(String, String, String), AppError> {
        let prepared = self.prepare_api_key(name).await?;
        let mut transaction = self.pool.begin().await?;
        self.insert_prepared_api_key(&mut transaction, account_id, &prepared)
            .await?;
        transaction.commit().await?;
        Ok((prepared.database_id, prepared.api_key, prepared.created_at))
    }
}

pub(crate) fn has_bearer_header(headers: &HeaderMap) -> bool {
    headers.contains_key(axum::http::header::AUTHORIZATION)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let (scheme, token) = value.split_once(' ')?;
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

fn parse_api_key<'a>(
    token: &'a str,
    expected_prefix: &str,
) -> Result<(&'a str, &'a str), AppError> {
    let mut parts = token.splitn(3, '_');
    if parts.next() != Some(expected_prefix) {
        return Err(AppError::Unauthorized);
    }
    let key_id = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)?;
    let secret = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)?;
    Ok((key_id, secret))
}
