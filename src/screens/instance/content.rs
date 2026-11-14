//! The Content tab: installed mods / packs / shaders, updates and restore points.

use std::collections::HashSet;

use rune::{Cursor, El, col, el, row, text};

use super::{icon_btn, t_};
use crate::app::{App, Ignore, Route};
use crate::content::ContentItem;
use crate::core::i18n::tf;
use crate::core::toast::ToastKind;
use crate::icons;
use crate::instance::{Instance, ModLoader};
use crate::restore::{ContentUpdate, RestorePoint};
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    pub items: Vec<ContentItem>,
    pub loading: bool,
    pub loaded: bool,
    pub shared: HashSet<String>,
    /// "all" or a project type.
    pub filter: String,
    pub updates: Vec<ContentUpdate>,
    pub checking: bool,
    pub updating_id: Option<String>,
    pub applying_all: bool,
    pub restore_points: Vec<RestorePoint>,
    pub show_restore: bool,
    pub restoring_id: Option<String>,
}

const TYPES: [(&str, &str); 4] = [
    ("mod", "browse.tabMods"),
    ("resourcepack", "browse.tabResourcePacks"),
    ("shader", "browse.tabShaders"),
    ("datapack", "browse.tabDatapacks"),
];

fn page_id(app: &App) -> String {
    app.instance_page.id.clone()
}

pub fn load(app: &mut App) {
    let id = page_id(app);
    if id.is_empty() {
        return;
    }
    app.instance_page.content.loading = true;
    app.fx.blocking(
        move |host| {
            let items = crate::commands::list_content(host.clone(), id.clone());
            let shared = crate::commands::shared_content(host.clone(), id.clone()).unwrap_or_default();
            (id, items, shared)
        },
        |app, (id, items, shared)| {
            if app.instance_page.id != id {
                return;
            }
            let content = &mut app.instance_page.content;
            content.loading = false;
            match items {
                Ok(items) => {
                    content.items = items;
                    content.loaded = true;
                    content.shared = shared.into_iter().collect();
                }
                Err(e) => app.toasts.error(e.to_string()).ignore(),
            }
        },
    );
}

pub fn refresh_restore_points(app: &mut App) {
    let id = page_id(app);
    app.fx.blocking(
        move |host| (crate::commands::list_restore_points(host.clone(), id.clone()), id),
        |app, (result, id)| {
            if app.instance_page.id == id {
                if let Ok(points) = result {
                    app.instance_page.content.restore_points = points;
                }
            }
        },
    );
}

fn open_browse(app: &mut App) {
    app.ui.browse_instance_id = Some(page_id(app));
    app.go(Route::Browse);
}

fn toggle(app: &mut App, version_id: String, enabled: bool) {
    let id = page_id(app);
    app.fx.blocking(
        move |host| crate::commands::set_content_enabled(host.clone(), id, version_id, !enabled),
        |app, result| match result {
            Ok(()) => load(app),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        },
    );
}

fn remove(app: &mut App, version_id: String) {
    let id = page_id(app);
    app.fx.blocking(
        move |host| crate::commands::remove_content(host.clone(), id, version_id),
        |app, result| match result {
            Ok(()) => load(app),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        },
    );
}

