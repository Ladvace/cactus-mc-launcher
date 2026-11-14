//! Storage (data folder, instances folder, exports, content cache, reset), Service status,
//! About, and the move-data and reset-everything modals.

use rune::{El, FileDialogOptions, Shadow, col, el, row, text};

use super::widgets::{card, card_with_head, h3, label, setting};
use super::{Service, edit};
use crate::app::{App, Ignore};
use crate::core::i18n::{t, tf};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const GITHUB: &str = "https://github.com/Ladvace/cactus-mc-launcher";
const DISCORD: &str = "https://discord.gg/bfdUaMf7Mg";
const WEBSITE: &str = "https://gianmarcocavallo.com";
const AUTHOR: &str = "Ladvace";

pub fn load_cache(app: &mut App) {
    app.settings_page.cache_loading = true;
    app.fx.blocking(|host| crate::commands::content_cache_stats(host.clone()), |app, result| {
        app.settings_page.cache = result.ok();
        app.settings_page.cache_loading = false;
    });
}

fn clear_cache(app: &mut App) {
    app.settings_page.clearing_cache = true;
    app.fx.blocking(|host| crate::commands::clear_content_cache(host.clone()), |app, result| {
        match result {
            Ok(stats) => {
                app.settings_page.cache = Some(stats);
                app.toasts.success(t("settings.cacheCleared")).ignore();
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
        app.settings_page.clearing_cache = false;
    });
}

fn clear_exports(app: &mut App) {
    app.settings_page.clearing_exports = true;
    app.fx.blocking(|host| crate::commands::clear_exports(host.clone()), |app, result| {
        match result {
            Ok(n) => app.toasts.success(tf("settings.exportsCleared", &[("count", &n.to_string())])).ignore(),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
        app.settings_page.clearing_exports = false;
    });
}

pub fn check_services(app: &mut App) {
    app.settings_page.services_checking = true;
    app.fx.spawn(async { crate::status::service_status().await }, |app, list| {
        app.settings_page.services = list
            .iter()
            .filter_map(|service| serde_json::to_value(service).ok().and_then(|value| serde_json::from_value::<Service>(value).ok()))
            .collect();
        app.settings_page.services_checking = false;
    });
}

fn any_running(app: &App) -> bool {
    app.instances.iter().any(|instance| app.is_running(&instance.id) || app.is_busy(&instance.id))
}

fn browse_data_dir(_: &mut App, tx: &mut rune::Tx) {
    tx.open_folder(FileDialogOptions { title: t("settings.chooseDataFolder"), ..Default::default() }, |app: &mut App, result, _| {
        if let Ok(Some(path)) = result {
            let folder = path.to_string_lossy().into_owned();
            if folder != app.settings_page.data_dir {
                app.settings_page.pending_data_dir = Some(folder);
                app.settings_page.data_move_open = true;
            }
        }
    });
}

fn browse_instances_dir(_: &mut App, tx: &mut rune::Tx) {
    tx.open_folder(FileDialogOptions { title: t("settings.chooseInstancesFolder"), ..Default::default() }, |app: &mut App, result, _| {
        if let Ok(Some(path)) = result {
            edit(app, |draft| draft.instances_dir = path.to_string_lossy().into_owned());
        }
    });
}

/// After the data folder moved or was wiped: what `location.reload()` did.
fn reload(app: &mut App) {
    let host = app.fx.host.clone();
    let settings = crate::commands::get_settings(host.state());
    crate::core::i18n::set_locale(&settings.language);
    app.settings = settings;
    app.refresh_instances();
    app.refresh_accounts();
    if let Some(id) = app.settings_page.unsaved_toast.take() {
        app.toasts.dismiss(id);
    }
    super::on_enter(app);
}

fn confirm_move(app: &mut App) {
    app.settings_page.moving_data = true;
    let path = app.settings_page.pending_data_dir.clone();
    app.fx.blocking(move |host| crate::commands::set_data_dir(host.clone(), host.state(), host.state(), path), |app, result| {
        app.settings_page.moving_data = false;
        app.settings_page.data_move_open = false;
        match result {
            Ok(()) => {
                // `ui-prefs.json` moved with the data; keep writing to the new copy.
                app.prefs = crate::core::prefs::Prefs::load_for(&app.fx.host);
                reload(app);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn reset_everything(app: &mut App) {
    app.settings_page.resetting = true;
    app.fx.blocking(|host| crate::commands::reset_app_data(host.clone(), host.state(), host.state(), host.state()), |app, result| {
        app.settings_page.resetting = false;
        match result {
            Ok(()) => {
                app.settings_page.reset_open = false;
                app.prefs.clear();
                // Also a pre-move copy in the default folder, or `Prefs::load_for` would adopt it.
                if let Ok(dir) = crate::paths::default_data_dir(&app.fx.host) {
                    let _ = std::fs::remove_file(dir.join("ui-prefs.json"));
                }
                app.ui.onboarded = false;
                reload(app);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn path_text(theme: &Theme, path: String) -> El {
    text(path).font(theme.pixel(11.0)).color(theme.accent).wrap_anywhere().max_w(420.0)
}

pub fn storage(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let busy = any_running(app);
    let actions = || row().gap(8.0);
    let mut rows = vec![setting(
        theme,
        "data-dir",
        true,
        label(theme, t("settings.appDataFolder"), Some(t("settings.appDataFolderDesc")))
            .child(path_text(theme, if page.data_dir.is_empty() { "…".into() } else { page.data_dir.clone() })),
        actions()
            .child(Btn::new(t("settings.openFolder")).key("open-data-dir").ghost().on_press(|app| {
                let dir = app.settings_page.data_dir.clone();
                if !dir.is_empty() {
                    if let Err(e) = open::that(&dir) {
                        app.toasts.error(e.to_string()).ignore();
                    }
                }
            }).build(theme))
            .child(Btn::new(t("settings.move")).key("move-data-dir").ghost().disabled(busy).on_press_tx(browse_data_dir).build(theme))
            .child(Btn::new(t("settings.reset")).key("reset-data-dir").ghost().disabled(busy).on_press(|app| {
                app.settings_page.pending_data_dir = None;
                app.settings_page.data_move_open = true;
            }).build(theme)),
    )];
    if busy {
        rows.push(text(t("settings.closeRunningInstances")).font(theme.sans(12.0, 400.0)).color(theme.text_muted).mb(4.0));
    }
    let draft = &page.draft;
    let mut instance_actions = actions().child(Btn::new(t("settings.browse")).key("browse-instances").ghost().on_press_tx(browse_instances_dir).build(theme));
    if !draft.instances_dir.is_empty() {
        instance_actions = instance_actions
            .child(Btn::new(t("settings.reset")).key("reset-instances-dir").ghost().on_press(|app| edit(app, |draft| draft.instances_dir.clear())).build(theme));
    }
    rows.push(setting(
        theme,
        "instances-dir",
        false,
        label(theme, t("settings.instancesFolder"), Some(t("settings.instancesFolderDesc")))
            .child(path_text(theme, if draft.instances_dir.is_empty() { t("settings.defaultAppDataFolder") } else { draft.instances_dir.clone() })),
        instance_actions,
    ));
    rows.push(setting(
        theme,
        "exports",
        false,
        label(theme, t("settings.exportedSetups"), Some(t("settings.exportedSetupsDesc"))),
        actions().child(
            Btn::new(if page.clearing_exports { t("settings.clearing") } else { t("settings.clearExports") })
                .key("clear-exports")
                .ghost()
                .disabled(page.clearing_exports)
                .on_press(clear_exports)
                .build(theme),
        ),
    ));
    rows.push(setting(
        theme,
        "content-cache",
        false,
        label(theme, t("settings.sharedContentCache"), Some(t("settings.sharedContentCacheDesc"))),
        actions()
            .child(
                Btn::new(if page.clearing_cache { t("settings.clearing") } else { t("settings.clearCache") })
                    .key("clear-cache")
                    .ghost()
                    .disabled(page.cache_loading || page.clearing_cache)
                    .on_press(clear_cache)
                    .build(theme),
            )
            .child(
                Btn::new(if page.cache_loading { "…".to_string() } else { t("settings.refresh") })
                    .key("refresh-cache")
                    .ghost()
                    .disabled(page.cache_loading)
                    .on_press(load_cache)
                    .build(theme),
            ),
    ));
    if let Some(cache) = &page.cache {
        let stat = |value: String, caption: String, accent: bool| {
            col()
                .grow(1.0)
                .w(0.0)
                .gap(3.0)
                .padding(12.0)
                .fill(theme.bg_input)
                .border(2.0, theme.border)
                .child(text(value).font(theme.pixel(18.0)).color(if accent { theme.accent } else { theme.text }))
                .child(
                    text(caption.to_uppercase())
                        .font(rune::tok::TextStyle { tracking: 11.0 * 0.04, ..theme.sans(11.0, 400.0) })
                        .color(theme.text_muted),
                )
        };
        rows.push(
            row()
                .key("cache-stats")
                .w_full()
                .gap(10.0)
                .pt(14.0)
                .border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle)
                .child(stat(cache.files.to_string(), t("settings.uniqueFiles"), false))
                .child(stat(ui::fmt::bytes(cache.bytes), t("settings.onDisk"), false))
                .child(stat(ui::fmt::bytes(cache.saved_bytes), t("settings.savedBySharing"), true)),
        );
    }
    let danger_label = col()
        .gap(2.0)
        .grow(1.0)
        .shrink()
        .child(text(t("settings.resetEverything")).font(theme.sans(13.5, 500.0)).color(theme.danger))
        .child(super::widgets::small(theme, t("settings.resetEverythingDesc")));
    rows.push(
        setting(
            theme,
            "reset-everything",
            false,
            danger_label,
            Btn::new(t("settings.resetEllipsis")).key("reset-open").danger().icon(&icons::TRASH).on_press(|app| app.settings_page.reset_open = true).build(theme),
        )
        .mt(6.0)
        .pt(16.0),
    );
    card(theme, "storage", t("settings.storage"), rows)
}

pub fn status(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let head = row()
        .w_full()
        .center_y()
        .justify_between()
        .gap(12.0)
        .child(text(t("settings.serviceStatus")).font(theme.pixel(14.0)).color(theme.text))
        .child(
            Btn::new(if page.services_checking { t("settings.checking") } else { t("settings.refresh") })
                .key("check-services")
                .ghost()
                .sm()
                .icon(&icons::REFRESH)
                .disabled(page.services_checking)
                .on_press(check_services)
                .build(theme),
        );
    let mut list = col().key("status-list").w_full().mt(4.0);
    let n = page.services.len();
    for (i, svc) in page.services.iter().enumerate() {
        let (dot_color, ring) = match svc.up {
            Some(true) => (theme.success, Some(theme.success.alpha(0.22))),
            Some(false) => (theme.danger, Some(theme.danger.alpha(0.22))),
            None => (theme.text_muted, None),
        };
        let mut dot = el().size(9.0, 9.0).radius(4.5).fill(dot_color);
        if let Some(ring) = ring {
            dot = dot.box_shadow(Shadow { x: 0.0, y: 0.0, blur: 0.0, spread: 3.0, color: ring, inset: false });
        }
        let mut state = row().child(
            text(match svc.up {
                None => t("settings.statusNotConfigured"),
                Some(true) => t("settings.statusOperational"),
                Some(false) => t("settings.statusDown"),
            })
            .font(theme.sans(12.5, 400.0))
            .color(theme.text_muted),
        );
        if let (Some(true), Some(ms)) = (svc.up, svc.latency_ms) {
            state = state.child(text(format!(" · {ms} ms")).font(theme.sans(12.5, 400.0)).color(theme.text_muted.alpha(0.7)));
        }
        let mut entry = row()
            .key(format!("status-{}", svc.id))
            .w_full()
            .center_y()
            .gap(10.0)
            .px(2.0)
            .py(9.0)
            .child(dot)
            .child(text(svc.name.clone()).font(theme.sans(13.0, 400.0)).color(theme.text).grow(1.0))
            .child(state);
        if i + 1 < n {
            entry = entry.border_sides([0.0, 0.0, 1.0, 0.0], theme.border);
        }
        list = list.child(entry);
    }
    if n == 0 {
        list = list.child(
            row().key("status-empty").w_full().justify_center().py(9.0).child(
                text(if page.services_checking { t("settings.checking") } else { t("settings.statusUnavailable") })
                    .font(theme.sans(13.0, 400.0))
                    .color(theme.text_muted),
            ),
        );
    }
    card_with_head(theme, "status", head, vec![
        text(t("settings.serviceStatusDesc")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).wrap().mt(8.0).mb(12.0),
        list,
    ])
}

pub fn about(_app: &App, theme: &Theme) -> El {
    let version = env!("CARGO_PKG_VERSION");
    let link = |key: &str, name: &str, icon: &'static rune::Icon, url: &'static str| {
        Btn::new(name).key(key).ghost().sm().icon(icon).on_press_tx(move |_, tx| {
            let _ = tx.open_url(url);
        })
    };
    let credit = row()
        .gap(4.0)
        .child(text(t("settings.createdBy")).font(theme.sans(12.5, 400.0)).color(theme.text_muted))
        .child(
            text(AUTHOR)
                .key("author-link")
                .font(theme.sans(12.5, 400.0))
                .color(theme.accent)
                .spans(vec![{
                    let mut span = rune::Span::new(0, AUTHOR.len());
                    span.underline = Some(theme.accent);
                    span
                }])
                .role("link")
                .focusable(true)
                .cursor(rune::Cursor::Pointer)
                .on_press_tx(|_: &mut App, tx| {
                    let _ = tx.open_url(WEBSITE);
                }),
        );
    card_with_head(theme, "about", h3(theme, t("settings.about")), vec![
        text(t("settings.aboutTagline")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).wrap().mb(12.0),
        setting(
            theme,
            "updates",
            true,
            label(theme, t("settings.updates"), Some(tf("settings.onVersion", &[("version", version)]))),
            // The updater store isn't in the Rune app yet.
            Btn::new(t("settings.checkForUpdates")).key("check-updates").ghost().sm().icon(&icons::REFRESH).build(theme),
        ),
        row()
            .key("about-links")
            .flex_wrap()
            .gap(8.0)
            .mb(12.0)
            .child(link("github", "GitHub", &icons::CUBE, GITHUB).build(theme))
            .child(link("discord", "Discord", &icons::USERS, DISCORD).build(theme))
            .child(
                Btn::new(t("settings.changelog"))
                    .key("changelog")
                    .ghost()
                    .sm()
                    .icon(&icons::CLOCK)
                    .on_press(|app| app.ui.changelog_open = true)
                    .build(theme),
            ),
        credit,
    ])
}

pub fn modals(app: &App, theme: &Theme, layer: &mut Vec<El>) {
    let page = &app.settings_page;
    let warn = |before: String, bold: String, after: String| {
        text(format!("{before} {bold} {after}"))
            .font(rune::tok::TextStyle { line_height: 14.0 * 1.6, ..theme.sans(14.0, 400.0) })
            .color(theme.text_secondary)
            .spans(vec![rune::Span::new(before.len() + 1, before.len() + 1 + bold.len()).weight(700.0).color(theme.text)])
            .wrap()
            .mb(18.0)
    };
    if page.data_move_open {
        let body = col()
            .w_full()
            .child(warn(
                t("settings.moveDataWarn1"),
                page.pending_data_dir.clone().unwrap_or_else(|| t("settings.theDefaultFolder")),
                t("settings.moveDataWarn2"),
            ))
            .child(
                row()
                    .w_full()
                    .justify_end()
                    .gap(10.0)
                    .child(Btn::new(t("common.cancel")).key("move-cancel").ghost().on_press(|app| app.settings_page.data_move_open = false).build(theme))
                    .child(
                        Btn::new(if page.moving_data { t("settings.moving") } else { t("settings.moveAndReload") })
                            .key("move-confirm")
                            .primary()
                            .disabled(page.moving_data)
                            .on_press(confirm_move)
                            .build(theme),
                    ),
            );
        layer.push(ui::modal(theme, "move-data-modal", t("settings.moveDataTitle"), 440.0, body, None, |app| app.settings_page.data_move_open = false));
    }
    if page.reset_open {
        let body = col()
            .w_full()
            .child(warn(t("settings.resetWarn1"), t("settings.resetWarnBold"), t("settings.resetWarn2")))
            .child(
                row()
                    .w_full()
                    .justify_end()
                    .gap(10.0)
                    .child(Btn::new(t("common.cancel")).key("reset-cancel").ghost().on_press(|app| app.settings_page.reset_open = false).build(theme))
                    .child(
                        Btn::new(if page.resetting { t("settings.resetting") } else { t("settings.deleteEverything") })
                            .key("reset-confirm")
                            .danger()
                            .disabled(page.resetting)
                            .on_press(reset_everything)
                            .build(theme),
                    ),
            );
        layer.push(ui::modal(theme, "reset-modal", t("settings.resetEverythingTitle"), 430.0, body, None, |app| app.settings_page.reset_open = false));
    }
}
