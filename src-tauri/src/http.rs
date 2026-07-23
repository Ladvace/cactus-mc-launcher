//! Shared HTTP client with an identifying User-Agent — Modrinth may block a
//! generic/absent UA, and some Mojang/Xbox endpoints reject one outright.

use std::net::IpAddr;
use std::sync::OnceLock;

use crate::error::{AppError, Result};

pub const USER_AGENT: &str = concat!(
    "cactus-launcher/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Ladvace/cactus-mc-launcher)"
);

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn backend_base() -> Option<String> {
    let base = option_env!("CACTUS_API_BASE")?.trim().trim_end_matches('/');
    (!base.is_empty()).then(|| base.to_string())
}

pub fn client() -> Result<reqwest::Client> {
    if let Some(existing) = CLIENT.get() {
        return Ok(existing.clone());
    }
    // A connect timeout bounds how long we wait to *establish* a connection
    // (e.g. a cold/unreachable backend) without capping slow downloads, which
    // keep transferring once connected.
    let built = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(15))
        // Abort a request that stalls with no bytes for 60s — bounds slow-loris
        // style hangs without capping the total time of a large, steady download.
        .read_timeout(std::time::Duration::from_secs(60))
        .build()?;
    // If another thread initialised it first, use theirs so everyone shares one.
    let _ = CLIENT.set(built);
    Ok(CLIENT.get().expect("client just set").clone())
}

/// IPs a user-supplied URL must never resolve to — blocks SSRF into loopback,
/// LAN, link-local (incl. the `169.254.169.254` cloud-metadata endpoint), and
/// other non-public ranges.
fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.octets()[0] == 0
                // Carrier-grade NAT 100.64.0.0/10
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // link-local fe80::/10
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // unique-local fc00::/7
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // IPv4-mapped — inspect the embedded v4
                || v6.to_ipv4_mapped().map(|m| is_disallowed_ip(IpAddr::V4(m))).unwrap_or(false)
        }
    }
}

/// Validate a URL supplied to an image/proxy fetch: must be http(s) and every
/// address it resolves to must be public. Guards commands like `download_image`
/// from being turned into an SSRF probe of internal services.
pub async fn ensure_public_url(raw: &str) -> Result<()> {
    let url = reqwest::Url::parse(raw)
        .map_err(|_| AppError::Other("invalid URL".into()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::Other("only http(s) URLs are allowed".into()));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::Other("URL has no host".into()))?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| AppError::Other("could not resolve host".into()))?;
    let mut resolved = false;
    for addr in addrs {
        resolved = true;
        if is_disallowed_ip(addr.ip()) {
            return Err(AppError::Other(
                "refusing request to a private or loopback address".into(),
            ));
        }
    }
    if !resolved {
        return Err(AppError::Other("host did not resolve".into()));
    }
    Ok(())
}
