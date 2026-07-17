use std::collections::HashMap;

use serde::Deserialize;
use tauri::AppHandle;

use crate::error::Result;
use crate::paths;

/// A downloadable artifact with integrity metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientDownload {
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    pub client: ClientDownload,
    /// Vanilla dedicated-server jar. Present for most releases from 1.2.5 on,
    /// absent for very old versions and some snapshots.
    #[serde(default)]
    pub server: Option<ClientDownload>,
}

/// OS/feature constraint. `action` is "allow" or "disallow".
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    /// Maven repository base URL (Fabric/Quilt/Forge style libraries that have
    /// no `downloads` block — resolved from `name` + this base).
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    /// Old-style natives: maps OS name -> classifier key (e.g. "natives-osx").
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(rename = "totalSize", default)]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    #[serde(default = "default_java_component")]
    pub component: String,
    #[serde(default = "default_java_major")]
    pub major_version: u32,
}

fn default_java_component() -> String {
    "jre-legacy".into()
}
fn default_java_major() -> u32 {
    8
}

/// A single argument entry: either a plain string or a value gated by rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

/// The per-version JSON from Mojang. `arguments` is present on 1.13+, while
/// `minecraft_arguments` is the legacy single-string form for older versions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetail {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub main_class: String,
    pub downloads: Downloads,
    pub asset_index: AssetIndexRef,
    pub assets: String,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
}

/// Fetch the per-version JSON, caching it under `meta/versions/<id>/<id>.json`.
/// `url` comes from the version manifest entry.
pub async fn fetch_detail(app: &AppHandle, id: &str, url: &str) -> Result<VersionDetail> {
    let cache = paths::version_dir(app, id)?.join(format!("{id}.json"));
    if cache.exists() {
        if let Ok(text) = std::fs::read_to_string(&cache) {
            if let Ok(detail) = serde_json::from_str::<VersionDetail>(&text) {
                return Ok(detail);
            }
        }
    }
    let text = reqwest::get(url).await?.error_for_status()?.text().await?;
    let detail: VersionDetail = serde_json::from_str(&text)?;
    std::fs::write(&cache, &text)?;
    Ok(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_parses_with_server() {
        let json = r#"{"client":{"sha1":"a","size":1,"url":"c"},"server":{"sha1":"b","size":2,"url":"s"}}"#;
        let d: Downloads = serde_json::from_str(json).unwrap();
        assert_eq!(d.client.url, "c");
        assert_eq!(d.server.unwrap().url, "s");
    }

    #[test]
    fn downloads_parses_without_server() {
        let json = r#"{"client":{"sha1":"a","url":"c"}}"#;
        let d: Downloads = serde_json::from_str(json).unwrap();
        assert!(d.server.is_none());
        assert_eq!(d.client.size, 0); // size defaults
    }
}
