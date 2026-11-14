//! `FolderOverlay.svelte`: a group opened as a folder (rename, cover, drag a card out to
//! ungroup it), plus the `groupCovers` store it edits.

use std::cell::Cell;

use rune::{Cursor, Cx, Drag, El, FileDialogOptions, Phase, TextEdit, Tx, col, el, row, text};

use super::image::{IMAGE_EXTENSIONS, file_to_icon_data_uri};
use crate::instance::UpdateInstance;
use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::screens::home::instance_card;
use crate::ui::{self, Btn};

/// The body's inner width (640 less border and padding), and the card grid in it:
/// `repeat(auto-fill, minmax(150px, 1fr))`, 12 apart, 170 tall.
const BODY_W: f32 = 600.0;
const GAP: f32 = 12.0;
const CELL_H: f32 = 170.0;

#[derive(Default)]
pub struct State {
    /// The group shown; follows renames while open.
    current: String,
    name: TextEdit,
    /// Whether the name field was focused last frame, to rename on blur.
    was_focused: Cell<bool>,
    drag: Option<DragState>,
}

#[derive(Clone)]
struct DragState {
    id: String,
    dx: f32,
    dy: f32,
    started: bool,
    over_remove: bool,
}

// groupCovers.svelte.ts: the covers live in `app.home.covers`, saved with `grid::save_covers`.

/// The cover image (a data URI) chosen for group `name`.
pub fn group_cover<'a>(app: &'a App, name: &str) -> Option<&'a String> {
    app.home.covers.get(name)
}

/// Set (or with `None`, clear) group `name`'s cover.
pub fn set_group_cover(app: &mut App, name: &str, uri: Option<String>) {
    match uri {
        Some(uri) => app.home.covers.insert(name.to_string(), uri),
        None => app.home.covers.remove(name),
    };
    crate::screens::home::save_covers(app);
}

fn rename_cover(app: &mut App, from: &str, to: &str) {
    if from != to && let Some(uri) = app.home.covers.remove(from) {
        app.home.covers.insert(to.to_string(), uri);
        crate::screens::home::save_covers(app);
    }
}

fn close(app: &mut App) {
    app.ui.folder_open = None;
    app.modals.folder = State::default();
}

fn set_members_group(app: &mut App, from: &str, to: &str) {
    let ids: Vec<String> = app.instances.iter().filter(|i| i.group.as_deref() == Some(from)).map(|i| i.id.clone()).collect();
    for id in ids {
        super::update(app, &id, UpdateInstance { group: Some(to.to_string()), ..Default::default() });
    }
}

fn rename(app: &mut App) {
    let next = app.modals.folder.name.text.trim().to_string();
    let current = app.modals.folder.current.clone();
    if next.is_empty() || next == current {
        return;
    }
    set_members_group(app, &current, &next);
    rename_cover(app, &current, &next);
    app.modals.folder.current = next.clone();
    app.ui.folder_open = Some(next);
}

fn ungroup_all(app: &mut App) {
    let current = app.modals.folder.current.clone();
    set_members_group(app, &current, "");
    close(app);
}

fn pick_cover_file(app: &mut App, tx: &mut Tx) {
    let name = app.modals.folder.current.clone();
    let options = FileDialogOptions { title: tr("folder.upload"), ..Default::default() }.filter("Image", &IMAGE_EXTENSIONS);
    tx.open_file(options, move |app: &mut App, result, _| match result {
        Ok(Some(path)) => app.fx.blocking(move |_| file_to_icon_data_uri(&path, 256), move |app, icon| match icon {
            Ok(uri) => set_group_cover(app, &name, Some(uri)),
            Err(e) => {
                app.toasts.error(e);
            }
        }),
        Ok(None) => {}
        Err(e) => {
            app.toasts.error(e.to_string());
        }
    });
}

fn pick_cover_sticker(app: &mut App) {
    let name = app.modals.folder.current.clone();
    if name.is_empty() {
        return;
    }
    let title = tf("folder.coverFor", &[("name", &name)]);
    app.open_sticker_picker(title, move |app, uri| set_group_cover(app, &name, Some(uri)));
}

/// Opening, or a rename from elsewhere: show the group and its name.
fn sync(app: &mut App) {
    if let Some(name) = app.ui.folder_open.clone()
        && name != app.modals.folder.current
    {
        app.modals.folder.current = name.clone();
        app.modals.folder.name = TextEdit::new(name);
    }
}

