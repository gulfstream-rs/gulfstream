use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString,
};
use uuid::Uuid;

use crate::error::AppError;

use super::AuthService;

impl AuthService {
    pub async fn hash_account_password(&self, password: &str) -> Result<String, AppError> {
        let peppered = format!("{password}{}", self.browser.password_pepper);
        let params = Params::new(
            self.browser.password_argon2_memory_kib,
            self.browser.password_argon2_iterations,
            self.browser.password_argon2_parallelism,
            None,
        )
        .map_err(AppError::internal)?;
        let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(AppError::internal)?;
        tokio::task::spawn_blocking(move || {
            Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
                .hash_password(peppered.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(anyhow::Error::from)
        })
        .await
        .map_err(AppError::internal)?
        .map_err(AppError::internal)
    }

    pub async fn verify_account_password(
        &self,
        password: &str,
        expected_hash: &str,
    ) -> Result<bool, AppError> {
        let peppered = format!("{password}{}", self.browser.password_pepper);
        let expected_hash = expected_hash.to_owned();
        let params = Params::new(
            self.browser.password_argon2_memory_kib,
            self.browser.password_argon2_iterations,
            self.browser.password_argon2_parallelism,
            None,
        )
        .map_err(AppError::internal)?;
        tokio::task::spawn_blocking(move || {
            let hash = PasswordHash::new(&expected_hash)?;
            Ok::<bool, anyhow::Error>(
                Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
                    .verify_password(peppered.as_bytes(), &hash)
                    .is_ok(),
            )
        })
        .await
        .map_err(AppError::internal)?
        .map_err(AppError::internal)
    }
}
