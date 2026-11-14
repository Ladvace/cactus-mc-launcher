//! Settings: `src/routes/settings/+page.svelte`, LanguageSelect, Flag.
//!
//! As the Svelte page: a `draft` of the settings that the Save button (or the "unsaved changes"
//! toast) writes, while some controls save their one setting at once (`save_one`).

mod appearance;
mod bg;
mod data;
mod general;
mod java;
mod widgets;

use std::collections::HashMap;

use rune::{Cx, El, ScrollSpec, TextEdit, col, row, text};
use serde::Deserialize;

use crate::app::App;
use crate::content::CacheStats;
use crate::core::i18n::t;
use crate::core::toast::ToastKind;
use crate::settings::Settings;
use crate::theme::Theme;
use crate::ui::Btn;

pub const JAVA_MAJORS: [u32; 3] = [8, 17, 21];

#[derive(Default)]
pub struct JavaSetup {
    pub busy: bool,
    pub label: String,
    pub current: usize,
    pub total: usize,
    pub installed: Vec<String>,
    pub error: Option<String>,
}

/// `ServiceStatus` from status.rs, read back from its JSON.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: String,
    pub name: String,
    pub up: Option<bool>,
    pub latency_ms: Option<u64>,
}

#[derive(Default)]
pub struct State {
    pub draft: Settings,
    saved: bool,
    unsaved_toast: Option<u64>,
    animated_tab: bool,
    /// The open dropdown: "lang" | "date" | "loader".
    pub open: Option<&'static str>,

    username: TextEdit,
    jvm_args: TextEdit,
    game_width: TextEdit,
    game_height: TextEdit,
    giphy: TextEdit,
    ngrok: TextEdit,
    /// The `colorField` hex of the current background kind.
    hex: TextEdit,
    java_paths: [TextEdit; 3],
    /// `javaEditing`: a major switched to "own Java" this visit.
    java_editing: HashMap<u32, bool>,
    managed_paths: HashMap<u32, String>,
    java: JavaSetup,

    cache: Option<CacheStats>,
    cache_loading: bool,
    clearing_cache: bool,
    clearing_exports: bool,
    data_dir: String,
    pending_data_dir: Option<String>,
    data_move_open: bool,
    moving_data: bool,
    reset_open: bool,
    resetting: bool,
    services: Vec<Service>,
    services_checking: bool,
}

/// Mount: seed the draft and its fields from the settings, load what the page shows.
pub fn on_enter(app: &mut App) {
    let page = &mut app.settings_page;
    page.draft = app.settings.clone();
    page.animated_tab = page.draft.background.starts_with("anim:");
    page.open = None;
    page.java_editing.clear();
    seed_fields(app);
    data::load_cache(app);
    data::check_services(app);
    app.fx.blocking(|host| crate::commands::get_data_dir(host.clone()), |app, result| {
        if let Ok(dir) = result {
            app.settings_page.data_dir = dir;
        }
    });
    java::load_managed(app);
}

pub fn on_event(app: &mut App, name: &str, payload: &serde_json::Value) {
    if name == "java-setup" {
        #[derive(Deserialize)]
        struct JavaProgress {
            label: String,
            current: usize,
            total: usize,
        }
        if let Ok(progress) = serde_json::from_value::<JavaProgress>(payload.clone()) {
            let java = &mut app.settings_page.java;
            java.label = progress.label;
            java.current = progress.current;
            java.total = progress.total;
        }
    }
}

fn seed_fields(app: &mut App) {
    let page = &mut app.settings_page;
    let draft = &page.draft;
    page.username = TextEdit::new(draft.offline_username.clone());
    page.jvm_args = TextEdit::new(draft.jvm_args.clone());
    page.game_width = TextEdit::new(draft.game_width.to_string());
    page.game_height = TextEdit::new(draft.game_height.to_string());
    page.giphy = TextEdit::new(draft.giphy_api_key.clone());
    page.ngrok = TextEdit::new(draft.ngrok_authtoken.clone());
    page.hex = TextEdit::new(appearance::current_color(draft).to_uppercase());
    for (i, major) in JAVA_MAJORS.iter().enumerate() {
        page.java_paths[i] = TextEdit::new(draft.java_paths.get(major).cloned().unwrap_or_default());
    }
}

/// For a field's `on_edit`: after the runtime applies the edit, copy the fields into the draft.
pub(super) fn field(app: &mut App) -> &mut App {
    app.fx.post(sync);
    app
}

/// The fields into the draft (`bind:value`), the credentials saved once they lose focus
/// (`persistCredential` on blur).
fn sync(app: &mut App) {
    let page = &mut app.settings_page;
    page.draft.offline_username = page.username.text.clone();
    page.draft.jvm_args = page.jvm_args.text.clone();
    if let Ok(w) = page.game_width.text.trim().parse() {
        page.draft.game_width = w;
    }
    if let Ok(h) = page.game_height.text.trim().parse() {
        page.draft.game_height = h;
    }
    page.draft.giphy_api_key = page.giphy.text.clone();
    page.draft.ngrok_authtoken = page.ngrok.text.clone();
    let hex = page.hex.text.trim().to_lowercase();
    if bg::is_hex(&hex) && hex != appearance::current_color(&page.draft).to_lowercase() {
        appearance::set_color(&mut page.draft, &hex);
    }
    for (i, major) in JAVA_MAJORS.iter().enumerate() {
        if !java::managed(page, *major) {
            let path = page.java_paths[i].text.clone();
            java::set_path(&mut page.draft, *major, &path);
        }
    }
    if !app.settings_page.giphy.focused {
        persist_credential(app, true);
    }
    if !app.settings_page.ngrok.focused {
        persist_credential(app, false);
    }
    changed(app);
}

