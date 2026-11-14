//! SkinEditor.svelte: paint the 64×64 skin sheet texel by texel (draw, erase, fill, pick), with
//! undo, a grid, the layout guides and flat front/back previews, then upload it.

use std::cell::Cell;

use rune::{Color, Cursor, Drag, El, Phase, Tx, col, el, row, text};

use super::skins::{self, BACK, FRONT, Img};
use super::viewer::Model;
use crate::app::{App, Ignore};
use crate::core::i18n::t as tr;
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const SIZE: usize = 64;
/// Points per texel on the canvas (Svelte's is fluid at about 7.4).
const TEXEL: f32 = 7.0;
const VIEW: f32 = SIZE as f32 * TEXEL;
/// Points per texel in the 16×32 previews.
const PREVIEW_TEXEL: f32 = 4.5;
const UNDO_LIMIT: usize = 40;

const GUIDES: [[usize; 4]; 12] = [
    [0, 0, 32, 16], [32, 0, 32, 16],
    [16, 16, 24, 16], [16, 32, 24, 16],
    [0, 16, 16, 16], [0, 32, 16, 16],
    [40, 16, 16, 16], [40, 32, 16, 16],
    [16, 48, 16, 16], [0, 48, 16, 16],
    [32, 48, 16, 16], [48, 48, 16, 16],
];

pub const PALETTE: [&str; 15] = [
    "#000000", "#3d3a34", "#7f7970", "#c9c4ba", "#ffffff",
    "#b1836a", "#8a5a3b", "#e8b23a", "#d9e746", "#57c84a",
    "#3aa0e8", "#3a4ae8", "#9b4ae8", "#e84a8a", "#e84a4a",
];

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Tool {
    #[default]
    Draw,
    Erase,
    Fill,
    Pick,
}

const TOOLS: [(Tool, &str); 4] = [
    (Tool::Draw, "skinEditor.draw"),
    (Tool::Erase, "skinEditor.erase"),
    (Tool::Fill, "skinEditor.fill"),
    (Tool::Pick, "skinEditor.pick"),
];

pub struct State {
    pub open: bool,
    pub uuid: String,
    pub variant: Model,
    pub tool: Tool,
    /// The colour as `#rrggbb` text (the Svelte `<input type=color>`).
    pub color: rune::TextEdit,
    pub show_grid: bool,
    pub loading: bool,
    pub applying: bool,
    pub sheet: Vec<u8>,
    undo: Vec<Vec<u8>>,
    redo: Vec<Vec<u8>>,
    last: Option<(i32, i32)>,
    painting: bool,
    pub view: rune::Pixels,
    pub front: rune::Pixels,
    pub back: rune::Pixels,
    /// The device scale the pixels were last drawn at.
    drawn_scale: Cell<f32>,
}

impl Default for State {
    fn default() -> Self {
        State {
            open: false,
            uuid: String::new(),
            variant: Model::Classic,
            tool: Tool::Draw,
            color: rune::TextEdit::new("#57c84a"),
            show_grid: true,
            loading: true,
            applying: false,
            sheet: vec![0; SIZE * SIZE * 4],
            undo: Vec::new(),
            redo: Vec::new(),
            last: None,
            painting: false,
            view: rune::Pixels::new(),
            front: rune::Pixels::new(),
            back: rune::Pixels::new(),
            drawn_scale: Cell::new(0.0),
        }
    }
}

