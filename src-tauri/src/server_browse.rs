use serde::{Deserialize, Serialize};

use crate::error::Result;

const HOST: &str = "https://minecraft-list.info";
const API: &str = "https://minecraft-list.info/api/v1/servers";
const UA: &str = "CactusLauncher (+https://cactus.gianmarcocavallo.com)";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseParams {
    #[serde(default)]
    pub query: String,
    /// "players" | "votes" | "rating"
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseServer {
    pub name: String,
    pub address: String,
    pub online: bool,
    pub players: u64,
    pub max_players: u64,
    pub version: String,
    pub description: String,
    pub votes: u64,
    pub rating: f64,
    pub country: String,
    pub favicon: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseResult {
    pub servers: Vec<BrowseServer>,
    pub page: u32,
    pub has_more: bool,
}

#[derive(Deserialize)]
struct Raw {
    host: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    online: bool,
    #[serde(default)]
    players: u64,
    #[serde(default)]
    max_players: u64,
    #[serde(default)]
    server_version_raw: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    votes_count: u64,
    #[serde(default)]
    average_rating: f64,
    #[serde(default)]
    country: String,
    #[serde(default)]
    favicon_path: Option<String>,
}

#[derive(Deserialize, Default)]
struct View {
    #[serde(alias = "hydra:next")]
    next: Option<String>,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(alias = "hydra:member", default)]
    member: Vec<Raw>,
    #[serde(alias = "hydra:view", default)]
    view: Option<View>,
}

#[tauri::command]
pub async fn browse_servers(params: BrowseParams) -> Result<BrowseResult> {
    browse(params).await
}

async fn browse(params: BrowseParams) -> Result<BrowseResult> {
    let page = params.page.max(1);
    let page_str = page.to_string();
    let mut query: Vec<(String, String)> = vec![("page".into(), page_str)];
    let order_field = match params.sort.as_deref() {
        Some("votes") => "votesCount",
        Some("rating") => "averageRating",
        _ => "players",
    };
    query.push((format!("order[{order_field}]"), "desc".into()));
    let trimmed = params.query.trim();
    if !trimmed.is_empty() {
        query.push(("name".into(), trimmed.to_string()));
    }

    let resp = crate::http::client()?
        .get(API)
        .header("User-Agent", UA)
        .header("Accept", "application/ld+json")
        .query(&query)
        .send()
        .await?
        .error_for_status()?;
    let collection: Collection = resp.json().await?;

    let servers = collection
        .member
        .into_iter()
        .map(|raw| {
            let address = match raw.port {
                Some(port) if port != 25565 => format!("{}:{}", raw.host, port),
                _ => raw.host,
            };
            BrowseServer {
                name: raw.name,
                address,
                online: raw.online,
                players: raw.players,
                max_players: raw.max_players,
                version: raw.server_version_raw,
                description: raw.description,
                votes: raw.votes_count,
                rating: raw.average_rating,
                country: raw.country,
                favicon: raw
                    .favicon_path
                    .filter(|path| !path.is_empty())
                    .map(|path| format!("{HOST}{path}")),
            }
        })
        .collect();

    Ok(BrowseResult {
        servers,
        page,
        has_more: collection.view.and_then(|view| view.next).is_some(),
    })
}
