//! `HomeGrid.svelte` and `stores/instanceLayout.svelte.ts`: the tiles, packed as CSS grid's
//! `row dense` flow, with folders, drag to reorder or group, and resize in arrange mode.

use std::collections::BTreeMap;
use std::rc::Rc;

use rune::{Color, Cursor, Drag, El, Phase, PointerPress, Shadow, Spring, SurfaceFit, Tx, col, el, row, text};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::core::prefs::Prefs;
use crate::instance::{Instance, UpdateInstance};
use crate::theme::Theme;
use crate::ui;

use super::card::{card_face, instance_card};

pub const GAP: f32 = 16.0;
pub const CELL: f32 = 168.0;
pub const PITCH: f32 = CELL + GAP;
const MAX_H: u32 = 4;
const DRAG_THRESHOLD: f32 = 6.0;
const LAYOUT_KEY: &str = "cactus:instanceLayout";
pub const COVERS_KEY: &str = "cactus:groupCovers";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub w: u32,
    pub h: u32,
    pub order: i64,
}

const UNORDERED: i64 = 9_007_199_254_740_991;

/// `instanceLayout.load`, legacy size labels included.
pub fn load_layout(prefs: &Prefs) -> BTreeMap<String, Cell> {
    let raw: BTreeMap<String, Value> = prefs.read(LAYOUT_KEY, BTreeMap::new());
    let mut out = BTreeMap::new();
    for (id, saved) in raw {
        let order = saved.get("order").and_then(Value::as_i64).unwrap_or(0);
        let num = |field: &str| saved.get(field).and_then(Value::as_f64).map(|value| value.max(1.0) as u32);
        let cell = match (num("w"), num("h"), saved.get("size").and_then(Value::as_str)) {
            (Some(w), Some(h), _) => Some(Cell { w, h, order }),
            (_, _, Some("s")) => Some(Cell { w: 1, h: 1, order }),
            (_, _, Some("w")) => Some(Cell { w: 2, h: 1, order }),
            (_, _, Some("t")) => Some(Cell { w: 1, h: 2, order }),
            (_, _, Some("l")) => Some(Cell { w: 2, h: 2, order }),
            _ => None,
        };
        if let Some(cell) = cell {
            out.insert(id, cell);
        }
    }
    out
}

pub fn cell_of(layout: &BTreeMap<String, Cell>, id: &str) -> Cell {
    layout.get(id).copied().unwrap_or(Cell { w: 1, h: 1, order: UNORDERED })
}

fn save_layout(app: &mut App) {
    let cells = app.home.layout.clone();
    app.prefs.write(LAYOUT_KEY, cells);
}

pub fn reset_layout(app: &mut App) {
    app.home.layout.clear();
    save_layout(app);
}

/// Reorders the in-memory layout; the drag saves it once, on release.
fn reorder(app: &mut App, ids: &[String]) {
    for (i, id) in ids.iter().enumerate() {
        let cell = cell_of(&app.home.layout, id);
        app.home.layout.insert(id.clone(), Cell { order: i as i64, ..cell });
    }
}

pub fn save_covers(app: &mut App) {
    let covers = app.home.covers.clone();
    app.prefs.write(COVERS_KEY, covers);
}

pub enum Entry<'a> {
    Instance(&'a Instance),
    Folder { name: String, instances: Vec<&'a Instance> },
}

impl Entry<'_> {
    pub fn id(&self) -> String {
        match self {
            Entry::Instance(inst) => inst.id.clone(),
            Entry::Folder { name, .. } => format!("folder:{name}"),
        }
    }
}

/// A drag from a tile (`press` / `started` / `dropTarget` in HomeGrid).
#[derive(Clone, Debug)]
pub struct DragState {
    pub id: String,
    pub folder: bool,
    /// Where in the tile it was grabbed, and the tile's place in the grid at the press.
    grab: [f32; 2],
    origin: [f32; 2],
    /// The pointer in grid points.
    pointer: [f32; 2],
    pub started: bool,
    pub over: Option<String>,
    last_over: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    E,
    S,
    Se,
}

