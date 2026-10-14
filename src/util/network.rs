use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::error::AppError;

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

#[cfg(test)]
mod tests {
    use super::is_non_public_ip;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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
