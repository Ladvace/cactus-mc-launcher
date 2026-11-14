//! The window background: `src/lib/background.ts` (`backgroundCss` / `animClass`), the `anim-*`
//! keyframes of app.css, `ReactiveGrid.svelte` (the Circuit cursor trail) and `NoiseGrid.svelte`
//! (the Lattice noise field).
//!
//! Static forms are fills, Rune gradients and vector shapes; the drifting presets loop with
//! `LoopSpec::slide`; the cursor trail is a pool of cells moved by `Live` values without view
//! re-runs; the noise field is a `Pixels` image a background thread redraws at ~30 fps.

use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use base64::Engine;
use rune::{Color, Cx, Easing, El, LineCap, Live, Pixels, Shape, Spring, SurfaceFit, el};
use rune_motion::LoopSpec;

use crate::app::App;
use crate::theme::{EMBER_STOPS, NEBULA_STOPS, TIDE_STOPS, Theme, hex_color};

/// The background element's key; its size is the window's.
pub const KEY: &str = "bg-layer";

/// `DEFAULT_COLOR`: `--bg-app`.
const DEFAULT_COLOR: &str = "#17161a";

pub struct State {
    trail: RefCell<Trail>,
    noise: RefCell<Option<Noise>>,
    /// Decoded `texture:` sources, and the last tiling of one at a window size.
    decoded: RefCell<HashMap<String, Option<Rc<image::RgbaImage>>>>,
    tiled: RefCell<Option<((String, u32, u32), Pixels)>>,
}

impl Default for State {
    fn default() -> Self {
        State { trail: RefCell::new(Trail::new()), noise: RefCell::new(None), decoded: Default::default(), tiled: RefCell::new(None) }
    }
}

// Parsing (background.ts).

fn split_once_pipe(rest: &str) -> (Option<&str>, &str) {
    if rest.starts_with('#') {
        if let Some(i) = rest.find('|').filter(|&i| i > 0) {
            return (Some(&rest[..i]), &rest[i + 1..]);
        }
    }
    (None, rest)
}

/// `parseTexture`: `[<opacity>|][#color|]<uri>`.
fn parse_texture(rest: &str) -> (Option<&str>, &str, f32) {
    let mut opacity = 0.5;
    let mut rest = rest;
    if let Some((head, tail)) = rest.split_once('|') {
        let valid = head.parse::<f32>().ok().filter(|alpha| (0.0..=1.0).contains(alpha) && (head.starts_with('0') || head.starts_with('1')));
        if let Some(alpha) = valid {
            opacity = alpha;
            rest = tail;
        }
    }
    let (color, uri) = split_once_pipe(rest);
    (color, uri, opacity)
}

/// `hexToRgba`: six-digit hex only, else the default app colour.
fn hex6(s: &str, alpha: f32) -> Color {
    let h = s.trim().trim_start_matches('#');
    let ok = h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit());
    rune::color::hex(&format!("#{}", if ok { h } else { &DEFAULT_COLOR[1..] })).unwrap_or(Color::BLACK).alpha(alpha)
}

