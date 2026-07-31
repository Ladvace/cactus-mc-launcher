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

/// Resolve a URL's host and return every address it maps to, erroring if the
/// scheme isn't http(s) or any resolved address is private/loopback/link-local/
/// metadata. Used by `guarded_get`, which pins each request to the validated
/// addresses returned here.
async fn resolve_public_addrs(url: &reqwest::Url) -> Result<Vec<std::net::SocketAddr>> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::Other("only http(s) URLs are allowed".into()));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::Other("URL has no host".into()))?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| AppError::Other("could not resolve host".into()))?
        .collect();
    if addrs.is_empty() {
        return Err(AppError::Other("host did not resolve".into()));
    }
    for addr in &addrs {
        if is_disallowed_ip(addr.ip()) {
            return Err(AppError::Other(
                "refusing request to a private or loopback address".into(),
            ));
        }
    }
    Ok(addrs)
}

/// Ranges that are never a legitimate Minecraft server: link-local (incl. the
/// `169.254.169.254` cloud-metadata endpoint), unspecified, multicast, broadcast.
/// Deliberately narrower than `is_disallowed_ip` — LAN (`192.168.x`) and loopback
/// servers are common and must stay pingable; only the never-a-server ranges are
/// refused so the pinger can't be aimed at the metadata service.
pub fn is_never_a_server_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_link_local() || v4.is_unspecified() || v4.is_multicast() || v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            v6.is_unspecified() || v6.is_multicast() || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

const MAX_GUARDED_REDIRECTS: usize = 5;

/// GET a webview/third-party-supplied URL with SSRF protection that holds across
/// redirects and DNS rebinding. Redirects are followed manually (never
/// automatically); every hop is re-resolved and re-checked against the
/// private/loopback/link-local/metadata blocklist, and each request is pinned to
/// the exact addresses that were validated, so a rebind between the check and the
/// connect can't reach an internal service. Use this for any URL that originates
/// outside the launcher (icons, stickers, skins).
pub async fn guarded_get(raw: &str) -> Result<reqwest::Response> {
    let mut current = reqwest::Url::parse(raw).map_err(|_| AppError::Other("invalid URL".into()))?;
    for _ in 0..MAX_GUARDED_REDIRECTS {
        let addrs = resolve_public_addrs(&current).await?;
        let host = current.host_str().unwrap_or_default().to_string();
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::none())
            // Pin DNS for this host to the addresses we just validated so the
            // fetch can't connect to a different (rebinding) address.
            .resolve_to_addrs(&host, &addrs)
            .build()?;
        let resp = client.get(current.clone()).send().await?;
        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| AppError::Other("redirect without a location".into()))?;
            current = current
                .join(location)
                .map_err(|_| AppError::Other("invalid redirect location".into()))?;
            continue;
        }
        return Ok(resp);
    }
    Err(AppError::Other("too many redirects".into()))
}
