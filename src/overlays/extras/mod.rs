//! CommandPalette, ChangelogModal, UpdatePrompt, CraftingTable and the animated backgrounds (ReactiveGrid, NoiseGrid, the `anim:*` presets) and DecorLayer.

mod background;
mod changelog;
mod crafting;
mod decor;
mod palette;
pub mod updater;

pub use palette::close as close_palette;

use rune::{Cx, El, KeyEvent, Tx};

use crate::app::App;

#[derive(Default)]
pub struct State {
    pub palette: palette::State,
    pub crafting: crafting::State,
    pub updater: updater::State,
    bg: background::State,
    decor: decor::State,
}

/// Once, after the app state is built.
pub fn on_start(app: &mut App) {
    crafting::on_start(app);
    updater::on_start(app);
}

/// A backend event, after the core stores took it.
pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

/// Window keys nothing focused took; true consumes the key.
pub fn on_key(app: &mut App, key: &KeyEvent, tx: &mut Tx) -> bool {
    palette::on_key(app, key, tx)
}

/// Push whatever of these overlays is open onto `layer`, in +layout.svelte's order.
pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    updater::view(app, cx, layer);
    palette::view(app, cx, layer);
    changelog::view(app, cx, layer);
    crafting::view(app, cx, layer);
}

/// The window background under everything (`.bg-layer` with `backgroundCss` / `animClass` of
/// `src/lib/background.ts`, ReactiveGrid, NoiseGrid). Fills its parent.
pub fn background(app: &App, cx: &mut Cx<App>) -> El {
    background::view(app, cx)
}

/// `DecorLayer.svelte`: the decor theme's sprites above the content, never hit.
pub fn decor(app: &App, cx: &mut Cx<App>) -> Option<El> {
    decor::view(app, cx)
}

/// The window's size in points: the background layer fills it.
fn window_size(cx: &Cx<App>) -> [f32; 2] {
    cx.layout_size(background::KEY).unwrap_or([1180.0, 760.0])
}