fn persist_credential(app: &mut App, giphy: bool) {
    let page = &mut app.settings_page;
    let (edit, saved) = if giphy { (&mut page.giphy, &app.settings.giphy_api_key) } else { (&mut page.ngrok, &app.settings.ngrok_authtoken) };
    let value = edit.text.trim().to_string();
    if edit.text != value {
        edit.replace_text(value.clone());
    }
    if &value == saved {
        return;
    }
    let mut next = page.draft.clone();
    if giphy {
        page.draft.giphy_api_key = value.clone();
        next.giphy_api_key = value;
    } else {
        page.draft.ngrok_authtoken = value.clone();
        next.ngrok_authtoken = value;
    }
    app.save_settings(next);
}

/// `settingsEqual(draft, settings)`, negated.
pub(super) fn dirty(app: &App) -> bool {
    serde_json::to_value(&app.settings_page.draft).ok() != serde_json::to_value(&app.settings).ok()
}

/// The one "unsaved changes" toast, shown while the draft differs and dismissed once it doesn't.
pub(super) fn changed(app: &mut App) {
    let page = &mut app.settings_page;
    if !page.hex.focused {
        let color = appearance::current_color(&page.draft).to_uppercase();
        if page.hex.text != color {
            page.hex.replace_text(color);
        }
    }
    let is_dirty = dirty(app);
    match app.settings_page.unsaved_toast {
        None if is_dirty => {
            let id = app.toasts.with_action(ToastKind::Info, t("settings.unsavedChanges"), t("settings.saveChanges"), save);
            app.settings_page.unsaved_toast = Some(id);
        }
        Some(id) if !is_dirty => {
            app.toasts.dismiss(id);
            app.settings_page.unsaved_toast = None;
        }
        _ => {}
    }
}

/// Change the draft only; Save writes it.
pub(super) fn edit(app: &mut App, f: impl FnOnce(&mut Settings)) {
    f(&mut app.settings_page.draft);
    changed(app);
}

/// `saveOne`: the draft and the saved settings both take the change now.
pub(super) fn save_one(app: &mut App, f: impl Fn(&mut Settings)) {
    f(&mut app.settings_page.draft);
    app.update_settings(|settings| f(settings));
    changed(app);
}

pub(super) fn save(app: &mut App) {
    let next = app.settings_page.draft.clone();
    app.save_settings(next);
    if !dirty(app) {
        app.settings_page.saved = true;
        app.fx.after(std::time::Duration::from_millis(1800), |app| app.settings_page.saved = false);
    }
    changed(app);
}

pub fn view(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let page = col()
        .key("settings-page")
        .w_full()
        .max_w(760.0)
        .px(32.0)
        .py(28.0)
        .child(row().key("head").w_full().center_y().justify_between().mb(24.0)
            .child(text(t("settings.title")).font(theme.pixel(24.0)).color(theme.text))
            .child(save_actions(app, &theme, "head-save")))
        .child(general::account(app, &theme))
        .child(general::general(app, &theme, layer))
        .child(general::sound(app, &theme))
        .child(general::news(app, &theme))
        .child(general::dock(app, &theme))
        .child(general::accessibility(app, &theme))
        .child(appearance::view(app, &theme))
        .child(java::view(app, &theme))
        .child(general::game_window(app, &theme))
        .child(general::servers(app, &theme))
        .child(data::storage(app, &theme))
        .child(data::status(app, &theme))
        .child(data::about(app, &theme))
        .child(row().key("save-bar").w_full().justify_end().pt(4.0).child(save_actions(app, &theme, "bar-save")))
        .child(
            text(format!("Cactus Launcher v{}", env!("CARGO_PKG_VERSION")))
                .font(theme.sans(11.0, 400.0))
                .color(theme.text_muted)
                .self_center()
                .mt(20.0),
        );
    data::modals(app, &theme, layer);
    col()
        .key("settings")
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(row().w_full().justify_center().child(page))
}

/// `.head-actions` / `.save-bar`: "Saved", "Unsaved changes", Save.
fn save_actions(app: &App, theme: &Theme, key: &str) -> El {
    let mut actions = row().center_y().gap(12.0);
    if app.settings_page.saved {
        actions = actions.child(text(t("settings.saved")).font(theme.sans(13.0, 500.0)).color(theme.accent));
    }
    if dirty(app) {
        actions = actions.child(text(t("settings.unsavedChanges")).font(theme.sans(12.5, 500.0)).color(theme.warning));
    }
    actions.child(Btn::new(t("settings.saveChanges")).key(key).primary().on_press(save).build(theme))
}
