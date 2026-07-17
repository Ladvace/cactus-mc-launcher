//! CurseForge provider. Maps the CurseForge (Eternal) API into the normalized
//! Modrinth-shaped types so the rest of the app is source-agnostic.
//!
//! Needs a free API key from https://console.curseforge.com, provided at build
//! time via `CURSEFORGE_API_KEY` (see build.rs / .env).
//!
//! Note: mod authors can opt out of third-party distribution. For those files
//! the API returns a null download URL and we cannot fetch them — the file's
//! `url` is left empty and install refuses with a helpful message.

use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::modrinth::{
    SearchHit, SearchParams, SearchResults, Version, VersionFile, VersionHashes,
};

const API_BASE: &str = "https://api.curseforge.com";
const GAME_ID: u64 = 432; // Minecraft
const USER_AGENT: &str = concat!("cactus-launcher/", env!("CARGO_PKG_VERSION"));

const CF_API_KEY: &str = match option_env!("CURSEFORGE_API_KEY") {
    Some(k) => k,
    None => "",
};

pub fn is_configured() -> bool {
    !CF_API_KEY.is_empty()
}

fn ensure_configured() -> Result<()> {
    if CF_API_KEY.is_empty() {
        return Err(AppError::Other(
            "CurseForge isn't configured. Add a CURSEFORGE_API_KEY in \
             src-tauri/.env (get one at https://console.curseforge.com)."
                .into(),
        ));
    }
    Ok(())
}

fn client() -> Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "x-api-key",
        CF_API_KEY
            .parse()
            .map_err(|_| AppError::Other("invalid CurseForge API key".into()))?,
    );
    Ok(reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()?)
}

/// CurseForge class id for a project type.
fn class_id(project_type: &str) -> u64 {
    match project_type {
        "modpack" => 4471,
        "resourcepack" => 12,
        "shader" => 6552,
        "datapack" => 6945,
        _ => 6, // mod
    }
}

fn project_type_of(class_id: u64) -> &'static str {
    match class_id {
        4471 => "modpack",
        12 => "resourcepack",
        6552 => "shader",
        6945 => "datapack",
        _ => "mod",
    }
}

/// CurseForge modLoaderType for a loader name (0 = any).
fn loader_type(loader: &str) -> u64 {
    match loader.to_lowercase().as_str() {
        "forge" => 1,
        "fabric" => 4,
        "quilt" => 5,
        "neoforge" => 6,
        _ => 0,
    }
}

/// CurseForge sortField for our sort keys.
fn sort_field(sort: &str) -> u64 {
    match sort {
        "downloads" => 6,  // TotalDownloads
        "newest" | "updated" => 3, // LastUpdated
        _ => 2,            // Popularity
    }
}

const LOADER_NAMES: [&str; 5] = ["Forge", "Fabric", "Quilt", "NeoForge", "LiteLoader"];

// --- API response shapes (only the fields we use) ---

