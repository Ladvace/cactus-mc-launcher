//! News feed aggregation for the Home screen.
//!
//! Currently surfaces the official Minecraft news feed (the same one the
//! vanilla launcher uses). The `NewsItem` shape and `source` tag are
//! deliberately source-agnostic so more feeds (patch notes, launcher updates,
//! gaming news) can be folded in later without changing the frontend contract.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http;

const MOJANG_NEWS: &str = "https://launchercontent.mojang.com/v2/news.json";
const MOJANG_HOST: &str = "https://launchercontent.mojang.com";
const MAX_ITEMS: usize = 20;
const CACHE_TTL_SECS: i64 = 1800; // 30 minutes

/// A single, source-agnostic news entry sent to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub date: String,
    pub image: Option<String>,
    pub link: Option<String>,
    /// Which feed this came from ("minecraft"), for badges/filtering later.
    pub source: String,
}

// --- Simple in-memory cache so revisiting Home doesn't re-hit the network ---

struct Cache {
    fetched_at: i64,
    items: Vec<NewsItem>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn cached() -> Option<Vec<NewsItem>> {
    let guard = CACHE.lock().ok()?;
    let entry = guard.as_ref()?;
    (now() - entry.fetched_at < CACHE_TTL_SECS).then(|| entry.items.clone())
}

fn store(items: &[NewsItem]) {
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(Cache {
            fetched_at: now(),
            items: items.to_vec(),
        });
    }
}

/// Returns the latest news, served from cache when fresh. `force` bypasses the
/// cache (e.g. an explicit refresh).
pub async fn get(force: bool) -> Result<Vec<NewsItem>> {
    if !force {
        if let Some(items) = cached() {
            return Ok(items);
        }
    }
    let items = fetch_minecraft().await?;
    store(&items);
    Ok(items)
}

// --- Minecraft (Mojang launcher content) ---

#[derive(Deserialize)]
struct MojangNews {
    entries: Vec<MojangEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MojangEntry {
    title: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    read_more_link: Option<String>,
    #[serde(default)]
    news_page_image: Option<MojangImage>,
    #[serde(default)]
    play_page_image: Option<MojangImage>,
}

#[derive(Deserialize)]
struct MojangImage {
    url: String,
}

async fn fetch_minecraft() -> Result<Vec<NewsItem>> {
    let feed: MojangNews = http::client()?
        .get(MOJANG_NEWS)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let items = feed
        .entries
        .into_iter()
        .take(MAX_ITEMS)
        .map(map_entry)
        .collect();
    Ok(items)
}

fn map_entry(entry: MojangEntry) -> NewsItem {
    let image = entry
        .news_page_image
        .or(entry.play_page_image)
        .map(|img| absolute_url(&img.url));
    // No stable id in the feed; derive a unique one from date + title.
    let id = format!("mc:{}:{}", entry.date, entry.title);
    NewsItem {
        id,
        title: entry.title,
        summary: entry.text,
        category: entry.category,
        date: entry.date,
        image,
        link: entry.read_more_link,
        source: "minecraft".into(),
    }
}

fn absolute_url(url: &str) -> String {
    if url.starts_with("http") {
        url.to_string()
    } else {
        format!("{MOJANG_HOST}{url}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_absolute_image_urls() {
        assert_eq!(
            absolute_url("/v2/images/foo.png"),
            "https://launchercontent.mojang.com/v2/images/foo.png"
        );
        assert_eq!(absolute_url("https://cdn.example/foo.png"), "https://cdn.example/foo.png");
    }

    #[test]
    fn maps_a_feed_entry() {
        let json = r#"{
            "title": "The Aether awaits",
            "category": "Minecraft: Java Edition",
            "date": "2026-07-14",
            "text": "Step through a glowstone portal.",
            "readMoreLink": "https://minecraft.net/article/aether",
            "newsPageImage": { "url": "/v2/images/aether.png" }
        }"#;
        let entry: MojangEntry = serde_json::from_str(json).unwrap();
        let item = map_entry(entry);
        assert_eq!(item.source, "minecraft");
        assert_eq!(item.link.as_deref(), Some("https://minecraft.net/article/aether"));
        assert_eq!(
            item.image.as_deref(),
            Some("https://launchercontent.mojang.com/v2/images/aether.png")
        );
        assert!(item.id.contains("2026-07-14"));
    }
}