#[derive(Clone, Debug)]
pub struct ResizeState {
    pub id: String,
    axis: Axis,
    start: Cell,
}

/// Places `spans` on `cols` columns as `grid-auto-flow: row dense` does: each tile in the
/// first free spot, scanning from the top-left.
fn pack(spans: &[(u32, u32)], cols: u32) -> (Vec<(u32, u32)>, u32) {
    let mut taken: Vec<Vec<bool>> = Vec::new();
    let mut out = Vec::with_capacity(spans.len());
    let mut rows = 0;
    for &(w, h) in spans {
        let w = w.clamp(1, cols);
        let mut top = 0u32;
        'find: loop {
            for left in 0..=(cols - w) {
                let free = (top..top + h).all(|y| {
                    (left..left + w).all(|x| taken.get(y as usize).is_none_or(|line| !line[x as usize]))
                });
                if free {
                    for y in top..top + h {
                        while taken.len() <= y as usize {
                            taken.push(vec![false; cols as usize]);
                        }
                        for x in left..left + w {
                            taken[y as usize][x as usize] = true;
                        }
                    }
                    out.push((left, top));
                    rows = rows.max(top + h);
                    break 'find;
                }
            }
            top += 1;
        }
    }
    (out, rows)
}

pub fn span_px(n: u32) -> f32 {
    n as f32 * CELL + (n.saturating_sub(1)) as f32 * GAP
}

fn folder_icon(w: u32, h: u32) -> f32 {
    let avail_w = span_px(w) - 28.0;
    let avail_h = span_px(h) - 60.0;
    ((avail_w - 4.0) / 2.0).min((avail_h - 4.0) / 2.0).floor().clamp(28.0, 148.0)
}

pub fn set_group(app: &mut App, id: &str, name: &str) {
    app.update_instance(id, UpdateInstance { group: Some(name.to_string()), ..Default::default() }, |app, result| {
        if let Err(e) = result {
            app.toasts.error(e.to_string());
        }
    });
}

fn unique_folder_name(app: &App) -> String {
    let taken: Vec<&str> = app.instances.iter().filter_map(|inst| inst.group.as_deref()).filter(|group| !group.is_empty()).collect();
    let mut name = tr("grid.newFolder");
    let mut n = 2;
    while taken.contains(&name.as_str()) {
        name = tf("grid.newFolderNumbered", &[("n", &n.to_string())]);
        n += 1;
    }
    name
}

/// `performGroup`: dropping instance `dragged` on `target` puts both in one folder.
fn perform_group(app: &mut App, dragged: &str, target: &str) {
    if target == dragged {
        return;
    }
    if let Some(name) = target.strip_prefix("folder:") {
        let name = name.to_string();
        set_group(app, dragged, &name);
    } else {
        let name = unique_folder_name(app);
        set_group(app, dragged, &name);
        set_group(app, target, &name);
    }
}

type Rects = Rc<Vec<(String, [f32; 4])>>;

fn tile_under(rects: &Rects, point: [f32; 2]) -> Option<String> {
    rects
        .iter()
        .find(|(_, rect)| point[0] >= rect[0] && point[0] < rect[0] + rect[2] && point[1] >= rect[1] && point[1] < rect[1] + rect[3])
        .map(|(id, _)| id.clone())
}

