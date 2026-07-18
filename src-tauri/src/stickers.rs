//! Animated sticker search via the Giphy Stickers API (transparent animated
//! GIFs). Requires a free Giphy API key, entered by the user in Settings (get
//! one at https://developers.giphy.com). Empty = stickers disabled; the emoji
//! picker still works without it.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

const API_BASE: &str = "https://api.giphy.com/v1/stickers";

/// A sticker result normalized for the UI.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sticker {
    pub id: String,
    /// Small rendition shown in the picker grid.
    pub preview: String,
    /// Rendition downloaded and stored as the instance icon.
    pub full: String,
}

#[derive(Deserialize)]
struct GiphyResp {
    data: Vec<GiphyItem>,
}
#[derive(Deserialize)]
struct GiphyItem {
    id: String,
    images: GiphyImages,
}
#[derive(Deserialize)]
struct GiphyImages {
    fixed_width: Option<Rendition>,
    fixed_width_small: Option<Rendition>,
    fixed_width_downsampled: Option<Rendition>,
    preview_gif: Option<Rendition>,
}
#[derive(Deserialize)]
struct Rendition {
    url: String,
}

/// Search stickers (or trending when the query is empty). `api_key` is the
/// user's Giphy key from settings.
pub async fn search(api_key: &str, query: &str, offset: u32) -> Result<Vec<Sticker>> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(AppError::Other(
            "Stickers aren't configured. Add your free Giphy API key in \
             Settings → Interface (grab one at https://developers.giphy.com)."
                .into(),
        ));
    }

    let trimmed_query = query.trim();
    let endpoint = if trimmed_query.is_empty() { "trending" } else { "search" };
    let limit = 30u32.to_string();
    let offset = offset.to_string();

    let mut params: Vec<(&str, &str)> = vec![
        ("api_key", api_key),
        ("limit", &limit),
        ("offset", &offset),
        ("rating", "pg-13"),
        ("bundle", "messaging_non_clips"),
    ];
    if !trimmed_query.is_empty() {
        params.push(("q", trimmed_query));
    }

    let resp: GiphyResp = crate::http::client()?
        .get(format!("{API_BASE}/{endpoint}"))
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let stickers = resp
        .data
        .into_iter()
        .filter_map(|item| {
            let images = item.images;
            let full = images
                .fixed_width
                .as_ref()
                .or(images.fixed_width_downsampled.as_ref())
                .map(|rendition| rendition.url.clone())?;
            let preview = images
                .fixed_width_small
                .as_ref()
                .or(images.preview_gif.as_ref())
                .map(|rendition| rendition.url.clone())
                .unwrap_or_else(|| full.clone());
            Some(Sticker {
                id: item.id,
                preview,
                full,
            })
        })
        .collect();

    Ok(stickers)
}

/// Download an image (e.g. a chosen sticker) and encode it as a data URI so the
/// instance icon keeps working offline.
pub async fn download_data_uri(url: &str) -> Result<String> {
    let resp = reqwest::get(url).await?.error_for_status()?;
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/gif")
        .split(';')
        .next()
        .unwrap_or("image/gif")
        .to_string();

    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        return Err(AppError::Other("the sticker was empty".into()));
    }
    if bytes.len() > 4_000_000 {
        return Err(AppError::Other("that sticker is too large".into()));
    }

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{content_type};base64,{encoded}"))
}
