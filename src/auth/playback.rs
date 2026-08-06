use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hmac::{KeyInit, Mac};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::{AuthService, HmacSha256};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaybackClaims {
    media_id: String,
    account_id: String,
    expires_at: i64,
}

impl AuthService {
    pub fn sign_playback_token(
        &self,
        media_id: &str,
        account_id: &str,
    ) -> Result<String, AppError> {
        let claims = PlaybackClaims {
            media_id: media_id.to_owned(),
            account_id: account_id.to_owned(),
            expires_at: Utc::now().timestamp() + self.security.playback_token_ttl_seconds,
        };
        let payload = serde_json::to_vec(&claims).map_err(AppError::internal)?;
        let encoded = URL_SAFE_NO_PAD.encode(payload);
        let mut mac =
            <HmacSha256 as KeyInit>::new_from_slice(self.security.playback_signing_key.as_bytes())
                .map_err(AppError::internal)?;
        mac.update(encoded.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{encoded}.{signature}"))
    }

    pub fn verify_playback_token(
        &self,
        token: &str,
        media_id: &str,
        account_id: &str,
    ) -> Result<(), AppError> {
        let (encoded, signature) = token.split_once('.').ok_or(AppError::Unauthorized)?;
        let supplied = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| AppError::Unauthorized)?;
        let mut mac =
            <HmacSha256 as KeyInit>::new_from_slice(self.security.playback_signing_key.as_bytes())
                .map_err(AppError::internal)?;
        mac.update(encoded.as_bytes());
        mac.verify_slice(&supplied)
            .map_err(|_| AppError::Unauthorized)?;
        let payload = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| AppError::Unauthorized)?;
        let claims: PlaybackClaims =
            serde_json::from_slice(&payload).map_err(|_| AppError::Unauthorized)?;
        if claims.media_id != media_id
            || claims.account_id != account_id
            || claims.expires_at < Utc::now().timestamp()
        {
            return Err(AppError::Unauthorized);
        }
        Ok(())
    }
}