/// Top-level comma split (commas inside parentheses stay).
fn split_args(input: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0i32, 0);
    for (i, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(input[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(input[start..].trim());
    out
}

/// A CSS colour: hex, `rgb()`/`rgba()`, `transparent`, or one of app.css's tokens.
fn css_color(css: &str, theme: &Theme) -> Option<Color> {
    let css = css.trim();
    if css.starts_with('#') {
        return rune::color::hex(css);
    }
    if css == "transparent" {
        return Some(Color::TRANSPARENT);
    }
    if let Some(inner) = css.strip_prefix("var(").and_then(|rest| rest.strip_suffix(')')) {
        return Some(match inner.trim() {
            "--bg-app" => theme.bg_app,
            "--bg-raised" => theme.bg_raised,
            "--bg-card" => theme.bg_card,
            "--border" => theme.border,
            "--border-subtle" => theme.border_subtle,
            "--accent" => theme.accent,
            "--accent-soft" => theme.accent_soft,
            "--text" => theme.text,
            _ => return None,
        });
    }
    let inner = css.strip_prefix("rgba(").or_else(|| css.strip_prefix("rgb("))?.strip_suffix(')')?;
    let parts: Vec<f32> = inner.split([',', ' ', '/']).filter(|part| !part.is_empty()).filter_map(|part| part.trim().parse().ok()).collect();
    if parts.len() < 3 {
        return None;
    }
    let hex = format!("#{:02x}{:02x}{:02x}", parts[0].clamp(0.0, 255.0) as u8, parts[1].clamp(0.0, 255.0) as u8, parts[2].clamp(0.0, 255.0) as u8);
    rune::color::hex(&hex).map(|color| color.alpha(parts.get(3).copied().unwrap_or(1.0)))
}

/// `linear-gradient(<angle> | to <side>, <colour> [<pos>]…)` as Rune's angle (radians) and stops.
fn css_linear_gradient(css: &str, theme: &Theme, w: f32, h: f32) -> Option<(f32, Vec<(f32, Color)>)> {
    let inner = css.trim().strip_prefix("linear-gradient(")?.strip_suffix(')')?;
    let mut args = split_args(inner);
    let mut angle = PI;
    let first = args.first()?.to_string();
    let parsed_angle = if let Some(side) = first.strip_prefix("to ") {
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        for word in side.split_whitespace() {
            match word {
                "top" => dy = -1.0,
                "bottom" => dy = 1.0,
                "left" => dx = -1.0,
                "right" => dx = 1.0,
                _ => {}
            }
        }
        // A corner: perpendicular to the diagonal that doesn't touch it.
        let (dx, dy) = if dx != 0.0 && dy != 0.0 { (dx * h, dy * w) } else { (dx, dy) };
        Some(dx.atan2(-dy))
    } else if let Some(num) = first.strip_suffix("deg") {
        num.trim().parse::<f32>().ok().map(f32::to_radians)
    } else if let Some(num) = first.strip_suffix("turn") {
        num.trim().parse::<f32>().ok().map(|num| num * 2.0 * PI)
    } else if let Some(num) = first.strip_suffix("rad") {
        num.trim().parse::<f32>().ok()
    } else {
        None
    };
    if let Some(parsed) = parsed_angle {
        angle = parsed;
        args.remove(0);
    }
    let len = (w * angle.sin()).abs() + (h * angle.cos()).abs();
    let mut stops: Vec<(Option<f32>, Color)> = Vec::new();
    for arg in args {
        // The colour is everything before a trailing position.
        let (color_css, pos) = match arg.rfind(|c: char| c == ' ').filter(|_| !arg.ends_with(')')) {
            Some(i) => (&arg[..i], Some(&arg[i + 1..])),
            None => (arg, None),
        };
        let color = css_color(color_css, theme)?;
        let pos = pos.and_then(|spec| {
            if let Some(num) = spec.strip_suffix('%') {
                num.parse::<f32>().ok().map(|num| num / 100.0)
            } else if let Some(num) = spec.strip_suffix("px") {
                num.parse::<f32>().ok().map(|num| num / len.max(1.0))
            } else {
                None
            }
        });
        stops.push((pos, color));
    }
    if stops.is_empty() {
        return None;
    }
    let n = stops.len();
    if stops[0].0.is_none() {
        stops[0].0 = Some(0.0);
    }
    if stops[n - 1].0.is_none() {
        stops[n - 1].0 = Some(1.0);
    }
    // Unpositioned stops spread evenly between their positioned neighbours.
    let mut i = 0;
    while i < n {
        if stops[i].0.is_none() {
            let prev = i - 1;
            let next = (i..n).find(|&j| stops[j].0.is_some()).unwrap_or(n - 1);
            let (prev_pos, next_pos) = (stops[prev].0.unwrap_or(0.0), stops[next].0.unwrap_or(1.0));
            for (k, j) in (prev + 1..next).enumerate() {
                stops[j].0 = Some(prev_pos + (next_pos - prev_pos) * (k + 1) as f32 / (next - prev) as f32);
            }
            i = next;
        }
        i += 1;
    }
    Some((angle, stops.into_iter().take(16).map(|(pos, color)| (pos.unwrap_or(0.0).clamp(0.0, 1.0), color)).collect()))
}

// Shapes for the patterns.

fn grid_lines(w: f32, h: f32, step: f32, color: Color) -> Shape {
    let mut shape = Shape::new();
    let mut y = 0.0;
    while y < h {
        shape = shape.line([0.0, y + 0.5], [w, y + 0.5], 1.0, LineCap::Butt, color);
        y += step;
    }
    let mut x = 0.0;
    while x < w {
        shape = shape.line([x + 0.5, 0.0], [x + 0.5, h], 1.0, LineCap::Butt, color);
        x += step;
    }
    shape
}

/// A dot of `radius` at the centre of each `step` tile, the tiles starting at `origin`.
fn dots(w: f32, h: f32, step: f32, origin: [f32; 2], radius: f32, color: Color) -> Shape {
    let mut shape = Shape::new();
    let x0 = origin[0].rem_euclid(step) - step;
    let y0 = origin[1].rem_euclid(step) - step;
    let mut y = y0 + step / 2.0;
    while y < h + step {
        let mut x = x0 + step / 2.0;
        while x < w + step {
            shape = shape.line([x - 0.01, y], [x + 0.01, y], radius * 2.0, LineCap::Round, color);
            x += step;
        }
        y += step;
    }
    shape
}

/// Longest piece a stripe is drawn in. Rune rasterises every shape piece over its bounding
/// box, so one diagonal spanning the window costs a full-window pass; short pieces keep the
/// boxes close to the stripe.
const STRIPE_PIECE: f32 = 24.0;

/// `repeating-linear-gradient(<deg>, color 0 <band>, transparent <band> <period>)` over w×h.
fn stripes(w: f32, h: f32, deg: f32, band: f32, period: f32, color: Color) -> Shape {
    let angle = deg.to_radians();
    let dir = [angle.sin(), -angle.cos()];
    let normal = [-dir[1], dir[0]];
    // The rect's extent across the stripes (along `dir`) and along them (along `normal`).
    let across = (w * dir[0]).abs() + (h * dir[1]).abs();
    let along = (w * normal[0]).abs() + (h * normal[1]).abs() + band;
    let centre = [w / 2.0, h / 2.0];
    let at = |t: f32, s: f32| [centre[0] + dir[0] * t + normal[0] * s, centre[1] + dir[1] * t + normal[1] * s];
    let pieces = (along / STRIPE_PIECE).ceil().max(1.0) as usize;
    let piece = along / pieces as f32;
    let mut shape = Shape::new();
    let mut t = -across / 2.0;
    while t < across / 2.0 {
        // The band's centre line, cut into butt-capped pieces that meet edge to edge.
        let mid = t + band / 2.0;
        for i in 0..pieces {
            let s0 = -along / 2.0 + piece * i as f32;
            shape = shape.line(at(mid, s0), at(mid, s0 + piece), band, LineCap::Butt, color);
        }
        t += period;
    }
    shape
}

/// `repeating-conic-gradient(color 0% 25%, base 0% 50%) 0 0 / tile tile`: the top-right and
/// bottom-left quarter of every tile.
fn checker(w: f32, h: f32, tile: f32, color: Color) -> Shape {
    let half = tile / 2.0;
    let mut shape = Shape::new();
    let mut y = 0.0;
    while y < h {
        let mut x = 0.0;
        while x < w {
            for [qx, qy] in [[x + half, y], [x, y + half]] {
                shape = shape.fill(vec![[qx, qy], [qx + half, qy], [qx + half, qy + half], [qx, qy + half]], color);
            }
            x += tile;
        }
        y += tile;
    }
    shape
}

fn layer(w: f32, h: f32) -> El {
    el().absolute(0.0, 0.0).size(w, h)
}

/// `patternCss`.
fn pattern(theme: &Theme, name: &str, w: f32, h: f32) -> Option<El> {
    Some(match name {
        "dots" => layer(w, h).shape(dots(w, h, 18.0, [0.0, 0.0], 1.5, theme.border)),
        "grid" => layer(w, h).shape(grid_lines(w, h, 22.0, theme.border_subtle)),
        "diagonal" => layer(w, h).shape(stripes(w, h, 45.0, 10.0, 20.0, theme.border_subtle)),
        "checker" => layer(w, h).shape(checker(w, h, 28.0, theme.border_subtle)),
        _ => return None,
    })
}

// Animated presets (app.css `.anim-*`).

/// `bg-drift`'s 0% → 100% → 0%, each half eased, as one looping curve.
static PING_PONG: LazyLock<&'static [[f32; 2]]> = LazyLock::new(|| {
    let n = 48;
    let points: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let x = i as f32 / n as f32;
            let half = if x <= 0.5 { x * 2.0 } else { (1.0 - x) * 2.0 };
            [x, Easing::EASE_IN_OUT.eval(half)]
        })
        .collect();
    Box::leak(points.into_boxed_slice())
});

