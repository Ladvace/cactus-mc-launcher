use serde::{Deserialize, Serialize};

use crate::error::Result;

const API_BASE: &str = "https://api.modrinth.com/v2";

pub fn client() -> Result<reqwest::Client> {
    crate::http::client()
}

#[derive(Debug, Serialize, Deserialize)]
// Modrinth returns snake_case; the frontend expects camelCase.
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub project_type: String,
    /// Which provider this hit came from ("modrinth" | "curseforge").
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "modrinth".into()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total_hits: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    #[serde(default)]
    pub query: String,
    /// "mod" | "modpack" | "resourcepack" | "shader" | "datapack"
    pub project_type: String,
    #[serde(default)]
    pub game_version: Option<String>,
    #[serde(default)]
    pub loader: Option<String>,
    /// "relevance" | "downloads" | "follows" | "newest" | "updated"
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub offset: u64,
    #[serde(default)]
    pub limit: u64,
}

pub async fn search(params: SearchParams) -> Result<SearchResults> {
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{}", params.project_type)]];
    if let Some(version) = params.game_version.as_deref().filter(|value| !value.is_empty()) {
        facets.push(vec![format!("versions:{version}")]);
    }
    if let Some(loader) = params.loader.as_deref().filter(|value| !value.is_empty()) {
        facets.push(vec![format!("categories:{loader}")]);
    }

    let facets_json = serde_json::to_string(&facets)?;
    let index = params.sort.unwrap_or_else(|| "relevance".into());
    let limit = if params.limit == 0 { 20 } else { params.limit.min(100) };
    let offset = params.offset.to_string();
    let limit_str = limit.to_string();

    let resp = client()?
        .get(format!("{API_BASE}/search"))
        .query(&[
            ("query", params.query.as_str()),
            ("facets", facets_json.as_str()),
            ("index", index.as_str()),
            ("offset", offset.as_str()),
            ("limit", limit_str.as_str()),
        ])
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(crate::error::AppError::Other(
            "Modrinth rate limit reached — wait a few seconds and try again.".into(),
        ));
    }

    Ok(resp.error_for_status()?.json().await?)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionHashes {
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub hashes: VersionHashes,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub downloads: u64,
}

pub async fn get_versions(
    project_id: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Vec<Version>> {
    let mut req = client()?.get(format!("{API_BASE}/project/{project_id}/version"));
    if let Some(loader) = loader.filter(|value| !value.is_empty()) {
        req = req.query(&[("loaders", format!("[\"{loader}\"]"))]);
    }
    if let Some(version) = game_version.filter(|value| !value.is_empty()) {
        req = req.query(&[("game_versions", format!("[\"{version}\"]"))]);
    }
    Ok(req.send().await?.error_for_status()?.json().await?)
}

pub async fn get_version(version_id: &str) -> Result<Version> {
    Ok(client()?
        .get(format!("{API_BASE}/version/{version_id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

impl Version {
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files
            .iter()
            .find(|file| file.primary)
            .or_else(|| self.files.first())
    }
}