fn check_updates(app: &mut App) {
    app.instance_page.content.checking = true;
    let id = page_id(app);
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::check_content_updates(host, id).await }, |app, result| {
        let content = &mut app.instance_page.content;
        content.checking = false;
        match result {
            Ok(list) => {
                let none = list.is_empty();
                content.updates = list;
                if none {
                    app.toasts.success(t_("content.upToDate"));
                }
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

/// Every update auto-creates a restore point first, so it's always undoable.
fn apply_updates(app: &mut App, list: Vec<ContentUpdate>, single: Option<String>) {
    if list.is_empty() {
        return;
    }
    let content = &mut app.instance_page.content;
    match &single {
        Some(version_id) => content.updating_id = Some(version_id.clone()),
        None => content.applying_all = true,
    }
    let id = page_id(app);
    let done: HashSet<String> = list.iter().map(|update| update.version_id.clone()).collect();
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::apply_content_updates(host, id, list).await }, move |app, result| {
        let content = &mut app.instance_page.content;
        content.updating_id = None;
        content.applying_all = false;
        match result {
            Ok(outcome) => {
                content.updates.retain(|update| !done.contains(&update.version_id));
                load(app);
                refresh_restore_points(app);
                let msg = if outcome.skipped.is_empty() {
                    tf("content.updatedN", &[("n", &outcome.updated.to_string())])
                } else {
                    tf("content.updatedSomeFailed", &[("done", &outcome.updated.to_string()), ("failed", &outcome.skipped.len().to_string())])
                };
                let point = outcome.restore_point_id;
                app.toasts.with_action(ToastKind::Success, msg, t_("content.undo"), move |app| restore(app, point.clone()));
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn restore(app: &mut App, point: String) {
    app.instance_page.content.restoring_id = Some(point.clone());
    let id = page_id(app);
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::restore_instance(host, id, point).await }, |app, result| {
        app.instance_page.content.restoring_id = None;
        match result {
            Ok(()) => {
                load(app);
                refresh_restore_points(app);
                app.instance_page.content.updates.clear();
                app.toasts.success(t_("content.restored"));
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn create_point(app: &mut App) {
    let id = page_id(app);
    let label = t_("content.manualPoint");
    app.fx.blocking(
        move |host| crate::commands::create_restore_point(host.clone(), id, label),
        |app, result| match result {
            Ok(_) => {
                refresh_restore_points(app);
                app.instance_page.content.show_restore = true;
                app.toasts.success(t_("content.restorePointSaved"));
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        },
    );
}

fn delete_point(app: &mut App, point: String) {
    let id = page_id(app);
    app.fx.blocking(
        move |host| crate::commands::delete_restore_point(host.clone(), id, point),
        |app, result| match result {
            Ok(()) => refresh_restore_points(app),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        },
    );
}

/// CSS `text-transform: capitalize`.
fn capitalize(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    let mut start = true;
    for ch in words.chars() {
        if start {
            out.extend(ch.to_uppercase());
        } else {
            out.push(ch);
        }
        start = ch.is_whitespace();
    }
    out
}

pub fn view(app: &App, theme: &Theme, inst: &Instance) -> El {
    let content = &app.instance_page.content;
    let n = content.items.len();
    let count = if n == 1 { "instance.itemInstalled" } else { "instance.itemsInstalled" };

    let mut actions = row().gap(8.0).center_y();
    if !content.updates.is_empty() {
        let label = if content.applying_all { t_("instance.updating") } else { tf("content.updateAll", &[("n", &content.updates.len().to_string())]) };
        actions = actions.child(
            Btn::new(label).primary().sm().key("update-all").disabled(content.applying_all).on_press(|app| {
                let list = app.instance_page.content.updates.clone();
                apply_updates(app, list, None);
            }).build(theme),
        );
    }
    if n > 0 {
        let label = if content.checking { t_("instance.checking") } else { t_("instance.checkForUpdates") };
        actions = actions.child(Btn::new(label).ghost().sm().key("check-updates").disabled(content.checking).on_press(check_updates).build(theme));
        let points = content.restore_points.len();
        let restore_btn = if points > 0 { Btn::new(points.to_string()).icon(&icons::CLOCK) } else { Btn::icon_only(&icons::CLOCK) };
        let panel_open = content.show_restore;
        let toggle = restore_btn.ghost().sm().key("restore-toggle").help(t_("content.restorePoints")).on_press(|app| {
                let content = &mut app.instance_page.content;
                content.show_restore = !content.show_restore;
            }).build(theme).label(t_("content.restorePoints"));
        // `.btn.on`: the panel is open.
        actions = actions.child(if panel_open { toggle.outline(2.0, 0.0, theme.accent) } else { toggle });
    }
    actions = actions.child(Btn::new(t_("instance.browseModrinth")).ghost().sm().icon(&icons::COMPASS).key("browse").on_press(open_browse).build(theme));

    let mut tab = col()
        .w_full()
        .child(row().w_full().justify_between().center_y().mb(14.0).child(ui::muted(theme, tf(count, &[("count", &n.to_string())]), 14.0)).child(actions));

    if content.show_restore {
        tab = tab.child(restore_panel(theme, content, &app.settings.date_format));
    }

    if content.loading && !content.loaded {
        let mut list = col().gap(8.0);
        for i in 0..4 {
            list = list.child(
                row().key(format!("sk-{i}")).gap(12.0).center_y().px(12.0).py(10.0).fill(theme.bg_card).border(2.0, theme.border)
                    .child(ui::skeleton(theme, Some(36.0), 36.0))
                    .child(col().grow(1.0).gap(5.0).child(ui::skeleton(theme, Some(300.0), 13.0)).child(ui::skeleton(theme, Some(480.0), 10.0))),
            );
        }
        return tab.child(list);
    }

    if n == 0 {
        let vanilla = inst.loader == ModLoader::Vanilla;
        let mut empty = col()
            .key("content-empty")
            .w_full()
            .items_center()
            .gap(12.0)
            .padding(48.0)
            .child(ui::image(app, "/empty-cactus.png", 200.0, 120.0))
            .child(ui::secondary(theme, t_("instance.noContent"), 14.0));
        if vanilla {
            empty = empty.child(
                text(t_("instance.vanillaNoMods")).font(theme.sans(12.5, 400.0)).color(theme.text_muted).wrap().align(rune::Align::Center).max_w(400.0),
            );
        }
        let label = if vanilla { t_("instance.browseModrinth") } else { t_("instance.findMods") };
        return tab.child(empty.child(row().justify_center().child(Btn::new(label).primary().icon(&icons::COMPASS).key("find-mods").on_press(open_browse).build(theme))));
    }

    let present: Vec<(&str, &str)> = TYPES.iter().copied().filter(|(ty, _)| content.items.iter().any(|item| item.project_type == *ty)).collect();
    let filter = if present.iter().any(|(ty, _)| *ty == content.filter) { content.filter.as_str() } else { "all" };
    if present.len() > 1 {
        let mut subtabs = row().flex_wrap().gap(6.0).mb(12.0).child(subtab(theme, "all", ui_t("content.allTypes"), n, filter == "all"));
        for (ty, key) in &present {
            let type_count = content.items.iter().filter(|item| item.project_type == *ty).count();
            subtabs = subtabs.child(subtab(theme, ty, ui_t(key), type_count, filter == *ty));
        }
        tab = tab.child(subtabs);
    }

    let mut list = col().key("content-list").gap(8.0);
    for item in content.items.iter().filter(|item| filter == "all" || item.project_type == filter) {
        list = list.child(content_row(app, theme, content, item));
    }
    tab.child(list)
}

fn ui_t(key: &str) -> String {
    t_(key)
}

fn subtab(theme: &Theme, ty: &str, label: String, count: usize, active: bool) -> El {
    let value = ty.to_string();
    row()
        .key(format!("subtab-{ty}"))
        .gap(6.0)
        .center_y()
        .px(11.0)
        .py(5.0)
        .fill(if active { theme.accent_soft } else { theme.bg_input })
        .border(2.0, if active { theme.accent } else { theme.border })
        .role("button")
        .label(label.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover(if active { theme.accent_soft } else { theme.bg_hover })
        .on_press(move |app: &mut App| app.instance_page.content.filter = value.clone())
        .child(text(label).font(theme.sans(12.5, 400.0)).color(if active { theme.accent } else { theme.text_secondary }))
        .child(text(count.to_string()).font(theme.sans(11.0, 400.0)).color(if active { theme.accent } else { theme.text_muted }))
}

fn content_row(app: &App, theme: &Theme, content: &State, item: &ContentItem) -> El {
    let version_id = item.version_id.clone();
    let icon = match item.icon_url.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => {
            let frame = el().size(36.0, 36.0).fill(theme.bg_input).border(2.0, rune::Color::BLACK.alpha(0.3));
            match app.images.get(&app.fx, url) {
                Some(img) => frame.pixels(img.pixels()),
                None => frame,
            }
        }
        None => el().size(36.0, 36.0).center().fill(theme.bg_input).border(2.0, rune::Color::BLACK.alpha(0.3)).child(ui::colored_icon(&icons::PACKAGE, 16.0, theme.text_muted)),
    };
    let info = col()
        .grow(1.0)
        .shrink()
        .gap(2.0)
        .child(text(item.title.clone()).font(theme.sans(13.5, 600.0)).color(theme.text).truncate())
        .child(text(capitalize(&format!("{} · {}", item.project_type, item.file_name))).font(theme.sans(11.5, 400.0)).color(theme.text_muted).truncate());
    let mut entry = row()
        .key(format!("item-{version_id}"))
        .gap(12.0)
        .center_y()
        .px(12.0)
        .py(10.0)
        .fill(theme.bg_card)
        .border(2.0, theme.border)
        .child(icon)
        .child(info);
    if content.shared.contains(&version_id) {
        entry = entry.child(el().help(t_("instance.sharedTooltip")).label(t_("instance.sharedTooltip")).child(ui::colored_icon(&icons::COPY, 13.0, theme.text_muted)));
    }
    if let Some(update) = content.updates.iter().find(|update| update.version_id == version_id) {
        let busy = content.updating_id.as_deref() == Some(version_id.as_str());
        let pending = update.clone();
        let target = version_id.clone();
        entry = entry.child(
            Btn::new(if busy { t_("instance.updating") } else { t_("instance.update") })
                .primary()
                .sm()
                .key(format!("update-{version_id}"))
                .disabled(busy)
                .help(tf("instance.updateTo", &[("version", &update.latest_number)]))
                .on_press(move |app| apply_updates(app, vec![pending.clone()], Some(target.clone())))
                .build(theme),
        );
    }
    let enabled = item.enabled;
    let target = version_id.clone();
    entry = entry.child(
        Btn::new(if enabled { t_("instance.enabled") } else { t_("instance.disabled") })
            .ghost()
            .sm()
            .key(format!("toggle-{version_id}"))
            .help(if enabled { t_("instance.disable") } else { t_("instance.enable") })
            .on_press(move |app| toggle(app, target.clone(), enabled))
            .build(theme),
    );
    let target = version_id.clone();
    entry = entry.child(icon_btn(theme, format!("remove-{version_id}"), &icons::TRASH, 15.0, 7.0, t_("common.remove"), false, move |app| remove(app, target.clone())));
    if enabled { entry } else { entry.opacity(0.55) }
}

fn restore_panel(theme: &Theme, content: &State, date_format: &str) -> El {
    let mut panel = col()
        .key("restore-panel")
        .mb(16.0)
        .padding(12.0)
        .gap(6.0)
        .fill(theme.bg_card)
        .border(2.0, theme.border)
        .child(
            row().justify_between().center_y().gap(10.0).mb(4.0)
                .child(ui::muted(theme, t_("content.restorePointsHint"), 14.0).wrap().grow(1.0).shrink())
                .child(Btn::new(t_("content.createRestorePoint")).ghost().sm().icon(&icons::PLUS).key("create-point").on_press(create_point).build(theme)),
        );
    if content.restore_points.is_empty() {
        return panel.child(row().justify_center().my(6.0).child(ui::muted(theme, t_("content.noRestorePoints"), 12.5)));
    }
    for point in &content.restore_points {
        let busy = content.restoring_id.as_deref() == Some(point.id.as_str());
        let details = format!(
            "{} · {}{}",
            ui::fmt::date_str(&point.created, date_format),
            point.content_count,
            if point.auto { format!(" · {}", t_("content.autoPoint")) } else { String::new() }
        );
        let (restore_id, delete_id) = (point.id.clone(), point.id.clone());
        panel = panel.child(
            row()
                .key(format!("point-{}", point.id))
                .gap(10.0)
                .center_y()
                .px(6.0)
                .py(8.0)
                .border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle)
                .child(col().grow(1.0).shrink().gap(2.0)
                    .child(text(point.label.clone()).font(theme.sans(13.0, 400.0)).color(theme.text).truncate())
                    .child(ui::muted(theme, details, 11.0)))
                .child(
                    Btn::new(if busy { t_("content.restoring") } else { t_("content.restore") })
                        .ghost()
                        .sm()
                        .key(format!("restore-{}", point.id))
                        .disabled(busy)
                        .on_press(move |app| restore(app, restore_id.clone()))
                        .build(theme),
                )
                .child(icon_btn(theme, format!("delete-point-{}", point.id), &icons::TRASH, 14.0, 7.0, t_("common.remove"), false, move |app| delete_point(app, delete_id.clone()))),
        );
    }
    panel
}
