//! Every `<img src>` the Svelte app drew: `data:` URIs, `http(s)` URLs, files on disk and static
//! paths (`/decor/…`). Ask from the view with [`Images::get`]; the first ask starts the load, and
//! the window wakes when it lands. Pixel art stays crisp: small images are decoded here and
//! upscaled nearest-neighbour before Rune's bilinear sampling sees them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use rune::{Image, ImageCache};

use crate::core::assets;
use crate::core::fx::Fx;

/// Images up to this size (either side) are pixel art: scaled up by whole numbers on the CPU.
const PIXEL_ART_MAX: u32 = 160;
const UPSCALE_TARGET: u32 = 256;

enum Slot {
    Loading,
    Ready(Image),
    Failed,
}

#[derive(Default)]
pub struct Images {
    cache: Option<Arc<ImageCache>>,
    slots: RefCell<HashMap<String, Slot>>,
}

impl Images {
    pub fn init(&mut self, cache: ImageCache) {
        self.cache = Some(Arc::new(cache));
    }

    /// The image at `src`, once it has loaded; `None` while loading or when it failed.
    pub fn get(&self, fx: &Fx, src: &str) -> Option<Image> {
        if src.is_empty() {
            return None;
        }
        if let Some(slot) = self.slots.borrow().get(src) {
            return match slot {
                Slot::Ready(img) if img.error().is_none() => Some(img.clone()),
                _ => None,
            };
        }
        self.start(fx, src);
        None
    }

    /// True when `src` failed to load (so a view can show its fallback rather than a blank).
    pub fn failed(&self, src: &str) -> bool {
        match self.slots.borrow().get(src) {
            Some(Slot::Failed) => true,
            Some(Slot::Ready(img)) => img.error().is_some(),
            _ => false,
        }
    }

    /// Forget `src` so the next `get` loads it again (an icon file that changed on disk).
    pub fn invalidate(&self, src: &str) {
        self.slots.borrow_mut().remove(src);
    }

    fn start(&self, fx: &Fx, src: &str) {
        self.slots.borrow_mut().insert(src.to_string(), Slot::Loading);
        let key = src.to_string();
        let source = src.to_string();
        let owned_fx = fx.clone();
        let fetch = async move {
            let bytes: Option<Vec<u8>> = if let Some(rest) = source.strip_prefix("data:") {
                let (meta, data) = rest.split_once(',').unwrap_or(("", ""));
                if meta.ends_with(";base64") {
                    base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()
                } else {
                    Some(data.as_bytes().to_vec())
                }
            } else if source.starts_with("http://") || source.starts_with("https://") {
                match crate::http::client() {
                    Ok(client) => match client.get(&source).send().await {
                        Ok(response) if response.status().is_success() => response.bytes().await.ok().map(|body| body.to_vec()),
                        _ => None,
                    },
                    Err(_) => None,
                }
            } else if let Some(bytes) = assets::file(&source) {
                Some(bytes.to_vec())
            } else {
                std::fs::read(source.trim_start_matches("file://")).ok()
            };
            let decoded = bytes.map(|bytes| {
                let pixels = tokio::task::block_in_place(|| decode(&bytes));
                (bytes, pixels)
            });
            decoded
        };
        owned_fx.spawn(fetch, move |app, out| {
            let slot = match (out, app.images.cache.as_ref()) {
                (Some((_, Some((w, h, rgba)))), Some(cache)) => Slot::Ready(cache.rgba(w, h, rgba)),
                // Formats we don't decode ourselves (SVG, ICNS…) go to Rune as they are.
                (Some((bytes, None)), Some(cache)) => Slot::Ready(cache.load(bytes)),
                _ => Slot::Failed,
            };
            app.images.slots.borrow_mut().insert(key, slot);
        });
    }
}

/// RGBA, nearest-neighbour upscaled when it's small pixel art.
fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    if w.max(h) > PIXEL_ART_MAX || w == 0 || h == 0 {
        return Some((w, h, img.into_raw()));
    }
    let scale = (UPSCALE_TARGET / w.max(h)).clamp(1, 16);
    let upscaled = image::imageops::resize(&img, w * scale, h * scale, image::imageops::FilterType::Nearest);
    Some((w * scale, h * scale, upscaled.into_raw()))
}
