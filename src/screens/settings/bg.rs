//! `src/lib/background.ts` for the page's previews: the tagged background string parsed
//! (`parsePattern`, `parseImage`, `parseTexture`, `bgKind`) and drawn small, as `backgroundCss`
//! and the `anim-*` classes of app.css paint it (animated ones as their still frame).

use rune::{Color, El, Shape, SurfaceFit, el};

use crate::app::App;
use crate::theme::{EMBER_STOPS, NEBULA_STOPS, TIDE_STOPS, Theme, parse_color};

pub const PATTERNS: [&str; 4] = ["dots", "grid", "diagonal", "checker"];
pub const DEFAULT_COLOR: &str = "#17161a";

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Kind {
    Default,
    Color,
    Pattern,
    Image,
    Tile,
    Texture,
    Anim,
}

pub fn kind(bg: &str) -> Kind {
    if bg.starts_with("color:") {
        Kind::Color
    } else if bg.starts_with("pattern:") {
        Kind::Pattern
    } else if bg.starts_with("image:") {
        Kind::Image
    } else if bg.starts_with("tile:") {
        Kind::Tile
    } else if bg.starts_with("texture:") {
        Kind::Texture
    } else if bg.starts_with("anim:") {
        Kind::Anim
    } else {
        Kind::Default
    }
}

/// `pattern:<name>[|#color]`.
pub fn parse_pattern(bg: &str) -> (String, Option<String>) {
    let rest = bg.strip_prefix("pattern:").unwrap_or("");
    match rest.split_once('|') {
        None => (rest.to_string(), None),
        Some((name, color)) => (name.to_string(), (!color.is_empty()).then(|| color.to_string())),
    }
}

fn parse_prefixed(bg: &str, prefix: &str) -> (String, Option<String>) {
    let rest = bg.strip_prefix(prefix).unwrap_or("");
    if rest.starts_with('#') {
        if let Some(i) = rest.find('|').filter(|&i| i > 0) {
            return (rest[i + 1..].to_string(), Some(rest[..i].to_string()));
        }
    }
    (rest.to_string(), None)
}

/// `image:[#color|]<uri>` as (uri, color).
pub fn parse_image(bg: &str) -> (String, Option<String>) {
    parse_prefixed(bg, "image:")
}

/// `texture:[<opacity>|][#color|]<uri>` as (uri, color, opacity).
pub fn parse_texture(bg: &str) -> (String, Option<String>, f32) {
    let mut rest = bg.strip_prefix("texture:").unwrap_or("");
    let mut opacity = 0.5;
    if let Some((head, tail)) = rest.split_once('|') {
        let ok = (head.starts_with('0') || head.starts_with('1')) && head.parse::<f32>().is_ok_and(|value| (0.0..=1.0).contains(&value));
        if ok {
            opacity = head.parse().unwrap_or(0.5);
            rest = tail;
        }
    }
    let mut color = None;
    if rest.starts_with('#') {
        if let Some(i) = rest.find('|').filter(|&i| i > 0) {
            color = Some(rest[..i].to_string());
            rest = &rest[i + 1..];
        }
    }
    (rest.to_string(), color, opacity)
}