fn on_tile_drag(app: &mut App, drag: &Drag, id: &str, folder: bool, origin: [f32; 2], rects: &Rects, arranging: bool) {
    match drag.phase {
        Phase::Down => {
            app.home.suppress_click = false;
            if app.home.resize.is_some() {
                return;
            }
            app.home.drag = Some(DragState {
                id: id.to_string(),
                folder,
                grab: [drag.start_x, drag.start_y],
                origin,
                pointer: [origin[0] + drag.start_x, origin[1] + drag.start_y],
                started: false,
                over: None,
                last_over: None,
            });
        }
        Phase::Move => {
            let Some(state) = app.home.drag.as_mut() else { return };
            if !state.started && drag.dx.hypot(drag.dy) < DRAG_THRESHOLD {
                return;
            }
            state.started = true;
            state.pointer = [state.origin[0] + state.grab[0] + drag.dx, state.origin[1] + state.grab[1] + drag.dy];
            let over = tile_under(rects, state.pointer).filter(|over_id| *over_id != state.id);
            if arranging {
                if over.is_some() && over != state.last_over {
                    let target = over.clone().unwrap_or_default();
                    let dragged = state.id.clone();
                    state.last_over = over;
                    let mut ids: Vec<String> = rects.iter().map(|(tile_id, _)| tile_id.clone()).collect();
                    if let (Some(from), Some(to)) = (ids.iter().position(|tile_id| *tile_id == dragged), ids.iter().position(|tile_id| *tile_id == target)) {
                        let moved = ids.remove(from);
                        ids.insert(to, moved);
                        reorder(app, &ids);
                    }
                } else {
                    state.last_over = over;
                }
            } else if !state.folder {
                state.over = over;
            }
        }
        Phase::Up => {
            let Some(state) = app.home.drag.take() else { return };
            if state.started {
                app.home.suppress_click = true;
                if arranging {
                    save_layout(app);
                }
            }
            if drag.cancelled() {
                return;
            }
            if state.started && !arranging && !state.folder {
                if let Some(target) = state.over {
                    perform_group(app, &state.id, &target);
                }
            }
        }
    }
}

fn on_resize(app: &mut App, drag: &Drag, id: &str, axis: Axis, max_cols: u32) {
    match drag.phase {
        Phase::Down => {
            app.home.drag = None;
            app.home.resize = Some(ResizeState { id: id.to_string(), axis, start: cell_of(&app.home.layout, id) });
        }
        Phase::Move => {
            let Some(resize) = app.home.resize.clone() else { return };
            let mut w = resize.start.w;
            let mut h = resize.start.h;
            if matches!(resize.axis, Axis::E | Axis::Se) {
                w = (resize.start.w as f32 + (drag.dx / PITCH).round()).clamp(1.0, max_cols as f32) as u32;
            }
            if matches!(resize.axis, Axis::S | Axis::Se) {
                h = (resize.start.h as f32 + (drag.dy / PITCH).round()).clamp(1.0, MAX_H as f32) as u32;
            }
            let current = cell_of(&app.home.layout, &resize.id);
            if current.w != w || current.h != h {
                app.home.layout.insert(resize.id.clone(), Cell { w, h, order: current.order });
            }
        }
        Phase::Up => {
            // Saved once here, not per step: each save rewrites the whole prefs file.
            if app.home.resize.take().is_some() {
                save_layout(app);
            }
        }
    }
}