pub fn parse_hex(input: &str) -> Option<[u8; 3]> {
    let digits = input.trim().strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

impl State {
    pub fn rgb(&self) -> [u8; 3] {
        parse_hex(&self.color.text).unwrap_or([0x57, 0xc8, 0x4a])
    }

    fn set_color(&mut self, hex: &str) {
        self.color = rune::TextEdit::new(hex);
    }

    fn push_undo(&mut self) {
        self.undo.push(self.sheet.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn paint(&mut self, x: i32, y: i32) {
        if !(0..SIZE as i32).contains(&x) || !(0..SIZE as i32).contains(&y) {
            return;
        }
        let i = (y as usize * SIZE + x as usize) * 4;
        let pixel = if self.tool == Tool::Erase {
            [0, 0, 0, 0]
        } else {
            let [r, g, b] = self.rgb();
            [r, g, b, 255]
        };
        self.sheet[i..i + 4].copy_from_slice(&pixel);
    }

    /// Bresenham from (x0, y0) to (x1, y1), painting each texel.
    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32) {
        let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx - dy;
        loop {
            self.paint(x0, y0);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn flood_fill(&mut self, sx: usize, sy: usize) {
        let idx = |x: usize, y: usize| (y * SIZE + x) * 4;
        let target: [u8; 4] = self.sheet[idx(sx, sy)..idx(sx, sy) + 4].try_into().unwrap();
        let [r, g, b] = self.rgb();
        let fill_color = [r, g, b, 255];
        if target == fill_color {
            return;
        }
        let mut stack = vec![(sx as i32, sy as i32)];
        while let Some((x, y)) = stack.pop() {
            if x < 0 || y < 0 || x >= SIZE as i32 || y >= SIZE as i32 {
                continue;
            }
            let i = idx(x as usize, y as usize);
            if self.sheet[i..i + 4] != target {
                continue;
            }
            self.sheet[i..i + 4].copy_from_slice(&fill_color);
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
    }

    fn pick(&mut self, x: usize, y: usize) {
        let i = (y * SIZE + x) * 4;
        if self.sheet[i + 3] == 0 {
            return;
        }
        let hex = format!("#{:02x}{:02x}{:02x}", self.sheet[i], self.sheet[i + 1], self.sheet[i + 2]);
        self.set_color(&hex);
        self.tool = Tool::Draw;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.sheet, prev));
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.sheet, next));
        }
    }

    /// Draw the canvas and previews at `scale` device pixels per point.
    pub fn render(&self, theme: &Theme, scale: f32) {
        self.drawn_scale.set(scale);
        let texel_px = ((TEXEL * scale).round() as usize).max(1);
        let n = SIZE * texel_px;
        let bg = rgba8(theme.bg_app);
        let mut img = Img::new(n, n);
        for ty in 0..SIZE {
            for tx in 0..SIZE {
                let mut color = bg;
                if (tx + ty) % 2 == 1 {
                    skins::blend_over(&mut color, &[255, 255, 255, 10]);
                }
                let i = (ty * SIZE + tx) * 4;
                skins::blend_over(&mut color, &self.sheet[i..i + 4]);
                for y in ty * texel_px..ty * texel_px + texel_px {
                    let row_start = (y * n + tx * texel_px) * 4;
                    for x in 0..texel_px {
                        img.rgba[row_start + x * 4..row_start + x * 4 + 4].copy_from_slice(&color);
                    }
                }
            }
        }
        let put = |img: &mut Img, x: usize, y: usize, color: [u8; 4]| {
            if x < n && y < n {
                skins::blend_over(&mut img.rgba[(y * n + x) * 4..][..4], &color);
            }
        };
        if self.show_grid {
            let grid_line = [0, 0, 0, 46];
            for i in 1..SIZE {
                for j in 0..n {
                    put(&mut img, i * texel_px, j, grid_line);
                    put(&mut img, j, i * texel_px, grid_line);
                }
            }
        }
        let guide = [232, 178, 58, 217];
        let guide_w = ((2.0 * scale).round() as usize).max(1);
        for [gx, gy, w, h] in GUIDES {
            let (x0, y0, x1, y1) = (gx * texel_px, gy * texel_px, (gx + w) * texel_px, (gy + h) * texel_px);
            for y in y0..y1 {
                for x in x0..x1 {
                    let edge = x < x0 + guide_w || x >= x1 - guide_w || y < y0 + guide_w || y >= y1 - guide_w;
                    if edge {
                        skins::blend_over(&mut img.rgba[(y * n + x) * 4..][..4], &guide);
                    }
                }
            }
        }
        self.view.set(n as u32, n as u32, img.rgba);

        let preview_px = ((PREVIEW_TEXEL * scale).round() as usize).max(1);
        for (pixels, faces) in [(&self.front, &FRONT), (&self.back, &BACK)] {
            let mut preview = Img::new(16 * preview_px, 32 * preview_px);
            for px in preview.rgba.chunks_mut(4) {
                px.copy_from_slice(&bg);
            }
            for rect in faces {
                preview.draw(&self.sheet, SIZE, *rect, preview_px);
            }
            pixels.set(preview.w as u32, preview.h as u32, preview.rgba);
        }
    }

    fn refresh(&self, theme: &Theme) {
        let scale = self.drawn_scale.get();
        if scale > 0.0 {
            self.render(theme, scale);
        }
    }
}

fn rgba8(color: Color) -> [u8; 4] {
    let hex = color.hex();
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
    [byte(1), byte(3), byte(5), byte(7)]
}

