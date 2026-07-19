//! Content-source abstraction. Browse/install go through here so additional
//! providers can be added without touching the callers. The Modrinth types
//! (`SearchResults`, `Version`, …) are the normalized shape every provider
//! maps into.

pub mod curseforge;

use serde::Deserialize;

use crate::error::Result;
use crate::modrinth::{self, SearchParams, SearchResults, Version};

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Modrinth,
    CurseForge,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: String,
    pub enabled: bool,
}

pub fn available() -> Vec<SourceInfo> {
    vec![
        SourceInfo { id: "modrinth".into(), enabled: true },
        SourceInfo {
            id: "curseforge".into(),
            enabled: curseforge::is_configured(),
        },
    ]
}

pub async fn search(source: Source, params: SearchParams) -> Result<SearchResults> {
    match source {
        Source::Modrinth => modrinth::search(params).await,
        Source::CurseForge => curseforge::search(params).await,
    }
}

pub async fn get_versions(
    source: Source,
    project_id: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Vec<Version>> {
    match source {
        Source::Modrinth => modrinth::get_versions(project_id, loader, game_version).await,
        Source::CurseForge => curseforge::get_versions(project_id, loader, game_version).await,
    }
}

pub async fn get_version(source: Source, version_id: &str) -> Result<Version> {
    match source {
        Source::Modrinth => modrinth::get_version(version_id).await,
        Source::CurseForge => curseforge::get_version(version_id).await,
    }
}