pub fn is_hex(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

fn hex_or_default(value: &str) -> Color {
    parse_color(value).unwrap_or_else(|| parse_color(DEFAULT_COLOR).unwrap_or(Color::BLACK))
}

/// `linear-gradient(160deg, #a 0%, #b 45%, #c 100%)` (stops without a position spread evenly).
fn parse_gradient(css: &str) -> Option<(f32, Vec<(f32, Color)>)> {
    let inner = css.trim().strip_prefix("linear-gradient(")?.strip_suffix(')')?;
    let mut parts = inner.split(',').map(str::trim);
    let angle = parts.next()?.strip_suffix("deg")?.trim().parse::<f32>().ok()?;
    let raw: Vec<(Color, Option<f32>)> = parts
        .filter_map(|stop| {
            let mut tokens = stop.split_whitespace();
            let color = parse_color(tokens.next()?)?;
            let pos = tokens.next().and_then(|token| token.strip_suffix('%')).and_then(|number| number.parse::<f32>().ok()).map(|percent| percent / 100.0);
            Some((color, pos))
        })
        .collect();
    if raw.len() < 2 {
        return None;
    }
    let n = raw.len() - 1;
    Some((angle.to_radians(), raw.into_iter().enumerate().map(|(i, (color, pos))| (pos.unwrap_or(i as f32 / n as f32), color)).collect()))
}

/// How far a pattern is drawn; swatches clip it.
const AREA: [f32; 2] = [260.0, 120.0];

fn grid_lines(step: f32, color: Color) -> Shape {
    let mut shape = Shape::new();
    let mut x = 0.0;
    while x < AREA[0] {
        shape = shape.fill(vec![[x, 0.0], [x + 1.0, 0.0], [x + 1.0, AREA[1]], [x, AREA[1]]], color);
        x += step;
    }
    let mut y = 0.0;
    while y < AREA[1] {
        shape = shape.fill(vec![[0.0, y], [AREA[0], y], [AREA[0], y + 1.0], [0.0, y + 1.0]], color);
        y += step;
    }
    shape
}

fn dots(step: f32, radius: f32, offset: [f32; 2], color: Color) -> Shape {
    let mut shape = Shape::new();
    let mut y = offset[1];
    while y < AREA[1] + step {
        let mut x = offset[0];
        while x < AREA[0] + step {
            shape = shape.fill(vec![[x - radius, y - radius], [x + radius, y - radius], [x + radius, y + radius], [x - radius, y + radius]], color);
            x += step;
        }
        y += step;
    }
    shape
}

/// `repeating-linear-gradient(<deg>, color 0 <on>, transparent <on> <period>)`.
fn stripes(deg: f32, on: f32, period: f32, color: Color) -> Shape {
    let angle = deg.to_radians();
    let dir = [angle.sin(), -angle.cos()];
    let along = [-dir[1], dir[0]];
    let reach = AREA[0] + AREA[1];
    let mut shape = Shape::new();
    let mut offset = -reach;
    while offset < reach {
        let quad = [offset, offset + on, offset + on, offset];
        let ends = [-reach, -reach, reach, reach];
        let points: Vec<[f32; 2]> = (0..4)
            .map(|i| {
                let (across, along_dist) = (quad[[0, 1, 1, 0][i]], ends[[0, 0, 1, 1][i]]);
                [dir[0] * across + along[0] * along_dist, dir[1] * across + along[1] * along_dist]
            })
            .collect();
        shape = shape.fill(points, color);
        offset += period;
    }
    shape
}

fn checker(size: f32, color: Color) -> Shape {
    let half = size / 2.0;
    let mut shape = Shape::new();
    let mut y = 0.0;
    while y < AREA[1] {
        let mut x = 0.0;
        while x < AREA[0] {
            for (ox, oy) in [(half, 0.0), (0.0, half)] {
                let (x0, y0) = (x + ox, y + oy);
                shape = shape.fill(vec![[x0, y0], [x0 + half, y0], [x0 + half, y0 + half], [x0, y0 + half]], color);
            }
            x += size;
        }
        y += size;
    }
    shape
}

fn pattern(theme: &Theme, name: &str) -> Option<Shape> {
    Some(match name {
        "dots" => dots(18.0, 1.5, [9.0, 9.0], theme.border),
        "grid" => grid_lines(22.0, theme.border_subtle),
        "diagonal" => stripes(45.0, 10.0, 20.0, theme.border_subtle),
        "checker" => checker(28.0, theme.border_subtle),
        _ => return None,
    })
}

fn layer(shape: Shape) -> El {
    el().size(AREA[0], AREA[1]).absolute(0.0, 0.0).shape(shape)
}

/// A box painted as `bg` would paint the window, inside a 2 pt `border`. Size the result.
pub fn framed(app: &App, theme: &Theme, bg: &str, border: Color) -> El {
    el().border(2.0, border).child(paint(app, theme, bg).w_full().h_full())
}

fn paint(app: &App, theme: &Theme, bg: &str) -> El {
    let base = el().clip_corners([0.0; 4]).fill(theme.bg_app);
    match kind(bg) {
        Kind::Default => base,
        Kind::Color => {
            let css = &bg[6..];
            if let Some((angle, stops)) = parse_gradient(css) {
                base.linear_gradient(angle, stops)
            } else if css.is_empty() {
                base
            } else {
                base.fill(hex_or_default(css))
            }
        }
        Kind::Pattern => {
            let (name, color) = parse_pattern(bg);
            let painted = match color {
                Some(color) => base.fill(hex_or_default(&color)),
                None => base,
            };
            match pattern(theme, &name) {
                Some(shape) => painted.child(layer(shape)),
                None => painted,
            }
        }
        Kind::Image | Kind::Tile | Kind::Texture => {
            let (uri, color, scrim) = match kind(bg) {
                Kind::Image => {
                    let (uri, color) = parse_image(bg);
                    (uri, color, 0.55)
                }
                Kind::Tile => {
                    let (uri, color) = parse_prefixed(bg, "tile:");
                    (uri, color, 0.0)
                }
                _ => {
                    let (uri, color, opacity) = parse_texture(bg);
                    (uri, color, (1.0 - opacity).clamp(0.0, 1.0))
                }
            };
            let tint = hex_or_default(color.as_deref().unwrap_or(DEFAULT_COLOR));
            let mut painted = base.fill(tint);
            if let Some(img) = app.images.get(&app.fx, &uri) {
                painted = painted.child(el().w_full().h_full().absolute(0.0, 0.0).pixels_fit(img.pixels(), SurfaceFit::Fill));
            }
            painted.child(el().w_full().h_full().absolute(0.0, 0.0).fill(tint.alpha(scrim)))
        }
        Kind::Anim => anim(theme, base, &bg[5..]),
    }
}

/// The `anim-*` classes, still.
fn anim(theme: &Theme, base: El, name: &str) -> El {
    let gradient = |stops: [&str; 5]| -> Vec<(f32, Color)> { stops.iter().enumerate().map(|(i, color)| (i as f32 / 4.0, hex_or_default(color))).collect() };
    match name {
        "nebula" => base.linear_gradient(130f32.to_radians(), gradient(NEBULA_STOPS)),
        "ember" => base.linear_gradient(130f32.to_radians(), gradient(EMBER_STOPS)),
        "tide" => base.linear_gradient(130f32.to_radians(), gradient(TIDE_STOPS)),
        "starfield" => base
            .fill(hex_or_default("#080a12"))
            .child(layer(dots(34.0, 1.0, [17.0, 17.0], Color::WHITE.alpha(0.35))))
            .child(layer(dots(52.0, 1.0, [40.0, 35.0], Color::WHITE.alpha(0.2)))),
        "beams" => base.fill(hex_or_default("#0a0f18")).child(layer(stripes(60.0, 3.0, 24.0, hex_or_default("#82b4ff").alpha(0.07)))),
        "circuit" => base.fill(hex_or_default("#0c1016")).child(layer(grid_lines(30.0, theme.accent_soft))),
        "lattice" => base.fill(hex_or_default("#0a0c12")).child(layer(grid_lines(16.0, Color::WHITE.alpha(0.04)))),
        _ => base,
    }
}
