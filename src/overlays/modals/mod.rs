//! CreateInstanceModal, InstanceContextMenu, GroupPicker, StickerPicker, FolderOverlay,
//! Onboarding and LauncherImport. (GroupContextMenu lives with Home.)

mod create;
mod folder;
mod group_picker;
mod image;
mod instance_menu;
mod launcher_import;
mod onboarding;
mod sticker;
mod widgets;

use rune::{Cx, El, Key, KeyEvent, Tx};

use crate::app::App;
use crate::instance::UpdateInstance;

/// The `groupCovers` store, for Home's group tiles and GroupContextMenu.
#[allow(unused_imports)]
pub use folder::{group_cover, set_group_cover};

#[derive(Default)]
pub struct State {
    create: create::State,
    menu: instance_menu::State,
    group: group_picker::State,
    sticker: sticker::State,
    folder: folder::State,
    onboarding: onboarding::State,
    import: launcher_import::State,
}

/// Once, after the app state is built.
pub fn on_start(app: &mut App) {
    create::reset(app);
    onboarding::on_start(app);
}

/// A backend event, after the core stores took it.
pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

/// Window keys nothing focused took; true consumes the key.
pub fn on_key(app: &mut App, key: &KeyEvent, _tx: &mut Tx) -> bool {
    if !app.ui.onboarded && matches!(key.key, Key::Enter) {
        return onboarding::on_enter_key(app);
    }
    false
}

/// Push whatever of these overlays is open onto `layer`, in +layout.svelte's order.
pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    if app.ui.create_instance_open {
        create::view(app, cx, layer);
    }
    if app.ui.instance_menu.is_some() {
        instance_menu::view(app, layer);
    }
    if app.ui.group_for.is_some() {
        group_picker::view(app, layer);
    }
    if app.ui.folder_open.is_some() {
        folder::view(app, cx, layer);
    }
    if app.ui.launcher_import_open {
        launcher_import::view(app, layer);
    }
    if app.ui.sticker_picker.is_some() {
        sticker::view(app, layer);
    }
    if !app.ui.onboarded {
        onboarding::view(app, cx, layer);
    }
}

/// `instancesStore.update(..).catch(showError)`.
fn update(app: &mut App, id: &str, change: UpdateInstance) {
    app.update_instance(id, change, |app, result| {
        if let Err(e) = result {
            app.toasts.error(e.to_string());
        }
    });
}
