//! `skins.svelte.ts` and `skin.ts`: skins the user set here, kept under `cactus:skinCache` with a
//! rendered face and front body so the UI shows them before Mojang's CDN catches up.

use std::collections::BTreeMap;

use base64::Engine;
use serde::{Deserialize, Serialize};

use super::viewer::Texture;
use crate::app::App;

pub const KEY: &str = "cactus:skinCache";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cached {
    pub full: String,
    pub face: String,
    pub body: String,
}

#[derive(Default)]
pub struct Cache {
    map: BTreeMap<String, Cached>,
}

impl Cache {
    pub fn load(app: &App) -> Cache {
        Cache { map: app.prefs.read(KEY, BTreeMap::new()) }
    }
    pub fn full(&self, uuid: &str) -> Option<&str> {
        self.map.get(uuid).map(|cached| cached.full.as_str())
    }
    pub fn face(&self, uuid: &str) -> Option<&str> {
        self.map.get(uuid).map(|cached| cached.face.as_str())
    }
    pub fn body(&self, uuid: &str) -> Option<&str> {
        self.map.get(uuid).map(|cached| cached.body.as_str())
    }
}

/// `skinCache.setSkin`: render the face and body views off the UI thread, then persist.
pub fn set_skin(app: &mut App, uuid: &str, full: String) {
    let uuid = uuid.to_string();
    app.fx.blocking(
        move |_| {
            let views = texture_from_data_uri(&full).map(|texture| (png_data_uri(&render_face(&texture)), png_data_uri(&render_body(&texture))));
            let (face, body) = views.unwrap_or_else(|| (full.clone(), full.clone()));
            Cached { full, face, body }
        },
        move |app, cached| {
            app.account_modal.skins.map.insert(uuid, cached);
            app.prefs.write(KEY, &app.account_modal.skins.map);
        },
    );
}

pub fn clear(app: &mut App, uuid: &str) {
    if app.account_modal.skins.map.remove(uuid).is_some() {
        app.prefs.write(KEY, &app.account_modal.skins.map);
    }
}

/// `skinFace(uuid, displaySize)`: the cached face, else Minotar's helm render at twice the size.
pub fn skin_face(app: &App, uuid: &str, display: u32) -> String {
    match app.account_modal.skins.face(uuid) {
        Some(face) => face.to_string(),
        None => format!("https://minotar.net/helm/{uuid}/{}.png", display * 2),
    }
}

/// Front faces as `[sx, sy, w, h, dx, dy]` on a 16×32 figure, base then layer.
pub const FRONT: [[usize; 6]; 12] = [
    [8, 8, 8, 8, 4, 0], [40, 8, 8, 8, 4, 0],
    [20, 20, 8, 12, 4, 8], [20, 36, 8, 12, 4, 8],
    [44, 20, 4, 12, 0, 8], [44, 36, 4, 12, 0, 8],
    [36, 52, 4, 12, 12, 8], [52, 52, 4, 12, 12, 8],
    [4, 20, 4, 12, 4, 20], [4, 36, 4, 12, 4, 20],
    [20, 52, 4, 12, 8, 20], [4, 52, 4, 12, 8, 20],
];

/// Back faces (arms and legs swap sides, as seen from behind).
pub const BACK: [[usize; 6]; 12] = [
    [24, 8, 8, 8, 4, 0], [56, 8, 8, 8, 4, 0],
    [32, 20, 8, 12, 4, 8], [32, 36, 8, 12, 4, 8],
    [52, 20, 4, 12, 12, 8], [52, 36, 4, 12, 12, 8],
    [44, 52, 4, 12, 0, 8], [60, 52, 4, 12, 0, 8],
    [12, 20, 4, 12, 8, 20], [12, 36, 4, 12, 8, 20],
    [28, 52, 4, 12, 4, 20], [12, 52, 4, 12, 4, 20],
];

/// Straight-alpha RGBA.
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

impl Img {
    pub fn new(w: usize, h: usize) -> Img {
        Img { w, h, rgba: vec![0; w * h * 4] }
    }

    /// `drawImage` of one 64×64-sheet rect, scaled by `k`, composited source-over.
    pub fn draw(&mut self, src: &[u8], src_w: usize, [sx, sy, w, h, dx, dy]: [usize; 6], k: usize) {
        for y in 0..h * k {
            for x in 0..w * k {
                let si = ((sy + y / k) * src_w + sx + x / k) * 4;
                let (tx, ty) = (dx * k + x, dy * k + y);
                if si + 4 > src.len() || tx >= self.w || ty >= self.h {
                    continue;
                }
                blend_over(&mut self.rgba[(ty * self.w + tx) * 4..][..4], &src[si..si + 4]);
            }
        }
    }
}