/// `.anim-nebula` / `-ember` / `-tide`: a 300% gradient whose position drifts across and back.
fn drift(w: f32, h: f32, colors: [&str; 5], period: f64, still: bool) -> El {
    let stops: Vec<(f32, Color)> = colors.iter().enumerate().map(|(i, c)| (i as f32 / 4.0, hex_color(c))).collect();
    let gradient = el().absolute(0.0, -h).size(w * 3.0, h * 3.0).linear_gradient(130f32.to_radians(), stops);
    if still {
        return gradient;
    }
    gradient.looping(LoopSpec::slide(period, [0.0, 0.0], [-2.0 / 3.0, 0.0]).easing(Easing::Piecewise(*PING_PONG)))
}

/// A layer that slides down by `travel` each `period`, seamless because `travel` is a whole
/// number of its tiles.
fn falling(w: f32, h: f32, travel: f32, period: f64, still: bool, shape: Shape) -> El {
    let strip = el().absolute(0.0, -travel).size(w, h + travel).shape(shape);
    if still {
        return strip;
    }
    strip.looping(LoopSpec::slide(period, [0.0, 0.0], [0.0, travel / (h + travel)]).easing(Easing::LINEAR))
}

fn anim(app: &App, theme: &Theme, name: &str, w: f32, h: f32, still: bool, root: El) -> El {
    match name {
        "nebula" => root.child(drift(w, h, NEBULA_STOPS, 18.0, still)),
        "ember" => root.child(drift(w, h, EMBER_STOPS, 15.0, still)),
        "tide" => root.child(drift(w, h, TIDE_STOPS, 21.0, still)),
        "starfield" => root
            .fill(hex_color("#080a12"))
            .child(falling(w, h, 340.0, 26.0, still, dots(w, h + 340.0, 34.0, [0.0, 0.0], 1.5, Color::WHITE.alpha(0.35))))
            .child(falling(w, h, 520.0, 26.0, still, dots(w, h + 520.0, 52.0, [14.0, 9.0], 1.5, Color::WHITE.alpha(0.2)))),
        "beams" => {
            let beams = el()
                .absolute(0.0, 0.0)
                // Twice as wide for the sideways slide; it never moves vertically.
                .size(w * 2.0, h)
                .shape(stripes(w * 2.0, h, 60.0, 3.0, 24.0, hex_color("#82b4ff").alpha(0.07)));
            let beams = if still { beams } else { beams.looping(LoopSpec::slide(14.0, [0.0, 0.0], [-0.5, 0.0]).easing(Easing::LINEAR)) };
            root.fill(hex_color("#0a0f18")).child(beams)
        }
        "circuit" => {
            let root = root.fill(hex_color("#0c1016")).child(layer(w, h).shape(grid_lines(w, h, 30.0, theme.accent_soft)));
            if still { root } else { reactive_grid(app, theme, root) }
        }
        "lattice" => {
            let root = root.fill(hex_color("#0a0c12")).child(layer(w, h).shape(grid_lines(w, h, 16.0, Color::WHITE.alpha(0.04))));
            if still {
                *app.extras.bg.noise.borrow_mut() = None;
                root
            } else {
                root.child(noise_grid(app, theme, w, h))
            }
        }
        _ => root,
    }
}