#[derive(Deserialize)]
struct Logo {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
struct Author {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct Category {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct FileIndex {
    #[serde(default, rename = "gameVersion")]
    game_version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    logo: Option<Logo>,
    #[serde(default)]
    authors: Vec<Author>,
    #[serde(default)]
    categories: Vec<Category>,
    #[serde(default)]
    class_id: u64,
    #[serde(default)]
    latest_files_indexes: Vec<FileIndex>,
}

#[derive(Deserialize)]
struct Pagination {
    #[serde(default)]
    total_count: u64,
}

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<CfMod>,
    #[serde(default)]
    pagination: Option<Pagination>,
}

#[derive(Deserialize)]
struct CfHash {
    #[serde(default)]
    value: String,
    #[serde(default)]
    algo: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    mod_id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    release_type: u64,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<CfHash>,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    file_length: u64,
}

#[derive(Deserialize)]
struct FilesResponse {
    data: Vec<CfFile>,
}

#[derive(Deserialize)]
struct FileResponse {
    data: CfFile,
}

// --- Mapping into normalized types ---

fn map_mod(cf_mod: CfMod) -> SearchHit {
    let mut versions: Vec<String> = cf_mod
        .latest_files_indexes
        .into_iter()
        .map(|file_index| file_index.game_version)
        .filter(|version| !version.is_empty())
        .collect();
    versions.dedup();

    SearchHit {
        project_id: cf_mod.id.to_string(),
        slug: cf_mod.slug,
        title: cf_mod.name,
        description: cf_mod.summary,
        author: cf_mod.authors.into_iter().next().map(|author| author.name).unwrap_or_default(),
        downloads: cf_mod.download_count as u64,
        follows: 0,
        icon_url: cf_mod.logo.and_then(|logo| logo.url),
        categories: cf_mod.categories.into_iter().map(|category| category.name).collect(),
        versions,
        project_type: project_type_of(cf_mod.class_id).to_string(),
        source: "curseforge".to_string(),
    }
}

fn map_file(cf_file: CfFile) -> Version {
    let sha1 = cf_file
        .hashes
        .into_iter()
        .find(|hash| hash.algo == 1)
        .map(|hash| hash.value);

    let loaders: Vec<String> = cf_file
        .game_versions
        .iter()
        .filter(|version| LOADER_NAMES.iter().any(|name| name.eq_ignore_ascii_case(version)))
        .map(|version| version.to_lowercase())
        .collect();
    let game_versions: Vec<String> = cf_file
        .game_versions
        .iter()
        .filter(|version| version.chars().next().map(|ch| ch.is_ascii_digit()).unwrap_or(false))
        .cloned()
        .collect();

    let version_type = match cf_file.release_type {
        2 => "beta",
        3 => "alpha",
        _ => "release",
    }
    .to_string();

    Version {
        // Composite id so `get_version` can re-fetch (CF needs mod id + file id).
        id: format!("{}:{}", cf_file.mod_id, cf_file.id),
        project_id: cf_file.mod_id.to_string(),
        name: cf_file.display_name.clone(),
        version_number: if cf_file.file_name.is_empty() {
            cf_file.display_name
        } else {
            cf_file.file_name.clone()
        },
        version_type,
        game_versions,
        loaders,
        files: vec![VersionFile {
            url: cf_file.download_url.unwrap_or_default(),
            filename: cf_file.file_name,
            primary: true,
            size: cf_file.file_length,
            hashes: VersionHashes { sha1, sha512: None },
        }],
        date_published: cf_file.file_date,
        downloads: cf_file.download_count as u64,
    }
}

// --- Provider entry points ---

pub async fn search(params: SearchParams) -> Result<SearchResults> {
    ensure_configured()?;

    let class = class_id(&params.project_type).to_string();
    let sort = sort_field(params.sort.as_deref().unwrap_or("relevance")).to_string();
    let limit = if params.limit == 0 { 20 } else { params.limit.min(50) };
    let index = params.offset.to_string();
    let page_size = limit.to_string();
    let game_id = GAME_ID.to_string();

    let mut query: Vec<(&str, String)> = vec![
        ("gameId", game_id),
        ("classId", class),
        ("searchFilter", params.query.clone()),
        ("sortField", sort),
        ("sortOrder", "desc".to_string()),
        ("index", index),
        ("pageSize", page_size),
    ];
    if let Some(version) = params.game_version.as_deref().filter(|value| !value.is_empty()) {
        query.push(("gameVersion", version.to_string()));
    }
    if let Some(loader) = params.loader.as_deref().filter(|value| !value.is_empty()) {
        let loader_type_id = loader_type(loader);
        if loader_type_id != 0 {
            query.push(("modLoaderType", loader_type_id.to_string()));
        }
    }

    let resp: SearchResponse = client()?
        .get(format!("{API_BASE}/v1/mods/search"))
        .query(&query)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(SearchResults {
        total_hits: resp.pagination.map(|pagination| pagination.total_count).unwrap_or(0),
        offset: params.offset,
        limit,
        hits: resp.data.into_iter().map(map_mod).collect(),
    })
}

pub async fn get_versions(
    project_id: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Vec<Version>> {
    ensure_configured()?;

    let mut query: Vec<(&str, String)> = vec![("pageSize", "50".to_string())];
    if let Some(version) = game_version.filter(|value| !value.is_empty()) {
        query.push(("gameVersion", version.to_string()));
    }
    if let Some(loader_name) = loader.filter(|value| !value.is_empty()) {
        let loader_type_id = loader_type(loader_name);
        if loader_type_id != 0 {
            query.push(("modLoaderType", loader_type_id.to_string()));
        }
    }

    let resp: FilesResponse = client()?
        .get(format!("{API_BASE}/v1/mods/{project_id}/files"))
        .query(&query)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(resp.data.into_iter().map(map_file).collect())
}

pub async fn get_version(version_id: &str) -> Result<Version> {
    ensure_configured()?;
    let (mod_id, file_id) = version_id
        .split_once(':')
        .ok_or_else(|| AppError::Other("invalid CurseForge version id".into()))?;

    let resp: FileResponse = client()?
        .get(format!("{API_BASE}/v1/mods/{mod_id}/files/{file_id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(map_file(resp.data))
}
