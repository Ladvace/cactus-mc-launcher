//! Shared HTTP client with an identifying User-Agent — Modrinth may block a
//! generic/absent UA, and some Mojang/Xbox endpoints reject one outright.

use std::sync::OnceLock;

use crate::error::Result;

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
        .build()?;
    // If another thread initialised it first, use theirs so everyone shares one.
    let _ = CLIENT.set(built);
    Ok(CLIENT.get().expect("client just set").clone())
}