// ReactiveGrid: the cell under the pointer lights and fades over 0.7 s, leaving a trail.

const TRAIL_CELL: f32 = 30.0;
const TRAIL_CELLS: usize = 40;
/// How long a lit cell holds before fading, so a pointer at rest still shows where it is.
const TRAIL_HOLD: Duration = Duration::from_millis(60);

struct TrailCell {
    rect: Live<[f32; 4]>,
    alpha: Live<f32>,
}

struct Trail {
    cells: Vec<TrailCell>,
    next: usize,
    last: Option<(i32, i32)>,
}

impl Trail {
    fn new() -> Trail {
        let cells = (0..TRAIL_CELLS)
            .map(|_| TrailCell { rect: Live::new([0.0, 0.0, TRAIL_CELL, TRAIL_CELL]), alpha: Live::new(0.0) })
            .collect();
        Trail { cells, next: 0, last: None }
    }

    /// `onMove`: re-light the current cell, or light the next one from the pool.
    fn moved(&mut self, x: f32, y: f32) {
        let cell = ((x / TRAIL_CELL).floor() as i32, (y / TRAIL_CELL).floor() as i32);
        if self.last != Some(cell) {
            self.last = Some(cell);
            self.next = (self.next + 1) % self.cells.len();
            let lit = &self.cells[self.next];
            lit.rect.set([cell.0 as f32 * TRAIL_CELL, cell.1 as f32 * TRAIL_CELL, TRAIL_CELL, TRAIL_CELL]);
        }
        self.cells[self.next].alpha.pulse(1.0, 0.0, TRAIL_HOLD);
    }
}

