//! `src/lib/image.ts` and `spriteToSquareIcon` of `decor.ts`: pictures turned into the data URIs
//! instance icons and group covers store. They run off the UI thread.

use std::io::Cursor;

use base64::Engine;
use image::{DynamicImage, RgbaImage, imageops};

/// Cap on an uploaded image before it's read (GIFs are stored as they are).
const MAX_IMAGE_BYTES: u64 = 20_000_000;

pub const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

fn png_uri(img: &RgbaImage) -> Result<String, String> {
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(out)))
}

/// `fileToIconDataUri`: a GIF as is, anything else cover-cropped to a `size` square.
pub fn file_to_icon_data_uri(path: &std::path::Path, size: u32) -> Result<String, String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_IMAGE_BYTES {
        return Err("that image is too large (max 20 MB)".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if image::guess_format(&bytes).ok() == Some(image::ImageFormat::Gif) {
        return Ok(format!("data:image/gif;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
    }
    let img = image::load_from_memory(&bytes).map_err(|_| "could not load image".to_string())?.to_rgba8();
    let (w, h) = img.dimensions();
    let scale = (size as f32 / w as f32).max(size as f32 / h as f32);
    let (sw, sh) = (((w as f32 * scale).round() as u32).max(size), ((h as f32 * scale).round() as u32).max(size));
    let scaled = imageops::resize(&img, sw, sh, imageops::FilterType::Lanczos3);
    let cropped = imageops::crop_imm(&scaled, (sw - size) / 2, (sh - size) / 2, size, size).to_image();
    png_uri(&cropped)
}

/// `spriteToSquareIcon`: the sprite at `url` (a static path) contained in a `size` square at 92%.
pub fn sprite_to_square_icon(url: &str, size: u32) -> Result<String, String> {
    let bytes = crate::core::assets::file(url).ok_or_else(|| format!("{url} not found"))?;
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_rgba8();
    let (w, h) = img.dimensions();
    let scale = (size as f32 / w as f32).min(size as f32 / h as f32) * 0.92;
    let (sw, sh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
    let filter = if w.max(h) * 2 <= sw.max(sh) { imageops::FilterType::Nearest } else { imageops::FilterType::Triangle };
    let scaled = imageops::resize(&img, sw, sh, filter);
    let mut canvas = RgbaImage::new(size, size);
    imageops::overlay(&mut canvas, &scaled, i64::from((size - sw) / 2), i64::from((size - sh) / 2));
    png_uri(&canvas)
}

/// `emojiToDataUri`: the emoji drawn at 70% of a `size` square by Rune's text stack (the system
/// colour emoji font), read back from an offscreen GPU target.
pub fn emoji_to_data_uri(emoji: &str, size: u32) -> Result<String, String> {
    let emoji = emoji.to_string();
    let rgba = std::panic::catch_unwind(move || render_emoji(&emoji, size)).map_err(|_| "could not draw that emoji".to_string())?;
    let img = RgbaImage::from_raw(size, size, rgba).ok_or("could not draw that emoji")?;
    png_uri(&img)
}

thread_local! {
    static EMOJI: std::cell::RefCell<(String, f32)> = const { std::cell::RefCell::new((String::new(), 0.0)) };
}

fn render_emoji(emoji: &str, size: u32) -> Vec<u8> {
    use rune::render::{Gpu, OFFSCREEN_FORMAT, Offscreen, Renderer};
    fn view(_: &(), _: &mut rune::Cx<()>) -> rune::El {
        let (glyph, px) = EMOJI.with(|cell| cell.borrow().clone());
        rune::el()
            .w_full()
            .h_full()
            .center()
            .child(rune::text(glyph).font(crate::theme::sans((px * 0.7).floor(), 400.0)).color(rune::Color::BLACK))
    }
    EMOJI.with(|cell| *cell.borrow_mut() = (emoji.to_string(), size as f32));
    let gpu = Gpu::headless();
    let mut runtime = rune::Runtime::new((), view, (size as f32, size as f32), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    runtime.frame(1.0);
    let mut renderer = Renderer::new(&gpu.device, OFFSCREEN_FORMAT);
    renderer.prepare(&gpu.device, &gpu.queue, &mut runtime.text.atlas, &runtime.instances, (size, size));
    renderer.upload_images(&gpu.device, &gpu.queue, &mut runtime.images);
    let target = Offscreen::new(&gpu.device, size, size);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.draw(&mut encoder, &target.view, [0.0, 0.0, 0.0, 0.0]);
    target.copy(&mut encoder);
    gpu.queue.submit([encoder.finish()]);
    target.read(&gpu.device)
}

#[cfg(test)]
mod tests {
    /// Needs a GPU: `cargo test emoji -- --ignored` (`EMOJI_PNG=out.png` keeps the picture).
    #[test]
    #[ignore]
    fn emoji_renders_to_a_png() {
        let uri = super::emoji_to_data_uri("🐷", 128).unwrap();
        let b64 = uri.strip_prefix("data:image/png;base64,").unwrap();
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert!(img.pixels().any(|p| p[3] > 200 && p[0] > 150), "drew nothing");
        if let Ok(out) = std::env::var("EMOJI_PNG") {
            std::fs::write(out, bytes).unwrap();
        }
    }
}
