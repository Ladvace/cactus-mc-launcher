//! Shared outbound HTTP client. Every request goes through here so it carries a
//! consistent, identifying `User-Agent`. Modrinth asks third-party clients to
//! identify themselves (a generic/absent UA "increases the likelihood that we
//! will block your traffic"), and several Mojang/Xbox endpoints reject requests
//! that send no UA at all.

use crate::error::Result;

/// Identifying User-Agent sent on every request.
pub const USER_AGENT: &str = concat!(
    "cactus-launcher/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Ladvace/cactus-mc-launcher)"
);

/// A `reqwest::Client` with the shared [`USER_AGENT`] set.
pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().user_agent(USER_AGENT).build()?)
}