fn reactive_grid(app: &App, theme: &Theme, root: El) -> El {
    let fill = theme.accent.alpha(0.45);
    let trail = app.extras.bg.trail.borrow();
    let cells = trail.cells.iter().enumerate().map(|(i, cell)| {
        el().key(format!("bg-trail-{i}"))
            .absolute(0.0, 0.0)
            .size(TRAIL_CELL, TRAIL_CELL)
            .fill(fill)
            .live_rect(&cell.rect)
            .live_opacity(cell.alpha.smooth(Spring::new(0.02, 0.0)).rest(Spring::new(0.7, 0.0)))
            .inert()
    });
    root.children(cells).on_pointer_move_local(|app: &mut App, pointer, tx| {
        app.extras.bg.trail.borrow_mut().moved(pointer.window_x, pointer.window_y);
        tx.live_only();
    })
}

// NoiseGrid: cells fade in and out following animated 3D value noise.

const NOISE_CELL: usize = 16;
const NOISE_SCALE: f64 = 0.14;
const NOISE_TIME: f64 = 0.00028;
const NOISE_THRESHOLD: f64 = 0.52;
const NOISE_MAX_ALPHA: f64 = 0.5;
const NOISE_FRAME: Duration = Duration::from_millis(33);

struct Noise {
    key: (u32, u32, [u8; 3]),
    pixels: Pixels,
    stop: Arc<AtomicBool>,
}

