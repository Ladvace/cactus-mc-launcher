//! `GroupContextMenu.svelte` and the `groupCovers` store: the menu a folder tile opens.

use std::path::PathBuf;

use base64::Engine;
use rune::{El, FileDialogOptions, Tx, el};
use rune_kit::{Menu, MenuItem};

use crate::app::{App, Ignore};
use crate::core::i18n::{t as tr, tf};
use crate::icons;

use super::grid::{save_covers, set_group};

const MAX_IMAGE_BYTES: u64 = 20_000_000;

fn close(app: &mut App) {
    app.ui.group_menu = None;
}

pub fn set_cover(app: &mut App, name: &str, uri: String) {
    app.home.covers.insert(name.to_string(), uri);
    save_covers(app);
}

pub fn clear_cover(app: &mut App, name: &str) {
    if app.home.covers.remove(name).is_some() {
        save_covers(app);
    }
}

/// `fileToIconDataUri`: a GIF as it is, anything else cropped to a 256 square.
fn icon_data_uri(path: PathBuf) -> Result<String, String> {
    let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if len > MAX_IMAGE_BYTES {
        return Err("that image is too large (max 20 MB)".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let to_base64 = |data: &[u8]| base64::engine::general_purpose::STANDARD.encode(data);
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif")) {
        return Ok(format!("data:image/gif;base64,{}", to_base64(&bytes)));
    }
    let img = image::load_from_memory(&bytes).map_err(|_| "could not load image".to_string())?;
    let square = img.resize_to_fill(256, 256, image::imageops::FilterType::Lanczos3);
    let mut out = std::io::Cursor::new(Vec::new());
    square.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", to_base64(out.get_ref())))
}

fn pick_file(app: &mut App, tx: &mut Tx, name: String) {
    close(app);
    let opts = FileDialogOptions::default().filter("Image", &["png", "jpg", "jpeg", "gif", "webp"]);
    tx.open_file(opts, move |app: &mut App, result, _| {
        let Ok(Some(path)) = result else { return };
        app.fx.blocking(move |_| icon_data_uri(path), move |app, result| match result {
            Ok(uri) => set_cover(app, &name, uri),
            Err(e) => app.toasts.error(e).ignore(),
        });
    });
}

fn ungroup_all(app: &mut App, name: &str) {
    let members: Vec<String> = app.instances.iter().filter(|inst| inst.group.as_deref() == Some(name)).map(|inst| inst.id.clone()).collect();
    for id in members {
        set_group(app, &id, "");
    }
    clear_cover(app, name);
}

/// Pushes the menu onto `layer` while `app.ui.group_menu` is set.
pub fn view(app: &App, layer: &mut Vec<El>) {
    let Some((name, x, y)) = app.ui.group_menu.clone() else { return };
    let kit = app.theme().kit();
    let has_cover = app.home.covers.contains_key(&name);
    let group_name = name.clone();
    let mut menu = Menu::new(format!("group-menu-{name}"))
        .standard()
        .min_width(180.0)
        .trigger(el())
        .open(true)
        .at(x, y)
        .on_close(close)
        .item(MenuItem::new(tr("group.openFolder")).icon(&icons::FOLDER).on_select({
            let group_name = group_name.clone();
            move |app: &mut App| {
                close(app);
                app.ui.folder_open = Some(group_name.clone());
            }
        }))
        .separator()
        .item(MenuItem::new(tr("group.uploadCover")).icon(&icons::EDIT).on_select_tx({
            let group_name = group_name.clone();
            move |app: &mut App, tx: &mut Tx| pick_file(app, tx, group_name.clone())
        }))
        .item(MenuItem::new(tr("group.stickersEmoji")).icon(&icons::SPARKLES).on_select({
            let group_name = group_name.clone();
            move |app: &mut App| {
                close(app);
                let target = group_name.clone();
                app.open_sticker_picker(tf("group.coverFor", &[("name", &group_name)]), move |app, uri| set_cover(app, &target, uri));
            }
        }));
    if has_cover {
        menu = menu.item(MenuItem::new(tr("group.removeCover")).icon(&icons::REFRESH).on_select({
            let group_name = group_name.clone();
            move |app: &mut App| {
                close(app);
                clear_cover(app, &group_name);
            }
        }));
    }
    menu = menu.separator().item(MenuItem::new(tr("group.ungroupAll")).icon(&icons::TRASH).on_select(move |app: &mut App| {
        close(app);
        ungroup_all(app, &group_name);
    }));
    let _ = menu.build(&kit, layer);
}
