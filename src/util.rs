use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
};

use axum::extract::multipart::Field;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

#[must_use]
pub fn clean_filename(input: &str, fallback: &str, maximum_bytes: usize) -> String {
    let source = Path::new(input)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(fallback);
    let cleaned: String = source
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let normalized = cleaned.trim_matches(['.', '_']);
    if normalized.is_empty() {
        return fallback.to_owned();
    }
    if normalized.len() <= maximum_bytes {
        return normalized.to_owned();
    }
    let truncated = &normalized[..maximum_bytes];
    let truncated = truncated.trim_matches(['.', '_']);
    if truncated.is_empty() {
        fallback.to_owned()
    } else {
        truncated.to_owned()
    }
}

#[must_use]
pub fn title_from_filename(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename)
        .replace(['_', '-'], " ")
        .trim()
        .to_owned()
}

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

pub async fn validate_remote_url(
    url: &reqwest::Url,
    allow_private_networks: bool,
) -> Result<Vec<SocketAddr>, AppError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::RemoteRejected(
            "only HTTP and HTTPS URLs are allowed".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::RemoteRejected(
            "URLs containing credentials are not allowed".to_owned(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::RemoteRejected("URL has no host".to_owned()))?;
    if !allow_private_networks
        && (host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost"))
    {
        return Err(AppError::RemoteRejected(
            "localhost is not allowed".to_owned(),
        ));
    }
    let port = url
        .port_or_known_default()
        .ok_or_else(|| AppError::RemoteRejected("URL has no usable port".to_owned()))?;
    let addresses: Vec<_> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| AppError::RemoteRejected(format!("cannot resolve remote host: {error}")))?
        .collect();
    if addresses.is_empty() {
        return Err(AppError::RemoteRejected(
            "remote host did not resolve".to_owned(),
        ));
    }
    if !allow_private_networks
        && addresses
            .iter()
            .any(|address| is_non_public_ip(address.ip()))
    {
        return Err(AppError::RemoteRejected(
            "private, loopback, link-local, documentation, multicast, or unspecified addresses are not allowed".to_owned(),
        ));
    }
    Ok(addresses)
}

fn is_non_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => is_non_public_v4(address),
        IpAddr::V6(address) => is_non_public_v6(address),
    }
}

fn is_non_public_v4(address: Ipv4Addr) -> bool {
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_unspecified()
        || address.is_documentation()
        || address.is_multicast()
        || address.octets()[0] == 0
}

fn is_non_public_v6(address: Ipv6Addr) -> bool {
    address.is_loopback()
        || address.is_unspecified()
        || address.is_unique_local()
        || address.is_unicast_link_local()
        || address.is_multicast()
        || address.to_ipv4_mapped().is_some_and(is_non_public_v4)
}

#[derive(Debug)]
pub struct CleanupPath {
    path: std::path::PathBuf,
    kind: CleanupKind,
    armed: bool,
}

#[derive(Clone, Copy, Debug)]
enum CleanupKind {
    File,
    Directory,
}

impl CleanupPath {
    #[must_use]
    pub fn file(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: CleanupKind::File,
            armed: true,
        }
    }

    #[must_use]
    pub fn directory(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: CleanupKind::Directory,
            armed: true,
        }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CleanupPath {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        match self.kind {
            CleanupKind::File => {
                let _ = std::fs::remove_file(&self.path);
            }
            CleanupKind::Directory => {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{clean_filename, is_non_public_ip};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn filename_cleanup_removes_paths_and_unsafe_characters() {
        assert_eq!(
            clean_filename("../../holiday clip.mp4", "upload", 240),
            "holiday_clip.mp4"
        );
        assert_eq!(clean_filename("..", "upload", 240), "upload");
        assert_eq!(clean_filename("movie.mp4", "upload", 5), "movie");
    }

    #[test]
    fn private_and_special_addresses_are_rejected() {
        assert!(is_non_public_ip(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(is_non_public_ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));
        assert!(is_non_public_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_non_public_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_non_public_ip(IpAddr::V6(
            "2606:4700:4700::1111".parse().expect("valid test address")
        )));
    }
}