impl Drop for Noise {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn hash(x: i32, y: i32, z: i32) -> f64 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263)).wrapping_add(z.wrapping_mul(1274126177));
    h = (h ^ ((h as u32 >> 13) as i32)).wrapping_mul(1274126177);
    h ^= (h as u32 >> 16) as i32;
    (h as u32) as f64 / 4294967295.0
}

fn value_noise(x: f64, y: f64, z: f64) -> f64 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let fade = |t: f64| t * t * (3.0 - 2.0 * t);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let (u, v, w) = (fade(x - xi), fade(y - yi), fade(z - zi));
    let (xi, yi, zi) = (xi as i32, yi as i32, zi as i32);
    let corner = |i, j, k| hash(xi + i, yi + j, zi + k);
    lerp(
        lerp(lerp(corner(0, 0, 0), corner(1, 0, 0), u), lerp(corner(0, 1, 0), corner(1, 1, 0), u), v),
        lerp(lerp(corner(0, 0, 1), corner(1, 0, 1), u), lerp(corner(0, 1, 1), corner(1, 1, 1), u), v),
        w,
    )
}

/// One frame: lit cells `CELL - 1` square, so the grid lines show between them.
fn noise_frame(w: u32, h: u32, rgb: [u8; 3], z: f64) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut buf = vec![0u8; w * h * 4];
    for row in 0..h.div_ceil(NOISE_CELL) {
        for col in 0..w.div_ceil(NOISE_CELL) {
            let n = value_noise(col as f64 * NOISE_SCALE, row as f64 * NOISE_SCALE, z);
            if n <= NOISE_THRESHOLD {
                continue;
            }
            let alpha = (((n - NOISE_THRESHOLD) / (1.0 - NOISE_THRESHOLD)) * NOISE_MAX_ALPHA * 255.0) as u8;
            let pixel = [rgb[0], rgb[1], rgb[2], alpha];
            for y in row * NOISE_CELL..((row + 1) * NOISE_CELL - 1).min(h) {
                let line = &mut buf[(y * w + col * NOISE_CELL) * 4..(y * w + ((col + 1) * NOISE_CELL - 1).min(w)) * 4];
                for dst in line.chunks_exact_mut(4) {
                    dst.copy_from_slice(&pixel);
                }
            }
        }
    }
    buf
}

fn rgb8(color: Color) -> [u8; 3] {
    let hex_str = color.hex();
    let n = u32::from_str_radix(hex_str.trim_start_matches('#').get(..6).unwrap_or("e8b23a"), 16).unwrap_or(0);
    [(n >> 16) as u8, (n >> 8) as u8, n as u8]
}

fn noise_grid(app: &App, theme: &Theme, w: f32, h: f32) -> El {
    let (pw, ph) = (w.ceil().max(1.0) as u32, h.ceil().max(1.0) as u32);
    let key = (pw, ph, rgb8(theme.accent));
    let mut slot = app.extras.bg.noise.borrow_mut();
    if slot.as_ref().map(|noise| noise.key) != Some(key) {
        let pixels = Pixels::new();
        pixels.set(pw, ph, noise_frame(pw, ph, key.2, 0.0));
        let stop = Arc::new(AtomicBool::new(false));
        let (frame_pixels, stopped) = (pixels.clone(), stop.clone());
        std::thread::Builder::new()
            .name("bg-noise".into())
            .spawn(move || {
                let t0 = Instant::now();
                while !stopped.load(Ordering::Relaxed) {
                    std::thread::sleep(NOISE_FRAME);
                    let z = t0.elapsed().as_secs_f64() * 1000.0 * NOISE_TIME;
                    frame_pixels.set(pw, ph, noise_frame(pw, ph, key.2, z));
                }
            })
            .ok();
        *slot = Some(Noise { key, pixels, stop });
    }
    let pixels = slot.as_ref().map(|noise| noise.pixels.clone()).unwrap_or_default();
    layer(w, h).key("bg-noise").pixels(&pixels).inert()
}

