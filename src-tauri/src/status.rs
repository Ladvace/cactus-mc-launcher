//! Reachability checks for the external services the launcher depends on, so a
//! user can tell at a glance whether a failed browse/login/launch is their
//! connection or an upstream outage.

use futures::future::join_all;
use serde::Serialize;
use std::time::{Duration, Instant};

use crate::http;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    id: String,
    name: String,
    /// `None` when the service isn't configured for this build (e.g. CurseForge
    /// with no backend proxy) — shown as neutral rather than down.
    up: Option<bool>,
    latency_ms: Option<u64>,
}

const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// A service counts as "up" if it answers with any non-server-error status: we
/// probe reachability, so a 401/404/405 still means the API is alive. Only a
/// connection failure, a timeout, or a 5xx counts as down.
async fn probe(client: &reqwest::Client, url: &str, head: bool) -> Option<bool> {
    let request = if head { client.head(url) } else { client.get(url) };
    match request.timeout(PROBE_TIMEOUT).send().await {
        Ok(response) => Some(!response.status().is_server_error()),
        Err(_) => Some(false),
    }
}

#[tauri::command]
pub async fn service_status() -> Vec<ServiceStatus> {
    let Ok(client) = http::client() else {
        return Vec::new();
    };

    // CurseForge is reached through our backend proxy; without one it's simply
    // unavailable in this build, so leave its URL empty (reported as neutral).
    let curseforge_url = http::backend_base()
        .map(|base| format!("{base}/v1/curseforge/v1/mods/search?gameId=432&pageSize=1&index=0"));

    // (id, display name, url, use HEAD).
    let checks: [(&str, &str, Option<String>, bool); 6] = [
        ("modrinth", "Modrinth", Some("https://api.modrinth.com/".into()), false),
        ("curseforge", "CurseForge", curseforge_url, false),
        (
            "minecraft",
            "Minecraft services",
            Some("https://api.minecraftservices.com/".into()),
            false,
        ),
        (
            "microsoft",
            "Microsoft sign-in",
            Some("https://login.microsoftonline.com/".into()),
            false,
        ),
        (
            "mojang",
            "Mojang (game files)",
            Some("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json".into()),
            true,
        ),
        (
            "loaders",
            "Mod loaders (Fabric)",
            Some("https://meta.fabricmc.net/v2/versions/loader".into()),
            false,
        ),
    ];

    let probes = checks.into_iter().map(|(id, name, url, head)| {
        let client = client.clone();
        async move {
            let (up, latency_ms) = match url {
                Some(url) => {
                    let start = Instant::now();
                    let up = probe(&client, &url, head).await;
                    (up, Some(start.elapsed().as_millis() as u64))
                }
                None => (None, None),
            };
            ServiceStatus { id: id.into(), name: name.into(), up, latency_ms }
        }
    });

    join_all(probes).await
}