pub fn blend_over(dst: &mut [u8], src: &[u8]) {
    let alpha = src[3] as u32;
    if alpha == 0 {
        return;
    }
    if alpha == 255 {
        dst.copy_from_slice(src);
        return;
    }
    let dst_alpha = dst[3] as u32;
    let out_alpha = alpha + dst_alpha * (255 - alpha) / 255;
    for c in 0..3 {
        let blended = (src[c] as u32 * alpha + dst[c] as u32 * dst_alpha * (255 - alpha) / 255) / out_alpha.max(1);
        dst[c] = blended.min(255) as u8;
    }
    dst[3] = out_alpha as u8;
}

/// A sheet scaled back to 64×64 texels (HD skins sample every `unit`th texel).
fn sheet64(texture: &Texture) -> Vec<u8> {
    let u = (texture.w / 64).max(1);
    let mut out = vec![0; 64 * 64 * 4];
    for y in 0..64 {
        for x in 0..64 {
            let si = (y * u * texture.w + x * u) * 4;
            out[(y * 64 + x) * 4..][..4].copy_from_slice(&texture.rgba[si..si + 4]);
        }
    }
    out
}

pub fn render_face(texture: &Texture) -> Img {
    let src = sheet64(texture);
    let mut img = Img::new(64, 64);
    img.draw(&src, 64, [8, 8, 8, 8, 0, 0], 8);
    img.draw(&src, 64, [40, 8, 8, 8, 0, 0], 8);
    img
}

pub fn render_body(texture: &Texture) -> Img {
    let src = sheet64(texture);
    let mut img = Img::new(128, 256);
    for rect in FRONT {
        img.draw(&src, 64, rect, 8);
    }
    img
}

pub fn encode_png(img: &Img) -> Option<Vec<u8>> {
    let buf = image::RgbaImage::from_raw(img.w as u32, img.h as u32, img.rgba.clone())?;
    let mut out = std::io::Cursor::new(Vec::new());
    buf.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

pub fn png_data_uri(img: &Img) -> String {
    bytes_data_uri(&encode_png(img).unwrap_or_default())
}

pub fn bytes_data_uri(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn data_uri_bytes(uri: &str) -> Option<Vec<u8>> {
    let (meta, data) = uri.strip_prefix("data:")?.split_once(',')?;
    if meta.ends_with(";base64") {
        base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()
    } else {
        Some(data.as_bytes().to_vec())
    }
}

pub fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some((w, h, img.into_raw()))
}

pub fn texture_from_data_uri(uri: &str) -> Option<Texture> {
    let (w, h, rgba) = decode(&data_uri_bytes(uri)?)?;
    Texture::skin(w, h, rgba)
}

/// SkinPanel's `cropCapeFront`: the cape's outer face (1,1 sized 10×16, scaled to the sheet).
pub fn cape_front(bytes: &[u8]) -> Option<Img> {
    let (w, h, rgba) = decode(bytes)?;
    let unit = (w as f32 / 64.0).max(f32::MIN_POSITIVE);
    let mut img = Img::new(10, 16);
    for y in 0..16 {
        for x in 0..10 {
            let sx = ((1 + x) as f32 * unit) as usize;
            let sy = ((1 + y) as f32 * unit) as usize;
            if sx < w as usize && sy < h as usize {
                let si = (sy * w as usize + sx) * 4;
                img.rgba[(y * 10 + x) * 4..][..4].copy_from_slice(&rgba[si..si + 4]);
            }
        }
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_composites_the_hat_over_the_face() {
        let mut rgba = vec![0u8; 64 * 64 * 4];
        let put = |rgba: &mut Vec<u8>, x: usize, y: usize, p: [u8; 4]| rgba[(y * 64 + x) * 4..][..4].copy_from_slice(&p);
        put(&mut rgba, 8, 8, [10, 20, 30, 255]);
        put(&mut rgba, 9, 8, [10, 20, 30, 255]);
        put(&mut rgba, 41, 8, [200, 0, 0, 255]);
        // Keep the hat area from being all-opaque.
        let texture = Texture::skin(64, 64, rgba).unwrap();
        let face = render_face(&texture);
        assert_eq!(&face.rgba[..4], &[10, 20, 30, 255]);
        assert_eq!(&face.rgba[8 * 4..8 * 4 + 4], &[200, 0, 0, 255]);
        let body = render_body(&texture);
        assert_eq!((body.w, body.h), (128, 256));
        // The face lands at (4,0) texels on the body.
        assert_eq!(&body.rgba[(4 * 8) * 4..(4 * 8) * 4 + 4], &[10, 20, 30, 255]);
    }

    #[test]
    fn png_round_trips_through_a_data_uri() {
        let mut img = Img::new(2, 1);
        img.rgba.copy_from_slice(&[1, 2, 3, 255, 4, 5, 6, 128]);
        let uri = png_data_uri(&img);
        let (w, h, rgba) = decode(&data_uri_bytes(&uri).unwrap()).unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(rgba, img.rgba);
    }
}