// Images.

/// The bytes of a static path, `data:` URI or local file; None for remote URLs.
fn local_bytes(src: &str) -> Option<Vec<u8>> {
    if let Some(rest) = src.strip_prefix("data:") {
        let (meta, data) = rest.split_once(',')?;
        return if meta.ends_with(";base64") {
            base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()
        } else {
            Some(data.as_bytes().to_vec())
        };
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        return None;
    }
    if let Some(bytes) = crate::core::assets::file(src) {
        return Some(bytes.to_vec());
    }
    std::fs::read(src.trim_start_matches("file://")).ok()
}

/// `url(..) repeat` at the image's own size: one window-sized image of the tiles.
fn texture_tiles(app: &App, uri: &str, w: f32, h: f32) -> Option<El> {
    let bg_state = &app.extras.bg;
    let img = bg_state
        .decoded
        .borrow_mut()
        .entry(uri.to_string())
        .or_insert_with(|| local_bytes(uri).and_then(|bytes| image::load_from_memory(&bytes).ok()).map(|decoded| Rc::new(decoded.to_rgba8())))
        .clone();
    let Some(img) = img else {
        // Remote: tile what the image loader has, at its size.
        let remote = app.images.get(&app.fx, uri)?;
        let [iw, ih] = remote.size()?;
        let (iw, ih) = (iw as f32, ih as f32);
        let mut root = layer(w, h);
        let mut y = 0.0;
        while y < h {
            let mut x = 0.0;
            while x < w {
                root = root.child(el().absolute(x, y).size(iw, ih).pixels(remote.pixels()));
                x += iw;
            }
            y += ih;
        }
        return Some(root);
    };
    let (pw, ph) = (w.ceil().max(1.0) as u32, h.ceil().max(1.0) as u32);
    let key = (uri.to_string(), pw, ph);
    let mut tiled = bg_state.tiled.borrow_mut();
    if tiled.as_ref().map(|(cached_key, _)| cached_key) != Some(&key) {
        let (iw, ih) = img.dimensions();
        let mut buf = vec![0u8; (pw * ph * 4) as usize];
        if iw > 0 && ih > 0 {
            let src = img.as_raw();
            for y in 0..ph {
                let sy = y % ih;
                let row = &src[(sy * iw * 4) as usize..((sy + 1) * iw * 4) as usize];
                let dst = &mut buf[(y * pw * 4) as usize..((y + 1) * pw * 4) as usize];
                for chunk in dst.chunks_mut((iw * 4) as usize) {
                    chunk.copy_from_slice(&row[..chunk.len()]);
                }
            }
        }
        let pixels = Pixels::new();
        pixels.set(pw, ph, buf);
        *tiled = Some((key, pixels));
    }
    let pixels = tiled.as_ref().map(|(_, pixels)| pixels.clone())?;
    Some(layer(w, h).pixels(&pixels))
}