pub fn view(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) {
    let Some(open) = app.ui.folder_open.as_deref() else { return };
    let folder = &app.modals.folder;
    if folder.current != open {
        app.fx.post(sync);
    }
    // `onblur={rename}`.
    let focused = folder.name.focused;
    if folder.was_focused.replace(focused) && !focused {
        app.fx.post(rename);
    }
    let theme = app.theme();
    let current = if folder.current.is_empty() { open } else { folder.current.as_str() };
    let members: Vec<_> = app.instances.iter().filter(|i| i.group.as_deref() == Some(current)).collect();

    let name_input = rune::text_input(&folder.name, theme.pixel(16.0))
        .single_line()
        .label(tr("folder.nameLabel"))
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(|app: &mut App| &mut app.modals.folder.name)
        .on_submit(|app: &mut App, _: &mut Tx| rename(app))
        .grow(1.0);
    let head = row()
        .key("folder-head")
        .gap(10.0)
        .center_y()
        .mb(12.0)
        .child(ui::input_frame(&theme, focused).grow(1.0).child(name_input))
        .child(Btn::new(tr("folder.ungroupAll")).danger().sm().key("ungroup-all").on_press(ungroup_all).build(&theme));
    let mut cover = row()
        .key("cover-controls")
        .gap(8.0)
        .center_y()
        .mb(16.0)
        .child(text(tr("folder.cover")).font(theme.sans(12.0, 400.0)).color(theme.text_muted))
        .child(Btn::new(tr("folder.upload")).ghost().sm().icon(&icons::EDIT).key("cover-upload").on_press_tx(pick_cover_file).build(&theme))
        .child(Btn::new(tr("folder.stickers")).ghost().sm().icon(&icons::SPARKLES).key("cover-stickers").on_press(pick_cover_sticker).build(&theme));
    if group_cover(app, current).is_some() {
        cover = cover.child(
            Btn::new(tr("common.remove"))
                .ghost()
                .sm()
                .key("cover-remove")
                .on_press(|app| {
                    let name = app.modals.folder.current.clone();
                    set_group_cover(app, &name, None);
                })
                .build(&theme),
        );
    }
    let mut body = col().key("folder").w(BODY_W).child(head).child(cover);
    if members.is_empty() {
        body = body.child(el().key("empty").padding(24.0).center().child(text(tr("folder.empty")).font(theme.body()).color(theme.text_muted)));
    } else {
        let cols = ((BODY_W + GAP) / (150.0 + GAP)).floor().max(1.0) as usize;
        let cell_w = (BODY_W - GAP * (cols as f32 - 1.0)) / cols as f32;
        let rows = members.len().div_ceil(cols);
        let grid_h = rows as f32 * CELL_H + (rows as f32 - 1.0) * GAP;
        let mut grid = el().key("grid").w(BODY_W).h(grid_h);
        let mut floating = None;
        for (n, inst) in members.iter().enumerate() {
            let (x, y) = ((n % cols) as f32 * (cell_w + GAP), (n / cols) as f32 * (CELL_H + GAP));
            let dragging = folder.drag.as_ref().filter(|drag| drag.started && drag.id == inst.id);
            let id = inst.id.clone();
            let mut cell = el()
                .key(format!("cell-{}", inst.id))
                .absolute(x, y)
                .size(cell_w, CELL_H)
                .cursor(Cursor::Grab)
                .on_drag(move |app: &mut App, drag: &mut Drag, _tx| on_drag(app, &id, drag, x, y, BODY_W, grid_h))
                .child(instance_card(app, inst, cell_w, CELL_H));
            if let Some(drag) = dragging {
                cell = cell.opacity(0.28);
                floating = Some(
                    el().key("drag-clone")
                        .absolute(x + drag.dx, y + drag.dy)
                        .size(cell_w, CELL_H)
                        .scale(1.05)
                        .shadow(16.0, 34.0, rune::Color::BLACK.alpha(0.5))
                        .child(instance_card(app, inst, cell_w, CELL_H)),
                );
            }
            grid = grid.child(cell);
        }
        body = body.child(grid.children(floating));
        if let Some(drag) = folder.drag.as_ref().filter(|drag| drag.started) {
            let ink = if drag.over_remove { theme.danger } else { theme.text_muted };
            body = body.child(
                row()
                    .key("remove-zone")
                    .center()
                    .gap(8.0)
                    .mt(14.0)
                    .padding(16.0)
                    .border(2.0, if drag.over_remove { theme.danger } else { theme.border })
                    .fill(if drag.over_remove { theme.danger.alpha(0.08) } else { rune::Color::TRANSPARENT })
                    .child(ui::colored_icon(&icons::TRASH, 14.0, ink))
                    .child(
                        text(if drag.over_remove { tr("folder.releaseToRemove") } else { tr("folder.dragOutToRemove") })
                            .font(theme.sans(13.0, 400.0))
                            .color(ink),
                    ),
            );
        }
    }
    body = body.child(text(tr("folder.hint")).font(theme.sans(12.0, 400.0)).color(theme.text_muted).wrap().mt(14.0));
    layer.push(ui::modal(&theme, "folder-modal", tr("folder.title"), 640.0, body, None, close));
}

/// A card dragged past 6 pt lifts; released outside the grid, it leaves the group.
fn on_drag(app: &mut App, id: &str, drag: &Drag, x: f32, y: f32, grid_w: f32, grid_h: f32) {
    match drag.phase {
        Phase::Down => {
            app.modals.folder.drag = Some(DragState { id: id.to_string(), dx: 0.0, dy: 0.0, started: false, over_remove: false });
        }
        Phase::Move => {
            let Some(drag_state) = app.modals.folder.drag.as_mut() else { return };
            if !drag_state.started && drag.dx.hypot(drag.dy) < 6.0 {
                return;
            }
            drag_state.started = true;
            drag_state.dx = drag.dx;
            drag_state.dy = drag.dy;
            let (px, py) = (x + drag.start_x + drag.dx, y + drag.start_y + drag.dy);
            drag_state.over_remove = px < 0.0 || py < 0.0 || px > grid_w || py > grid_h;
        }
        _ => {
            let Some(drag_state) = app.modals.folder.drag.take() else { return };
            if drag_state.started && drag_state.over_remove && matches!(drag.phase, Phase::Up) {
                super::update(app, &drag_state.id, UpdateInstance { group: Some(String::new()), ..Default::default() });
            }
        }
    }
}
