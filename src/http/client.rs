//! Client address resolution for rate limiting and geolocation.

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, header::HeaderName, request::Parts},
};

use crate::{error::AppError, infrastructure::protection::ClientKey, state::AppState};

/// The requesting client's address.
///
/// Uses `protection.client_ip_header` when configured (Gulfstream behind a
/// trusted proxy/CDN) and the TCP peer address otherwise. For list headers
/// such as `X-Forwarded-For` the last valid entry is used: it is the one the
/// trusted proxy appended, while earlier entries are client-controlled.
#[derive(Clone, Copy, Debug)]
pub struct ClientAddress(pub Option<IpAddr>);

impl ClientAddress {
    #[must_use]
    pub fn key(self) -> Option<ClientKey> {
        self.0.map(ClientKey::from_ip)
    }
}

impl FromRequestParts<AppState> for ClientAddress {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip());
        let header = state
            .config
            .protection
            .client_ip_header
            .as_deref()
            .and_then(|name| name.parse::<HeaderName>().ok());
        Ok(Self(resolve(&parts.headers, header.as_ref(), peer)))
    }
}

fn resolve(
    headers: &HeaderMap,
    header: Option<&HeaderName>,
    peer: Option<IpAddr>,
) -> Option<IpAddr> {
    let Some(header) = header else {
        return peer;
    };
    headers
        .get_all(header)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|entry| parse_entry(entry.trim()))
        .next_back()
        .or(peer)
}

/// Parses `203.0.113.7`, `[2001:db8::1]:443` or `203.0.113.7:8080`.
fn parse_entry(entry: &str) -> Option<IpAddr> {
    entry
        .parse::<IpAddr>()
        .ok()
        .or_else(|| entry.parse::<SocketAddr>().ok().map(|address| address.ip()))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn peer_is_used_without_trusted_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("198.51.100.9"));
        let peer = "192.0.2.1".parse().ok();
        assert_eq!(resolve(&headers, None, peer), peer);
    }

    #[test]
    fn last_forwarded_entry_wins() {
        let name = HeaderName::from_static("x-forwarded-for");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("10.0.0.1, spoofed, 198.51.100.9"),
        );
        assert_eq!(
            resolve(&headers, Some(&name), None),
            "198.51.100.9".parse().ok()
        );
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("[2001:db8::1]:443"),
        );
        assert_eq!(
            resolve(&headers, Some(&name), None),
            "2001:db8::1".parse().ok()
        );
        headers.insert("x-forwarded-for", HeaderValue::from_static("garbage"));
        let peer = "192.0.2.1".parse().ok();
        assert_eq!(resolve(&headers, Some(&name), peer), peer);
    }
}