/// `tile:`: the sprite repeated at 140 pt wide.
fn sprite_tiles(app: &App, uri: &str, w: f32, h: f32) -> Option<El> {
    let sprite = app.images.get(&app.fx, uri)?;
    let [iw, ih] = sprite.size()?;
    let (tile_w, tile_h) = (140.0, 140.0 * ih as f32 / iw.max(1) as f32);
    let mut root = layer(w, h);
    let mut y = 0.0;
    while y < h {
        let mut x = 0.0;
        while x < w {
            root = root.child(el().absolute(x, y).size(tile_w, tile_h).pixels(sprite.pixels()));
            x += tile_w;
        }
        y += tile_h.max(1.0);
    }
    Some(root)
}

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let theme = app.theme();
    let [w, h] = super::window_size(cx);
    let bg = app.settings.background.trim();
    let still = theme.reduce_motion || cx.is_reduced_motion();
    let root = el().key(KEY).w_full().h_full().clip_corners([0.0; 4]).fill(theme.bg_app);
    if !bg.starts_with("anim:lattice") || still {
        app.extras.bg.noise.borrow_mut().take();
    }
    if let Some(name) = bg.strip_prefix("anim:") {
        let name: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        return anim(app, &theme, &name, w, h, still, root);
    }
    if let Some(css) = bg.strip_prefix("color:") {
        let css = css.trim();
        if css.starts_with("linear-gradient(") {
            return match css_linear_gradient(css, &theme, w, h) {
                Some((angle, stops)) => root.linear_gradient(angle, stops),
                None => root,
            };
        }
        return root.fill(css_color(css, &theme).unwrap_or(theme.bg_app));
    }
    if let Some(rest) = bg.strip_prefix("pattern:") {
        let (name, color) = rest.split_once('|').map_or((rest, None), |(name, color)| (name, Some(color).filter(|color| !color.is_empty())));
        let base = color.and_then(|color| css_color(color, &theme)).unwrap_or(theme.bg_app);
        let root = root.fill(base);
        return match pattern(&theme, name, w, h) {
            Some(overlay) => root.child(overlay),
            None => root,
        };
    }
    if let Some(rest) = bg.strip_prefix("image:") {
        let (color, uri) = split_once_pipe(rest);
        let scrim = hex6(color.unwrap_or(DEFAULT_COLOR), 0.55);
        let mut root = root;
        if let Some(img) = app.images.get(&app.fx, uri) {
            root = root.child(layer(w, h).pixels_fit(img.pixels(), SurfaceFit::Fill));
        }
        return root.child(layer(w, h).fill(scrim));
    }
    if let Some(rest) = bg.strip_prefix("tile:") {
        let (color, uri) = split_once_pipe(rest);
        let base = color.and_then(|color| css_color(color, &theme)).unwrap_or(theme.bg_app);
        let root = root.fill(base);
        return match sprite_tiles(app, uri, w, h) {
            Some(tiles) => root.child(tiles),
            None => root,
        };
    }
    if let Some(rest) = bg.strip_prefix("texture:") {
        let (color, uri, opacity) = parse_texture(rest);
        let scrim = hex6(color.unwrap_or(DEFAULT_COLOR), (1.0 - opacity).clamp(0.0, 1.0));
        let mut root = root;
        if let Some(tiles) = texture_tiles(app, uri, w, h) {
            root = root.child(tiles);
        }
        return root.child(layer(w, h).fill(scrim));
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gradients_and_textures() {
        let theme = Theme::from_settings(&crate::settings::Settings::default());
        let (angle, stops) = css_linear_gradient("linear-gradient(160deg, #d98a2b 0%, #8a4a2a 45%, #3a2340 100%)", &theme, 100.0, 100.0).unwrap();
        assert!((angle - 160f32.to_radians()).abs() < 1e-5);
        assert_eq!(stops.iter().map(|stop| stop.0).collect::<Vec<_>>(), vec![0.0, 0.45, 1.0]);
        let (_, stops) = css_linear_gradient("linear-gradient(red, #000, rgba(0, 0, 0, 0.5))", &theme, 100.0, 100.0).unwrap_or((0.0, vec![]));
        assert!(stops.is_empty() || stops.len() == 3);
        let (_, stops) = css_linear_gradient("linear-gradient(to right, #000, #111, #222)", &theme, 100.0, 100.0).unwrap();
        assert_eq!(stops.iter().map(|stop| stop.0).collect::<Vec<_>>(), vec![0.0, 0.5, 1.0]);
        assert_eq!(parse_texture("0.5|#211c17|/textures/stone-blocks.png"), (Some("#211c17"), "/textures/stone-blocks.png", 0.5));
        assert_eq!(parse_texture("/a.png"), (None, "/a.png", 0.5));
        assert_eq!(split_once_pipe("#112233|data:x"), (Some("#112233"), "data:x"));
    }
}
