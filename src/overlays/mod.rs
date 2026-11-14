//! Everything drawn above the page: the dock, modals, menus, pickers, palettes.

pub mod accounts;
pub mod dock;
pub mod extras;
pub mod modals;

use rune::{Cx, El, Key, KeyEvent, Tx};

use crate::app::App;

pub fn on_start(app: &mut App) {
    accounts::on_start(app);
    modals::on_start(app);
    extras::on_start(app);
}

pub fn on_event(app: &mut App, name: &str, payload: &serde_json::Value) {
    accounts::on_event(app, name, payload);
    modals::on_event(app, name, payload);
    extras::on_event(app, name, payload);
}

/// `onGlobalKeydown` in +layout.svelte, then each overlay module.
pub fn on_key(app: &mut App, key: &KeyEvent, tx: &mut Tx) -> bool {
    if key.is_command('k') {
        if app.ui.command_palette_open {
            extras::close_palette(app);
        } else {
            app.ui.command_palette_open = true;
        }
        return true;
    }
    if matches!(key.key, Key::Escape) && app.ui.instance_menu.take().or(None).is_some() {
        return true;
    }
    accounts::on_key(app, key, tx) || modals::on_key(app, key, tx) || extras::on_key(app, key, tx)
}

/// In +layout.svelte's order: modals, menus, pickers, toasts, palette, changelog, crafting.
pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    modals::view(app, cx, layer);
    accounts::view(app, cx, layer);
    extras::view(app, cx, layer);
}
