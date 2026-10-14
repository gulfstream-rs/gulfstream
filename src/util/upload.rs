use std::path::Path;

use axum::extract::multipart::Field;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

pub async fn write_upload_field(
    mut field: Field<'_>,
    destination: &Path,
    maximum_bytes: u64,
) -> Result<(u64, String), AppError> {
    let mut output = tokio::fs::File::create(destination).await?;
    let mut size = 0_u64;
    let mut digest = Sha256::new();
    while let Some(chunk) = field.chunk().await? {
        size = size.saturating_add(chunk.len() as u64);
        if size > maximum_bytes {
            let _ = tokio::fs::remove_file(destination).await;
            return Err(AppError::PayloadTooLarge(format!(
                "source exceeds the configured limit of {maximum_bytes} bytes"
            )));
        }
        digest.update(&chunk);
        output.write_all(&chunk).await?;
    }
    output.flush().await?;
    output.sync_all().await?;
    Ok((size, hex::encode(digest.finalize())))
}

pub async fn read_text_field(
    mut field: Field<'_>,
    maximum_bytes: usize,
) -> Result<String, AppError> {
    let mut value = Vec::new();
    while let Some(chunk) = field.chunk().await? {
        if value.len().saturating_add(chunk.len()) > maximum_bytes {
            return Err(AppError::PayloadTooLarge(format!(
                "text field exceeds the configured limit of {maximum_bytes} bytes"
            )));
        }
        value.extend_from_slice(&chunk);
    }
    String::from_utf8(value)
        .map_err(|_| AppError::BadRequest("text fields must be UTF-8".to_owned()))
}
