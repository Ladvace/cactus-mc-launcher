//! `InstanceContextMenu.svelte`: what a right-click on an instance offers.

use rune::{El, FileDialogOptions, Tx};

use super::image::{IMAGE_EXTENSIONS, file_to_icon_data_uri};
use super::update;
use super::widgets::{Entry, context_menu};
use crate::instance::UpdateInstance;
use crate::app::{App, Route};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::instance::InstanceKind;

#[derive(Default)]
pub struct State {}

fn close(app: &mut App) {
    app.ui.instance_menu = None;
}

pub fn view(app: &App, layer: &mut Vec<El>) {
    let Some((id, x, y)) = app.ui.instance_menu.clone() else { return };
    let Some(inst) = app.instance(&id) else { return };
    let theme = app.theme();
    let running = app.is_running(&id);
    let busy = app.is_busy(&id);
    let has_icon = inst.icon.as_deref().is_some_and(|icon| !icon.is_empty());
    let mut items = Vec::new();

    let inst_id = id.clone();
    items.push(
        Entry::item(
            if running { tr("instanceMenu.stop") } else if busy { tr("instanceMenu.preparing") } else { tr("instanceMenu.play") },
            if running { &icons::STOP } else { &icons::PLAY },
            move |app| if app.is_running(&inst_id) { app.stop_instance(&inst_id) } else { app.launch_instance(&inst_id) },
        )
        .disabled(busy && !running),
    );
    let inst_id = id.clone();
    items.push(Entry::item(tr("instanceMenu.open"), &icons::FOLDER, move |app| app.go(Route::Instance(inst_id.clone()))));
    let inst_id = id.clone();
    items.push(Entry::item(tr("instanceMenu.openFolder"), &icons::CUBE, move |app| open_folder(app, &inst_id)));
    let inst_id = id.clone();
    if inst.kind == InstanceKind::Server {
        items.push(Entry::item_tx(tr("instanceMenu.copyServerAddress"), &icons::COPY, move |app, tx| copy_server_address(app, &inst_id, tx)));
    } else {
        items.push(Entry::item(tr("instanceMenu.createServer"), &icons::GLOBE, move |app| create_server(app, &inst_id)));
    }
    let inst_id = id.clone();
    items.push(Entry::item(
        match &inst.group {
            Some(group) if !group.is_empty() => tf("instanceMenu.groupLabel", &[("group", group)]),
            _ => tr("instanceMenu.moveToGroup"),
        },
        &icons::FOLDER,
        move |app| app.open_group_picker(&inst_id),
    ));
    items.push(Entry::Sep);
    let inst_id = id.clone();
    items.push(Entry::item_tx(tr("instanceMenu.uploadImage"), &icons::EDIT, move |_, tx| pick_file(&inst_id, tx)));
    let (inst_id, name) = (id.clone(), inst.name.clone());
    items.push(Entry::item(tr("instanceMenu.stickersEmoji"), &icons::SPARKLES, move |app| {
        let inst_id = inst_id.clone();
        app.open_sticker_picker(tf("instanceMenu.imageFor", &[("name", &name)]), move |app, uri| update(app, &inst_id, UpdateInstance { icon: Some(uri), ..Default::default() }));
    }));
    let (inst_id, cover) = (id.clone(), inst.cover_image);
    items.push(
        Entry::item(
            if cover { tr("instanceMenu.shrinkImage") } else { tr("instanceMenu.fillTile") },
            &icons::EXPAND,
            move |app| update(app, &inst_id, UpdateInstance { cover_image: Some(!cover), ..Default::default() }),
        )
        .disabled(!has_icon),
    );
    let inst_id = id.clone();
    items.push(Entry::item(tr("instanceMenu.resetImage"), &icons::REFRESH, move |app| update(app, &inst_id, UpdateInstance { icon: Some(String::new()), ..Default::default() })).disabled(!has_icon));
    items.push(Entry::Sep);
    let inst_id = id.clone();
    items.push(Entry::item(tr("instanceMenu.exportToFile"), &icons::UPLOAD, move |app| export_setup(app, &inst_id, "cactuspack")));
    let inst_id = id.clone();
    items.push(Entry::item(tr("instanceMenu.exportMrpack"), &icons::UPLOAD, move |app| export_setup(app, &inst_id, "mrpack")));

    context_menu(&theme, "instance-menu", x, y, items, close, layer);
}

/// `revealPath(await api.instanceFolder(id))`.
fn open_folder(app: &mut App, id: &str) {
    let id = id.to_string();
    app.fx.blocking(move |host| crate::commands::instance_folder(host.clone(), id), |app, result| match result {
        Ok(path) => {
            if let Err(e) = open::that(&path) {
                app.toasts.error(e.to_string());
            }
        }
        Err(e) => {
            app.toasts.error(e.to_string());
        }
    });
}

fn create_server(app: &mut App, id: &str) {
    let id = id.to_string();
    app.fx.blocking(move |host| crate::commands::create_server_from(host.clone(), host.state(), id), |app, result| match result {
        Ok(server) => {
            app.refresh_instances();
            app.toasts.success(tr("instanceMenu.serverCreated"));
            app.go(Route::Instance(server.id));
        }
        Err(e) => {
            app.toasts.error(e.to_string());
        }
    });
}

/// `localServerAddress`: localhost, with the port when server.properties moves it off 25565.
fn copy_server_address(app: &mut App, id: &str, tx: &mut Tx) {
    let props = crate::commands::read_server_properties(app.fx.host.clone(), id.to_string()).unwrap_or_default();
    let port = props
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.strip_prefix("server-port")?.trim_start().strip_prefix('=')?.trim().parse::<u16>().ok())
        .filter(|p| *p != 0)
        .unwrap_or(25565);
    let addr = if port == 25565 { "localhost".to_string() } else { format!("localhost:{port}") };
    tx.copy(addr.clone());
    app.toasts.success(tf("instanceMenu.copiedAddr", &[("addr", &addr)]));
}

/// The hidden `<input type=file>`: pick an image, then make it the icon.
fn pick_file(id: &str, tx: &mut Tx) {
    let id = id.to_string();
    let options = FileDialogOptions { title: tr("instanceMenu.uploadImage"), ..Default::default() }.filter("Image", &IMAGE_EXTENSIONS);
    tx.open_file(options, move |app: &mut App, result, _| match result {
        Ok(Some(path)) => app.fx.blocking(move |_| file_to_icon_data_uri(&path, 256), move |app, result| match result {
            Ok(uri) => update(app, &id, UpdateInstance { icon: Some(uri), ..Default::default() }),
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

fn export_setup(app: &mut App, id: &str, format: &'static str) {
    let host = app.fx.host.clone();
    let id = id.to_string();
    app.fx.spawn(async move { crate::commands::export_setup(host, id, format.to_string(), None).await }, |app, result| match result {
        Ok(export) => {
            app.toasts.success(if export.skipped.is_empty() {
                tr("instanceMenu.setupExported")
            } else {
                tf("instanceMenu.exportedSkipped", &[("count", &export.skipped.len().to_string())])
            });
            // `revealPath(result.path, { silent: true })`: show the file's folder.
            if let Some(dir) = std::path::Path::new(&export.path).parent() {
                let _ = open::that(dir);
            }
        }
        Err(e) => {
            app.toasts.error(e.to_string());
        }
    });
}