/// The grid of `entries` in `width` points.
pub fn view(app: &App, entries: &[Entry], width: f32, arranging: bool) -> El {
    let theme = app.theme();
    let max_cols = (((width + GAP) / PITCH).floor() as u32).max(1);
    let layout = &app.home.layout;
    let mut ordered: Vec<&Entry> = entries.iter().collect();
    ordered.sort_by_key(|entry| cell_of(layout, &entry.id()).order);
    let spans: Vec<(u32, u32)> = ordered
        .iter()
        .map(|entry| {
            let cell = cell_of(layout, &entry.id());
            (cell.w.clamp(1, max_cols), cell.h.clamp(1, MAX_H))
        })
        .collect();
    let (places, rows) = pack(&spans, max_cols);
    let rects: Rects = Rc::new(
        ordered
            .iter()
            .zip(&places)
            .zip(&spans)
            .map(|((entry, &(left, top)), &(w, h))| (entry.id(), [left as f32 * PITCH, top as f32 * PITCH, span_px(w), span_px(h)]))
            .collect(),
    );

    let active_drag = app.home.drag.as_ref().filter(|state| state.started);
    let mut grid = el().key("home-grid").w_full().h(span_px(rows).max(0.0));
    for (i, entry) in ordered.iter().enumerate() {
        let (id, rect) = &rects[i];
        let (w, h) = spans[i];
        let dragger = {
            let id = id.clone();
            let rects = rects.clone();
            let folder = matches!(entry, Entry::Folder { .. });
            let origin = [rect[0], rect[1]];
            move |app: &mut App, drag: &mut Drag, _tx: &mut Tx| on_tile_drag(app, drag, &id, folder, origin, &rects, arranging)
        };
        let mut content = tile_content(app, &theme, entry, w, h, arranging);
        if !arranging {
            content = content.on_drag(dragger.clone());
        }
        let dragging = active_drag.is_some_and(|state| state.id == *id);
        let target = active_drag.is_some_and(|state| state.over.as_deref() == Some(id.as_str()));
        let mut tile = el()
            .key(format!("tile-{id}"))
            .size(rect[2], rect[3])
            .absolute(rect[0], rect[1])
            .motion(Spring::new(0.26, 0.0))
            .child(content);
        if arranging {
            // Cards and folders don't take presses while arranging; the whole tile drags.
            tile = tile.child(
                el().key("dash")
                    .size(rect[2], rect[3])
                    .absolute(0.0, 0.0)
                    .border(2.0, theme.bg_app)
                    .border_dash(4.0, 6.0)
                    .cursor(Cursor::Grab)
                    .on_drag(dragger)
                    .on_secondary_press(|_: &mut App, _: PointerPress, _: &mut Tx| {}),
            );
        }
        if dragging {
            tile = tile.opacity(0.34).outline(2.0, -2.0, theme.accent);
        }
        if target {
            tile = tile
                .scale(1.04)
                .outline(2.0, -2.0, theme.accent)
                .box_shadow(Shadow { x: 0.0, y: 0.0, blur: 0.0, spread: 5.0, color: theme.accent_soft, inset: false });
        }
        if arranging {
            tile = tile.children(handles(&theme, id, rect[2], rect[3], max_cols));
        }
        grid = grid.child(tile);
    }

    // The tile under the pointer while it's dragged (`makeClone`).
    if let Some(state) = active_drag {
        if let Some(i) = rects.iter().position(|(id, _)| *id == state.id) {
            let (w, h) = spans[i];
            let rect = rects[i].1;
            grid = grid.child(
                el().key("drag-clone")
                    .size(rect[2], rect[3])
                    .absolute(state.pointer[0] - state.grab[0], state.pointer[1] - state.grab[1])
                    .raise()
                    .scale(1.06)
                    .rotate(2.5_f32.to_radians())
                    .outline(2.0, -2.0, theme.accent)
                    .shadow(2.0, 6.0, Color::BLACK.alpha(0.35))
                    .shadow(20.0, 44.0, Color::BLACK.alpha(0.55))
                    .cursor(Cursor::Grabbing)
                    .transition(rune::Transition::opacity().combined(rune::Transition::scale(0.32)))
                    .child(tile_content(app, &theme, ordered[i], w, h, false)),
            );
        }
    }
    grid
}

fn tile_content(app: &App, theme: &Theme, entry: &Entry, w: u32, h: u32, arranging: bool) -> El {
    let (width_px, height_px) = (span_px(w), span_px(h));
    match entry {
        Entry::Instance(inst) => instance_card(app, inst, width_px, height_px),
        Entry::Folder { name, instances } => folder_tile(app, theme, name, instances, w, h, arranging),
    }
}

