#[allow(dead_code)]
pub mod version;

use serde::{Deserialize, Serialize};

use crate::error::Result;

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftVersion {
    pub id: String,
    /// "release" | "snapshot" | "old_beta" | "old_alpha"
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub release_time: String,
    /// sha1 of the per-version JSON at `url` (from `version_manifest_v2`), used
    /// to verify the detail JSON before it's trusted/cached.
    #[serde(default)]
    pub sha1: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<MinecraftVersion>,
}

pub async fn fetch_versions() -> Result<VersionList> {
    let manifest: VersionManifest = crate::http::client()?
        .get(VERSION_MANIFEST_URL)
        .send()
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
