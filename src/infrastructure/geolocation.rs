//! Country attribution for playback analytics.
//!
//! Countries come from a trusted CDN header (for example Cloudflare's
//! `CF-IPCountry`) when configured, otherwise from a local MaxMind DB file.
//! Supported free databases: DB-IP "IP to Country Lite" (CC BY 4.0), MaxMind
//! GeoLite2 Country (GeoLite2 EULA) and IPinfo Lite (CC BY-SA 4.0). Only the
//! ISO 3166-1 alpha-2 code is kept; IP addresses are never stored.

use std::{net::IpAddr, path::Path};

use anyhow::Context;
use axum::http::{HeaderMap, HeaderName};
use maxminddb::{PathElement, Reader};

use crate::config::GeolocationConfig;

/// Code stored when the country cannot be determined (ISO 3166 user-assigned
/// "unknown or unspecified").
pub const UNKNOWN_COUNTRY: &str = "ZZ";

pub struct Geolocation {
    reader: Option<Reader<Vec<u8>>>,
    header: Option<HeaderName>,
}

impl std::fmt::Debug for Geolocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Geolocation")
            .field("database", &self.reader.is_some())
            .field("header", &self.header)
            .finish()
    }
}

impl Geolocation {
    pub fn load(config: &GeolocationConfig) -> anyhow::Result<Self> {
        let header = config
            .country_header
            .as_deref()
            .map(str::parse::<HeaderName>)
            .transpose()
            .context("parse geolocation.country_header")?;
        let reader = config
            .database_path
            .as_deref()
            .map(open_database)
            .transpose()?;
        Ok(Self { reader, header })
    }

    #[cfg(test)]
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            reader: None,
            header: None,
        }
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.reader.is_some() || self.header.is_some()
    }

    /// Resolves the viewer's country, or [`UNKNOWN_COUNTRY`].
    #[must_use]
    pub fn country(&self, headers: &HeaderMap, client: Option<IpAddr>) -> String {
        if let Some(code) = self
            .header
            .as_ref()
            .and_then(|name| headers.get(name))
            .and_then(|value| value.to_str().ok())
            .and_then(normalize_code)
        {
            return code;
        }
        client
            .and_then(|ip| self.lookup(ip))
            .unwrap_or_else(|| UNKNOWN_COUNTRY.to_owned())
    }

    fn lookup(&self, ip: IpAddr) -> Option<String> {
        let reader = self.reader.as_ref()?;
        let result = reader.lookup(ip).ok()?;
        // GeoIP2/GeoLite2/DB-IP layout first, then the IPinfo Lite layout.
        let paths: [&[PathElement<'_>]; 3] = [
            &[PathElement::Key("country"), PathElement::Key("iso_code")],
            &[
                PathElement::Key("registered_country"),
                PathElement::Key("iso_code"),
            ],
            &[PathElement::Key("country_code")],
        ];
        paths.into_iter().find_map(|path| {
            result
                .decode_path::<String>(path)
                .ok()
                .flatten()
                .as_deref()
                .and_then(normalize_code)
        })
    }
}

fn open_database(path: &Path) -> anyhow::Result<Reader<Vec<u8>>> {
    let reader = Reader::open_readfile(path)
        .with_context(|| format!("open geolocation database {}", path.display()))?;
    tracing::info!(
        path = %path.display(),
        database_type = %reader.metadata().database_type,
        build_epoch = reader.metadata().build_epoch,
        "geolocation database loaded"
    );
    Ok(reader)
}

/// Accepts ISO 3166-1 alpha-2 codes; rejects CDN placeholders such as `XX`
/// (unknown) and `T1` (Tor).
fn normalize_code(value: &str) -> Option<String> {
    let code = value.trim().to_ascii_uppercase();
    let valid = code.len() == 2
        && code.bytes().all(|byte| byte.is_ascii_uppercase())
        && code != "XX"
        && code != UNKNOWN_COUNTRY;
    valid.then_some(code)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn country_header_is_normalized_and_validated() {
        let geolocation = Geolocation::load(&GeolocationConfig {
            database_path: None,
            country_header: Some("cf-ipcountry".to_owned()),
        })
        .expect("config");
        let mut headers = HeaderMap::new();
        headers.insert("cf-ipcountry", HeaderValue::from_static("de"));
        assert_eq!(geolocation.country(&headers, None), "DE");
        for placeholder in ["XX", "T1", "", "DEU", "d1"] {
            headers.insert(
                "cf-ipcountry",
                HeaderValue::from_str(placeholder).expect("value"),
            );
            assert_eq!(geolocation.country(&headers, None), UNKNOWN_COUNTRY);
        }
    }

    #[test]
    fn missing_sources_yield_unknown() {
        let geolocation = Geolocation::disabled();
        assert!(!geolocation.is_enabled());
        assert_eq!(
            geolocation.country(&HeaderMap::new(), "192.0.2.1".parse().ok()),
            UNKNOWN_COUNTRY
        );
    }
}