fn folder_tile(app: &App, theme: &Theme, name: &str, instances: &[&Instance], w: u32, h: u32, _arranging: bool) -> El {
    let (width_px, height_px) = (span_px(w), span_px(h));
    let cover = app.home.covers.get(name).and_then(|src| app.images.get(&app.fx, src));
    let mut tile = card_face(theme)
        .key(format!("folder-{name}"))
        .size(width_px, height_px)
        .hover_group()
        .role("button")
        .label(name.to_string())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press({
            let name = name.to_string();
            move |app: &mut App| {
                if std::mem::take(&mut app.home.suppress_click) {
                    return;
                }
                app.ui.folder_open = Some(name.clone());
            }
        })
        .on_secondary_press({
            let name = name.to_string();
            move |app: &mut App, press: PointerPress, _: &mut Tx| app.open_group_menu(&name, press.x, press.y)
        });
    let count = text(instances.len().to_string()).font(theme.sans(11.0, 400.0)).color(theme.text_muted);
    let count = el().px(6.0).fill(theme.bg_input).border(2.0, theme.border_subtle).child(count);
    if let Some(img) = cover {
        tile = tile
            .child(el().key("cover").size(width_px - 4.0, height_px - 4.0).absolute(0.0, 0.0).pixels_fit(img.pixels(), SurfaceFit::Fill))
            .child(el().key("scrim").size(width_px - 4.0, height_px - 4.0).absolute(0.0, 0.0).linear_gradient(
                0.0,
                [(0.0, Color::BLACK.alpha(0.82)), (0.34, Color::BLACK.alpha(0.35)), (0.62, Color::BLACK.alpha(0.0))],
            ))
            .child(el().grow(1.0))
            .child(
                row().key("meta").gap(6.0).center_y().px(12.0).py(10.0)
                    .child(text(name.to_string()).font(theme.sans(13.0, 600.0)).color(Color::WHITE).truncate().grow(1.0).help(name.to_string()))
                    .child(count),
            );
    } else {
        let size = folder_icon(w, h);
        let mut grid = col().gap(4.0);
        for pair in instances.chunks(2).take(2) {
            let mut icon_row = row().gap(4.0);
            for inst in pair {
                icon_row = icon_row.child(ui::instance_icon(app, inst, size));
            }
            grid = grid.child(icon_row);
        }
        tile = tile
            .padding(8.0)
            .gap(6.0)
            .child(
                el().key("preview").grow(1.0).min_h(0.0).w_full().center().padding(4.0)
                    .fill(theme.bg_input).border(2.0, theme.border_subtle).clip_corners([0.0; 4])
                    .child(grid),
            )
            .child(
                row().key("meta").gap(6.0).center_y()
                    .child(text(name.to_string()).font(theme.sans(13.0, 600.0)).color(theme.text).truncate().grow(1.0).help(name.to_string()))
                    .child(count),
            );
    }
    tile.child(el().key("ring").size(width_px, height_px).absolute(-2.0, -2.0).border(2.0, theme.accent).group_opacity(0.0, 1.0).a11y_hidden())
}

fn handles(theme: &Theme, id: &str, w: f32, h: f32, max_cols: u32) -> Vec<El> {
    let handle = |key: &str, axis: Axis| {
        let id = id.to_string();
        el().key(key.to_string()).hover_group().on_drag(move |app: &mut App, drag: &mut Drag, _: &mut Tx| on_resize(app, drag, &id, axis, max_cols))
    };
    let bar = |bar_w: f32, bar_h: f32| el().size(bar_w, bar_h).fill(theme.accent).radius(2.0).group_opacity(0.0, 0.85);
    vec![
        handle("handle-e", Axis::E)
            .size(14.0, h)
            .absolute(w - 10.0, 0.0)
            .cursor(Cursor::ResizeColumn)
            .child(row().w_full().h_full().center_y().justify_end().pr(4.0).child(bar(3.0, 26.0))),
        handle("handle-s", Axis::S)
            .size(w, 14.0)
            .absolute(0.0, h - 10.0)
            .cursor(Cursor::ResizeRow)
            .child(col().w_full().h_full().items_center().justify_end().pb(4.0).child(bar(26.0, 3.0))),
        handle("handle-se", Axis::Se)
            .size(15.0, 15.0)
            .absolute(w - 3.0 - 15.0, h - 3.0 - 15.0)
            .cursor(Cursor::Crosshair)
            .border_sides([0.0, 3.0, 3.0, 0.0], theme.accent)
            .corners([0.0, 0.0, 5.0, 0.0])
            .opacity(0.9),
    ]
}
