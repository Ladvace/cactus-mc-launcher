//! Shared HTTP client with an identifying User-Agent — Modrinth may block a
//! generic/absent UA, and some Mojang/Xbox endpoints reject one outright.

use crate::error::Result;

pub const USER_AGENT: &str = concat!(
    "cactus-launcher/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Ladvace/cactus-mc-launcher)"
);

pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().user_agent(USER_AGENT).build()?)
}
