// Some fields are deserialized for completeness / future use (integrity sizes,
// java major version) but not yet read programmatically.
#[allow(dead_code)]
pub mod version;

use serde::{Deserialize, Serialize};

use crate::error::Result;

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// A single entry from Mojang's version manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftVersion {
    pub id: String,
    /// "release" | "snapshot" | "old_beta" | "old_alpha"
    #[serde(rename = "type")]
    pub kind: String,
    /// URL to the per-version JSON (used later by the launch pipeline).
    pub url: String,
    pub release_time: String,
}

#[derive(Debug, Deserialize)]
struct Latest {
    release: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    latest: Latest,
    versions: Vec<MinecraftVersion>,
}

/// The list of available Minecraft versions plus the latest release/snapshot ids.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<MinecraftVersion>,
}

/// Fetch the full Minecraft version manifest from Mojang.
pub async fn fetch_versions() -> Result<VersionList> {
    let manifest: VersionManifest = reqwest::get(VERSION_MANIFEST_URL)
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(VersionList {
        latest_release: manifest.latest.release,
        latest_snapshot: manifest.latest.snapshot,
        versions: manifest.versions,
    })
}
