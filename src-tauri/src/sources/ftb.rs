//! Feed The Beast (FTB) modpack provider — the public `api.modpacks.ch` API
//! (no key). FTB only offers modpacks. Their pack files are a mix of directly
//! hosted files and CurseForge references, so installing resolves the latter
//! through the CurseForge module (see content::install_ftb_modpack).

use futures::stream::{self, StreamExt};
use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::modrinth::{SearchHit, SearchParams, SearchResults, Version, VersionHashes, VersionFile};

pub const API_BASE: &str = "https://api.modpacks.ch/public";
const USER_AGENT: &str = concat!("drake-launcher/", env!("CARGO_PKG_VERSION"));

pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().user_agent(USER_AGENT).build()?)
}

#[derive(Deserialize)]
struct SearchResp {
    #[serde(default)]
    packs: Vec<u64>,
}

#[derive(Deserialize)]
struct Art {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct Author {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
pub struct PackVersion {
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: String,
}

#[derive(Deserialize)]
struct Pack {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    synopsis: String,
    #[serde(default)]
    art: Vec<Art>,
    #[serde(default)]
    authors: Vec<Author>,
    #[serde(default)]
    installs: u64,
    #[serde(default)]
    versions: Vec<PackVersion>,
}

async fn fetch_pack(client: &reqwest::Client, id: u64) -> Option<Pack> {
    client
        .get(format!("{API_BASE}/modpack/{id}"))
        .send()
        .await
        .ok()?
        .json::<Pack>()
        .await
        .ok()
}

fn pack_to_hit(p: Pack) -> SearchHit {
    let icon = p
        .art
        .into_iter()
        .find(|a| a.kind == "square")
        .map(|a| a.url)
        .filter(|u| !u.is_empty());
    SearchHit {
        project_id: p.id.to_string(),
        slug: p.id.to_string(),
        title: p.name,
        description: p.synopsis,
        author: p.authors.into_iter().next().map(|a| a.name).unwrap_or_default(),
        downloads: p.installs,
        follows: 0,
        icon_url: icon,
        categories: Vec::new(),
        versions: Vec::new(),
        project_type: "modpack".into(),
        source: "ftb".into(),
    }
}

/// Search FTB modpacks. FTB only has modpacks, so other project types return
/// nothing. An empty query returns the most-installed packs.
pub async fn search(params: SearchParams) -> Result<SearchResults> {
    if params.project_type != "modpack" {
        return Ok(SearchResults {
            hits: Vec::new(),
            total_hits: 0,
            offset: params.offset,
            limit: params.limit,
        });
    }

    let client = client()?;
    let query = params.query.trim();
    // FTB search returns only ids; empty query -> popular.
    let list_url = if query.is_empty() {
        format!("{API_BASE}/modpack/popular/installs/50")
    } else {
        format!("{API_BASE}/modpack/search/50?term={}", urlencoding(query))
    };
    let ids: Vec<u64> = client
        .get(list_url)
        .send()
        .await?
        .error_for_status()?
        .json::<SearchResp>()
        .await?
        .packs;

    let total = ids.len() as u64;
    let limit = if params.limit == 0 { 20 } else { params.limit };
    let start = params.offset as usize;
    let page: Vec<u64> = ids.into_iter().skip(start).take(limit as usize).collect();

    // Fetch pack details concurrently (search only gives ids).
    let hits: Vec<SearchHit> = stream::iter(page)
        .map(|id| {
            let client = client.clone();
            async move { fetch_pack(&client, id).await.map(pack_to_hit) }
        })
        .buffer_unordered(8)
        .filter_map(|x| async move { x })
        .collect()
        .await;

    Ok(SearchResults {
        hits,
        total_hits: total,
        offset: params.offset,
        limit,
    })
}

/// List a pack's versions (newest first). Version id is a composite
/// `packId:versionId` so the modpack installer can fetch the manifest.
pub async fn get_versions(project_id: &str) -> Result<Vec<Version>> {
    let pack = fetch_pack(&client()?, project_id.parse().unwrap_or(0))
        .await
        .ok_or_else(|| AppError::Other("FTB pack not found".into()))?;

    let mut versions: Vec<Version> = pack
        .versions
        .into_iter()
        .map(|v| Version {
            id: format!("{}:{}", pack.id, v.id),
            project_id: pack.id.to_string(),
            name: v.name.clone(),
            version_number: v.name,
            version_type: v.kind.to_lowercase(),
            game_versions: Vec::new(),
            loaders: Vec::new(),
            files: vec![VersionFile {
                url: String::new(),
                filename: String::new(),
                primary: true,
                size: 0,
                hashes: VersionHashes { sha1: None, sha512: None },
            }],
            date_published: String::new(),
            downloads: 0,
        })
        .collect();
    versions.reverse();
    Ok(versions)
}

// --- Install manifest (files + targets) ---

#[derive(Deserialize)]
pub struct Target {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, rename = "type")]
    pub kind: String,
}

#[derive(Deserialize)]
pub struct CfRef {
    pub project: u64,
    pub file: u64,
}

#[derive(Deserialize)]
pub struct FtbFile {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub serveronly: bool,
    #[serde(default)]
    pub curseforge: Option<CfRef>,
}

#[derive(Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub targets: Vec<Target>,
    #[serde(default)]
    pub files: Vec<FtbFile>,
}

pub async fn fetch_manifest(pack_id: u64, version_id: u64) -> Result<Manifest> {
    Ok(client()?
        .get(format!("{API_BASE}/modpack/{pack_id}/{version_id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Minimal percent-encoding for the search term (spaces and a few specials).
fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