/// Open the editor for the panel's account and load its skin.
pub fn open(app: &mut App) {
    let Some(uuid) = app.account_modal.panel.uuid.clone() else { return };
    let editor = &mut app.account_modal.editor;
    let saved = (editor.view.clone(), editor.front.clone(), editor.back.clone(), editor.drawn_scale.get(), editor.color.text.clone());
    *editor = State { open: true, uuid, variant: app.account_modal.panel.variant, ..State::default() };
    (editor.view, editor.front, editor.back) = (saved.0, saved.1, saved.2);
    editor.drawn_scale.set(saved.3);
    editor.set_color(&saved.4);
    load(app);
}

fn load(app: &mut App) {
    app.account_modal.editor.loading = true;
    let uuid = app.account_modal.editor.uuid.clone();
    let cached = app.account_modal.skins.full(&uuid).map(str::to_string);
    app.fx.spawn(
        async move {
            let uri = match cached {
                Some(cached_uri) => cached_uri,
                None => crate::commands::download_image(format!("https://minotar.net/skin/{uuid}")).await.unwrap_or_default(),
            };
            skins::data_uri_bytes(&uri).and_then(|bytes| skins::decode(&bytes))
        },
        |app, img| {
            let editor = &mut app.account_modal.editor;
            editor.sheet = vec![0; SIZE * SIZE * 4];
            // `drawImage(img, 0, 0)` onto a 64×64 canvas: the top-left 64×64, as it is.
            if let Some((w, h, rgba)) = img {
                for y in 0..(h as usize).min(SIZE) {
                    for x in 0..(w as usize).min(SIZE) {
                        let si = (y * w as usize + x) * 4;
                        editor.sheet[(y * SIZE + x) * 4..][..4].copy_from_slice(&rgba[si..si + 4]);
                    }
                }
            }
            editor.loading = false;
            let theme = app.theme();
            app.account_modal.editor.refresh(&theme);
        },
    );
}

fn close(app: &mut App) {
    app.account_modal.editor.open = false;
}

