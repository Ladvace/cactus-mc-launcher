//! `DecorLayer.svelte` and `DECOR_THEMES` in `src/lib/themes.ts`: sprites pinned round the
//! window edges, above the page and never hit.
//!
//! Sprites are decoded here rather than through `Images`: they are drawn mirrored, and a
//! `drop-shadow` needs a dark copy of their alpha, which Rune draws blurred under them.

use std::cell::RefCell;
use std::collections::HashMap;

use rune::{Cx, El, Pixels, el};

use crate::app::App;

struct Placement {
    sprite: u8,
    /// (top, right, bottom, left): points, or a fraction of the window when `pct`.
    at: [Option<Len>; 4],
    size: f32,
    rotate: f32,
    opacity: f32,
    flip: bool,
}

#[derive(Clone, Copy)]
enum Len {
    Pt(f32),
    Pct(f32),
}

const fn place(sprite: u8, at: [Option<Len>; 4], size: f32) -> Placement {
    Placement { sprite, at, size, rotate: 0.0, opacity: 1.0, flip: false }
}

impl Placement {
    const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
    const fn rotate(mut self, deg: f32) -> Self {
        self.rotate = deg;
        self
    }
    const fn flip(mut self) -> Self {
        self.flip = true;
        self
    }
}

use Len::{Pct, Pt};

const fn top_left(top: Len, left: Len) -> [Option<Len>; 4] {
    [Some(top), None, None, Some(left)]
}
const fn top_right(top: Len, right: Len) -> [Option<Len>; 4] {
    [Some(top), Some(right), None, None]
}
const fn bottom_left(bottom: Len, left: Len) -> [Option<Len>; 4] {
    [None, None, Some(bottom), Some(left)]
}
const fn bottom_right(bottom: Len, right: Len) -> [Option<Len>; 4] {
    [None, Some(right), Some(bottom), None]
}

fn theme_placements(id: &str) -> Option<Vec<Placement>> {
    Some(match id {
        "desert" => vec![
            place(0, bottom_left(Pt(6.0), Pt(8.0)), 118.0),
            place(8, bottom_right(Pt(6.0), Pt(12.0)), 104.0).flip(),
            place(1, top_left(Pt(72.0), Pt(14.0)), 42.0).opacity(0.9),
            place(18, top_right(Pt(78.0), Pt(34.0)), 34.0).opacity(0.85),
            place(6, bottom_left(Pt(8.0), Pct(0.26)), 60.0).opacity(0.8),
        ],
        "bloom" => vec![
            place(1, bottom_left(Pt(8.0), Pt(10.0)), 90.0),
            place(2, bottom_right(Pt(10.0), Pt(14.0)), 78.0),
            place(16, top_left(Pt(72.0), Pt(16.0)), 34.0).rotate(-12.0),
            place(17, top_right(Pt(80.0), Pt(32.0)), 36.0).rotate(10.0),
            place(19, top_right(Pct(0.44), Pt(16.0)), 28.0).opacity(0.8),
        ],
        "mystic" => vec![
            place(22, bottom_left(Pt(8.0), Pt(10.0)), 92.0),
            place(12, bottom_right(Pt(8.0), Pt(14.0)), 92.0).flip(),
            place(10, top_left(Pt(74.0), Pt(16.0)), 46.0).opacity(0.95),
            place(18, top_right(Pt(78.0), Pt(36.0)), 30.0).opacity(0.85),
            place(14, top_left(Pct(0.46), Pt(14.0)), 26.0).opacity(0.8),
        ],
        _ => return None,
    })
}

/// A sprite at its drawn size (device pixels), and its shadow: its alpha at 45% black.
struct Sprite {
    image: Pixels,
    shadow: Pixels,
    aspect: f32,
}

#[derive(Default)]
pub struct State {
    sprites: RefCell<HashMap<(u8, u32, bool), Option<Sprite>>>,
}

fn load(id: u8, width: u32, flip: bool) -> Option<Sprite> {
    let bytes = crate::core::assets::file(&format!("/decor/sprites/{id:02}.png"))?;
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let aspect = h as f32 / w.max(1) as f32;
    let scaled_w = width.clamp(1, w.max(1));
    let scaled_h = ((scaled_w as f32) * aspect).round().max(1.0) as u32;
    let mut img = image::imageops::resize(&img, scaled_w, scaled_h, image::imageops::FilterType::Triangle);
    if flip {
        image::imageops::flip_horizontal_in_place(&mut img);
    }
    let shadow: Vec<u8> = img.pixels().flat_map(|pixel| [0, 0, 0, (pixel[3] as f32 * 0.45) as u8]).collect();
    let (image, shadow_image) = (Pixels::new(), Pixels::new());
    image.set(scaled_w, scaled_h, img.into_raw());
    shadow_image.set(scaled_w, scaled_h, shadow);
    Some(Sprite { image, shadow: shadow_image, aspect })
}

pub fn view(app: &App, cx: &mut Cx<App>) -> Option<El> {
    let placements = theme_placements(app.settings.decor_theme.trim())?;
    let [win_w, win_h] = super::window_size(cx);
    let scale = cx.env().scale.max(1.0);
    let mut cache = app.extras.decor.sprites.borrow_mut();
    let mut root = el().key("decor-layer").w_full().h_full().absolute(0.0, 0.0).a11y_hidden().inert();
    for (i, placement) in placements.iter().enumerate() {
        let width_px = (placement.size * scale).round() as u32;
        let Some(sprite) = cache.entry((placement.sprite, width_px, placement.flip)).or_insert_with(|| load(placement.sprite, width_px, placement.flip)) else { continue };
        let h = placement.size * sprite.aspect;
        let resolve = |len: Option<Len>, of: f32| len.map(|len| match len {
            Pt(points) => points,
            Pct(fraction) => fraction * of,
        });
        let [top, right, bottom, left] = placement.at;
        let (top, right, bottom, left) = (resolve(top, win_h), resolve(right, win_w), resolve(bottom, win_h), resolve(left, win_w));
        // `drop-shadow(0 2px 5px rgba(0,0,0,.45))`: the alpha, 2 pt down, blurred σ 2.5.
        let stack = el()
            .size(placement.size, h)
            .child(el().absolute(0.0, 2.0).size(placement.size, h).pixels(&sprite.shadow).blur(2.5))
            .child(el().absolute(0.0, 0.0).size(placement.size, h).pixels(&sprite.image));
        let mut pinned = stack.key(format!("decor-{i}")).pin(left, top, right, bottom).opacity(placement.opacity);
        if placement.rotate != 0.0 {
            pinned = pinned.rotate(placement.rotate.to_radians());
        }
        root = root.child(pinned);
    }
    Some(root)
}