fn apply(app: &mut App) {
    let editor = &mut app.account_modal.editor;
    let img = Img { w: SIZE, h: SIZE, rgba: editor.sheet.clone() };
    let Some(png) = skins::encode_png(&img) else { return };
    editor.applying = true;
    let variant = if editor.variant == Model::Slim { "slim" } else { "classic" }.to_string();
    let uuid = editor.uuid.clone();
    let host = app.fx.host.clone();
    let uri = skins::bytes_data_uri(&png);
    app.fx.spawn(async move { crate::commands::set_skin(host, png, variant).await }, move |app, result| {
        app.account_modal.editor.applying = false;
        match result {
            Ok(()) => {
                if app.account_modal.panel.uuid.as_deref() == Some(&uuid) {
                    super::panel::set_skin_data(app, uri.clone());
                }
                skins::set_skin(app, &uuid, uri);
                app.toasts.success(tr("skinEditor.applied")).ignore();
                close(app);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn on_canvas(app: &mut App, drag: &Drag) {
    let texel = |pt: f32| (pt / TEXEL).floor() as i32;
    let (x, y) = (texel(drag.x), texel(drag.y));
    let inside = (0..SIZE as i32).contains(&x) && (0..SIZE as i32).contains(&y);
    let editor = &mut app.account_modal.editor;
    if editor.loading {
        return;
    }
    match drag.phase {
        Phase::Down => {
            if !inside {
                return;
            }
            match editor.tool {
                Tool::Pick => {
                    editor.pick(x as usize, y as usize);
                    return;
                }
                Tool::Fill => {
                    editor.push_undo();
                    editor.flood_fill(x as usize, y as usize);
                }
                Tool::Draw | Tool::Erase => {
                    editor.push_undo();
                    editor.painting = true;
                    editor.last = Some((x, y));
                    editor.paint(x, y);
                }
            }
        }
        Phase::Move => {
            if !editor.painting || !inside {
                return;
            }
            let Some((last_x, last_y)) = editor.last else { return };
            if (last_x, last_y) == (x, y) {
                return;
            }
            editor.line(last_x, last_y, x, y);
            editor.last = Some((x, y));
        }
        _ => {
            editor.painting = false;
            editor.last = None;
            return;
        }
    }
    let theme = app.theme();
    app.account_modal.editor.refresh(&theme);
}

fn edit(app: &mut App, f: impl FnOnce(&mut State)) {
    f(&mut app.account_modal.editor);
    let theme = app.theme();
    app.account_modal.editor.refresh(&theme);
}

fn side_label(theme: &Theme, label: String) -> El {
    text(label).font(theme.sans(11.0, 400.0)).color(theme.text_muted).mb(6.0)
}

fn boxed(theme: &Theme, pixels: &rune::Pixels, w: f32, h: f32) -> El {
    el().size(w + 4.0, h + 4.0).padding(2.0).fill(theme.bg_app).border(2.0, theme.border).child(el().size(w, h).pixels(pixels))
}

/// A `.tool` / `.mini` button: input fill, 2 pt border that turns accent when on or hovered.
fn flat(app: &App, theme: &Theme, key: &str, on: bool, disabled: bool) -> El {
    let hovered = app.account_modal.hovered.as_deref() == Some(key) && !disabled;
    let hover_key = key.to_string();
    let mut button = row()
        .key(key.to_string())
        .center()
        .gap(5.0)
        .fill(if on { theme.accent_soft } else { theme.bg_input })
        .border(2.0, if on || hovered { theme.accent } else { theme.border })
        .role("button")
        .focusable(!disabled)
        .on_hover_change(move |app: &mut App, over, _| super::set_hovered(app, &hover_key, over));
    if disabled {
        button = button.opacity(0.4).disabled(true);
    } else {
        button = button.cursor(Cursor::Pointer);
    }
    button
}

pub fn view(app: &App, cx: &mut rune::Cx<App>, layer: &mut Vec<El>) {
    let editor = &app.account_modal.editor;
    if !editor.open {
        return;
    }
    let theme = app.theme();
    let scale = cx.env().scale;
    if editor.drawn_scale.get() != scale {
        editor.render(&theme, scale);
    }

    let mut canvas = el()
        .key("skin-canvas")
        .size(VIEW + 4.0, VIEW + 4.0)
        .padding(2.0)
        .fill(theme.bg_app)
        .border(2.0, theme.border)
        .child(
            el().key("skin-canvas-px")
                .size(VIEW, VIEW)
                .pixels(&editor.view)
                .cursor(Cursor::Crosshair)
                .on_drag(|app: &mut App, drag: &mut Drag, tx: &mut Tx| {
                    on_canvas(app, drag);
                    tx.instant();
                }),
        );
    if editor.loading {
        canvas = canvas.child(
            row().key("loading")
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .center()
                .gap(8.0)
                .fill(Color::BLACK.alpha(0.5))
                .child(ui::spinner(&theme, 14.0))
                .child(text(tr("skinEditor.loading")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary)),
        );
    }

    let preview_column = |key: &str, pixels: &rune::Pixels, caption: String| {
        col().key(key.to_string()).items_center().gap(4.0).child(boxed(&theme, pixels, 16.0 * PREVIEW_TEXEL, 32.0 * PREVIEW_TEXEL)).child(text(caption).font(theme.sans(10.0, 400.0)).color(theme.text_muted))
    };
    let preview = col()
        .key("preview-box")
        .items_center()
        .child(side_label(&theme, tr("skinEditor.preview")))
        .child(row().gap(10.0).justify_center().child(preview_column("pv-front", &editor.front, tr("skinEditor.front"))).child(preview_column("pv-back", &editor.back, tr("skinEditor.back"))));

    let mut tools = col().key("tools").gap(6.0);
    for pair in TOOLS.chunks(2) {
        let mut tool_row = row().gap(6.0);
        for &(tool, key) in pair {
            let on = editor.tool == tool;
            tool_row = tool_row.child(
                flat(app, &theme, &format!("tool-{key}"), on, false)
                    .grow(1.0)
                    .padding(8.0)
                    .label(tr(key))
                    .on_press(move |app: &mut App| app.account_modal.editor.tool = tool)
                    .child(text(tr(key)).font(theme.sans(12.5, 600.0)).color(if on { theme.accent } else { theme.text_secondary })),
            );
        }
        tools = tools.child(tool_row);
    }

    let current = editor.rgb();
    let current_hex = format!("#{:02x}{:02x}{:02x}", current[0], current[1], current[2]);
    let mut swatches = col().key("swatches").gap(4.0).grow(1.0);
    for chunk in PALETTE.chunks(5) {
        let mut swatch_row = row().gap(4.0);
        for &hex in chunk {
            let on = current_hex == hex;
            let color = crate::theme::parse_color(hex).unwrap_or(Color::BLACK);
            let mut swatch = el()
                .key(format!("swatch-{hex}"))
                .size(26.0, 26.0)
                .fill(color)
                .border(2.0, if on { theme.text } else { theme.border })
                .role("button")
                .label(hex.to_string())
                .focusable(true)
                .cursor(Cursor::Pointer)
                .on_press(move |app: &mut App| app.account_modal.editor.set_color(hex));
            if on {
                swatch = swatch.outline(1.0, 0.0, theme.accent);
            }
            swatch_row = swatch_row.child(swatch);
        }
        swatches = swatches.child(swatch_row);
    }
    let color_row = col()
        .key("color")
        .gap(6.0)
        .child(
            row().gap(8.0).items_start()
                .child(el().key("color-current").size(34.0, 34.0).fill(crate::theme::parse_color(&current_hex).unwrap_or(Color::BLACK)).border(2.0, theme.border).label(tr("skinEditor.color")))
                .child(swatches),
        )
        .child(ui::input(&theme, &editor.color, "#57c84a", |app| &mut app.account_modal.editor.color).key("color-hex"));

    let mini = |key: &str, on: bool, disabled: bool| flat(app, &theme, key, on, disabled).min_w(34.0).h(32.0).px(8.0);
    let actions = row()
        .key("actions")
        .gap(6.0)
        .child(
            mini("undo", false, editor.undo.is_empty())
                .label(tr("skinEditor.undo"))
                .help(tr("skinEditor.undo"))
                .on_press(|app: &mut App| edit(app, State::undo))
                .child(text("↶").font(theme.sans(15.0, 400.0)).color(theme.text_secondary)),
        )
        .child(
            mini("redo", false, editor.redo.is_empty())
                .label(tr("skinEditor.redo"))
                .help(tr("skinEditor.redo"))
                .on_press(|app: &mut App| edit(app, State::redo))
                .child(text("↷").font(theme.sans(15.0, 400.0)).color(theme.text_secondary)),
        )
        .child(
            mini("grid", editor.show_grid, false)
                .label(tr("skinEditor.grid"))
                .help(tr("skinEditor.grid"))
                .on_press(|app: &mut App| edit(app, |editor| editor.show_grid = !editor.show_grid))
                .child(text("#").font(theme.sans(15.0, 400.0)).color(if editor.show_grid { theme.accent } else { theme.text_secondary })),
        )
        .child(
            mini("reset", false, false)
                .grow(1.0)
                .label(tr("skinEditor.reset"))
                .on_press(|app: &mut App| {
                    app.account_modal.editor.push_undo();
                    load(app);
                })
                .child(ui::colored_icon(&icons::REFRESH, 13.0, theme.text_secondary))
                .child(text(tr("skinEditor.reset")).font(theme.sans(12.5, 400.0)).color(theme.text_secondary)),
        );

    let side = col().key("side").w(190.0).gap(14.0).child(preview).child(tools).child(color_row).child(actions);
    let body = row().key("editor").gap(16.0).items_start().child(canvas).child(side);
    let footer = ui::buttons([
        Btn::new(tr("common.cancel")).key("editor-cancel").ghost().on_press(close).build(&theme),
        Btn::new(if editor.applying { tr("skinEditor.applying") } else { tr("skinEditor.apply") })
            .key("editor-apply")
            .primary()
            .disabled(editor.applying || editor.loading)
            .on_press(apply)
            .build(&theme),
    ]);
    layer.push(ui::modal(&theme, "skin-editor", tr("skinEditor.title"), 720.0, body, Some(footer), close));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> State {
        State { loading: false, ..State::default() }
    }

    #[test]
    fn line_paints_every_texel_between() {
        let mut state = editor();
        state.line(0, 0, 5, 2);
        let painted: Vec<usize> = (0..SIZE * 3).filter(|i| state.sheet[i * 4 + 3] == 255).collect();
        assert_eq!(painted.len(), 6);
        assert!(painted.contains(&0) && painted.contains(&(2 * SIZE + 5)));
    }

    #[test]
    fn fill_stops_at_other_colours_and_pick_reads_back() {
        let mut state = editor();
        state.set_color("#ff0000");
        state.line(0, 3, 63, 3);
        state.set_color("#0000ff");
        state.flood_fill(0, 0);
        assert_eq!(&state.sheet[..4], &[0, 0, 255, 255]);
        assert_eq!(&state.sheet[(10 * SIZE) * 4..(10 * SIZE) * 4 + 4], &[0, 0, 0, 0]);
        state.tool = Tool::Pick;
        state.pick(5, 3);
        assert_eq!(state.color.text, "#ff0000");
        assert_eq!(state.tool, Tool::Draw);
    }

    #[test]
    fn undo_and_redo_swap_sheets() {
        let mut state = editor();
        state.push_undo();
        state.paint(1, 1);
        state.undo();
        assert!(state.sheet.iter().all(|&byte| byte == 0));
        state.redo();
        assert_eq!(state.sheet[(SIZE + 1) * 4 + 3], 255);
        assert_eq!(parse_hex("#E8b23A"), Some([0xe8, 0xb2, 0x3a]));
        assert_eq!(parse_hex("e8b23a"), None);
    }
}
