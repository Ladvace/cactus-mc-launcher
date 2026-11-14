//! The instance page: `src/routes/instance/[id]/+page.svelte` with WorldsList, PlayersList,
//! ServerProperties, ServerShare, ServerAddress, InstanceJavaSettings, TuneupModal.

mod console;
mod content;
mod players;
mod properties;
mod server;
mod settings;
mod tuneup;
mod worlds;

use rune::{Color, Cursor, Cx, El, Icon, ScrollSpec, TextEdit, col, el, row, text};

use crate::app::{App, Ignore, LaunchRuntime, Route, to_percent};
use crate::core::i18n::{t, tf};
use crate::icons;
use crate::instance::{Instance, InstanceKind, UpdateInstance};
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    /// The instance the fields below belong to.
    pub id: String,
    pub tab: &'static str,
    pub content: content::State,
    pub settings: settings::State,
    pub command: TextEdit,
    pub worlds: worlds::State,
    pub players: players::State,
    pub props: properties::State,
    pub server: server::State,
    pub tuneup: tuneup::State,
    pub rename_open: bool,
    pub rename: TextEdit,
    pub delete_open: bool,
    pub busy: bool,
}

const CLIENT_TABS: [&str; 5] = ["Content", "Worlds", "Screenshots", "Logs", "Settings"];
const SERVER_TABS: [&str; 6] = ["Content", "Worlds", "Console", "Players", "Properties", "Settings"];

fn tabs(inst: &Instance) -> &'static [&'static str] {
    if inst.kind == InstanceKind::Server { &SERVER_TABS } else { &CLIENT_TABS }
}

fn tab_label(tab: &str) -> String {
    match tab {
        "Content" => t("instance.tabContent"),
        "Worlds" => t("instance.tabWorlds"),
        "Console" => t("instance.tabConsole"),
        "Players" => t("instance.tabPlayers"),
        "Properties" => t("instance.tabProperties"),
        "Settings" => t("instance.tabSettings"),
        "Screenshots" => t("instance.tabScreenshots"),
        "Logs" => t("instance.tabLogs"),
        _ => tab.to_string(),
    }
}

/// The route was entered (mount): load what the page shows.
pub fn on_enter(app: &mut App) {
    let Route::Instance(id) = app.route.clone() else { return };
    if app.instance_page.id != id {
        app.instance_page = State { id: id.clone(), ..State::default() };
    }
    app.instance_page.tab = "Content";
    app.instance_page.rename_open = false;
    app.instance_page.delete_open = false;
    app.instance_page.tuneup.open = false;
    if let Some(inst) = app.instance(&id).cloned() {
        settings::init(app, &inst);
        if inst.kind == InstanceKind::Server {
            server::load(app);
        }
    }
    settings::load_folder(app);
    enter_tab(app);
}

/// What switching to the current tab loads, as each tab's component does on mount.
fn enter_tab(app: &mut App) {
    match app.instance_page.tab {
        "Content" => {
            content::load(app);
            content::refresh_restore_points(app);
        }
        "Worlds" => worlds::load(app),
        "Players" => players::load(app),
        "Properties" => properties::load(app),
        _ => {}
    }
}

fn set_tab(app: &mut App, tab: &'static str) {
    if app.instance_page.tab != tab {
        app.instance_page.tab = tab;
        enter_tab(app);
    }
}

/// A backend event (`launch-status`, `modpack-progress`, …), after the core stores took it.
pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

/// The page. Popovers, menus and modals it opens go on `layer` (drawn above everything).
pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let page = &app.instance_page;
    let Some(inst) = app.instance(&page.id) else { return missing(app, &theme) };
    let is_server = inst.kind == InstanceKind::Server;
    let tab = if tabs(inst).contains(&page.tab) { page.tab } else { "Content" };

    let mut tab_row = row().gap(2.0).w_full().max_w(1040.0);
    for &name in tabs(inst) {
        let active = name == tab;
        tab_row = tab_row.child(
            el().key(format!("tab-{name}"))
                .px(14.0)
                .py(12.0)
                .border_sides([0.0, 0.0, 3.0, 0.0], if active { theme.accent } else { Color::TRANSPARENT })
                .role("tab")
                .label(tab_label(name))
                .focusable(true)
                .cursor(Cursor::Pointer)
                .on_press(move |app: &mut App| set_tab(app, name))
                .child(
                    text(tab_label(name))
                        .font(rune::tok::TextStyle { weight: 500.0, ..theme.pixel(14.0) })
                        .color(if active { theme.accent } else { theme.text_secondary }),
                ),
        );
    }
    let tab_bar = col().key("tabs").w_full().px(32.0).items_center().border_sides([0.0, 0.0, 1.0, 0.0], theme.border_subtle).child(tab_row);

    let body = match tab {
        "Settings" => settings::view(app, &theme, inst, cx),
        "Content" => content::view(app, &theme, inst),
        "Logs" | "Console" => console::view(app, &theme, inst, cx),
        "Properties" => properties::view(app, &theme, cx, layer),
        "Players" => players::view(app, &theme),
        "Worlds" => worlds::view(app, &theme),
        _ => col()
            .w_full()
            .items_center()
            .gap(12.0)
            .padding(56.0)
            .child(ui::colored_icon(&icons::PACKAGE, 34.0, theme.text_muted))
            .child(ui::muted(&theme, tf("instance.tabPlaceholder", &[("tab", &tab_label(tab))]), 14.0)),
    };
    let tab_body = col().key("tab-body").w_full().items_center().px(32.0).py(28.0).child(col().key(format!("body-{tab}")).w_full().max_w(1040.0).child(body));

    if page.rename_open {
        layer.push(rename_modal(app, &theme));
    }
    if page.delete_open {
        layer.push(delete_modal(app, &theme, inst));
    }
    if page.tuneup.open {
        layer.push(tuneup::view(app, &theme));
    }

    col()
        .key("instance-page")
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(banner(app, &theme, inst, is_server))
        .child(tab_bar)
        .child(tab_body)
}

fn missing(app: &App, theme: &Theme) -> El {
    if app.instances_loaded {
        return col()
            .w_full()
            .padding(80.0)
            .items_center()
            .gap(14.0)
            .child(ui::secondary(theme, t_("instance.noLongerExists"), 14.0).color(theme.text))
            .child(Btn::new(t_("instance.backToHome")).ghost().key("back-home").on_press(|app| app.go(Route::Home)).build(theme));
    }
    col()
        .w_full()
        .padding(80.0)
        .items_center()
        .child(
            col().w_full().max_w(900.0).gap(16.0).mt(24.0)
                .child(row().gap(16.0).center_y()
                    .child(ui::skeleton(theme, Some(64.0), 64.0))
                    .child(col().gap(8.0).child(ui::skeleton(theme, Some(180.0), 20.0)).child(ui::skeleton(theme, Some(120.0), 12.0))))
                .child(ui::skeleton(theme, None, 38.0))
                .child(ui::skeleton(theme, None, 140.0)),
        )
}

fn t_(key: &str) -> String {
    t(key)
}

fn fmt_playtime(sec: u64) -> String {
    if sec < 60 {
        return t("instance.playtimeLessThanMin");
    }
    let (hours, minutes) = (sec / 3600, (sec % 3600) / 60);
    if hours > 0 {
        tf("instance.playtimeHoursMinutes", &[("hours", &hours.to_string()), ("minutes", &minutes.to_string())])
    } else {
        tf("instance.playtimeMinutes", &[("minutes", &minutes.to_string())])
    }
}

fn badge(theme: &Theme, label: impl Into<String>, ink: Color, face: Color, edge: Color) -> El {
    el().px(10.0).py(4.0).fill(face).border(2.0, edge).child(
        text(label.into()).font(rune::tok::TextStyle { weight: 500.0, ..theme.pixel(12.0) }).color(ink),
    )
}

fn banner(app: &App, theme: &Theme, inst: &Instance, is_server: bool) -> El {
    let id = inst.id.clone();
    let runtime = app.launch.get(&id).cloned().unwrap_or_default();
    let busy = runtime.is_busy();
    let running = runtime.is_running();

    let mut badges = row().gap(6.0).flex_wrap().mb(10.0);
    if is_server {
        badges = badges.child(badge(theme, t_("instance.serverBadge"), theme.bg_app, theme.accent, theme.accent));
    }
    badges = badges
        .child(badge(theme, inst.loader.label(), theme.text_secondary, theme.bg_card, theme.border))
        .child(badge(theme, inst.mc_version.clone(), theme.text_secondary, theme.bg_card, theme.border));
    if let Some(loader_version) = inst.loader_version.as_deref().filter(|version| !version.is_empty()) {
        badges = badges.child(badge(theme, loader_version, theme.text_muted, theme.bg_card, theme.border));
    }
    let last_played = inst.last_played.map_or_else(|| t_("instance.never"), |played_at| ui::fmt::date(played_at, &app.settings.date_format));
    let stats = row()
        .gap(16.0)
        .child(row().gap(5.0).center_y()
            .child(ui::colored_icon(&icons::CLOCK, 13.0, theme.text_muted))
            .child(ui::muted(theme, tf("instance.played", &[("time", &fmt_playtime(inst.total_playtime_seconds))]), 12.5)))
        .child(ui::muted(theme, tf("instance.lastPlayed", &[("date", &last_played)]), 12.5));
    let titles = col()
        .grow(1.0)
        .shrink()
        .child(ui::heading(theme, inst.name.clone(), 26.0).truncate().mb(8.0))
        .child(badges)
        .child(stats);

    let mut actions = row().gap(8.0).center_y();
    actions = if running {
        let id = id.clone();
        actions.child(Btn::new(t_("instance.stop")).danger().big().icon(&icons::STOP).key("stop").on_press(move |app| app.stop_instance(&id)).build(theme))
    } else {
        let label = match (busy, is_server) {
            (true, true) => t_("instance.starting"),
            (true, false) => t_("instance.preparing"),
            (false, true) => t_("instance.startServer"),
            (false, false) => t_("instance.play"),
        };
        let id = id.clone();
        actions.child(Btn::new(label).primary().big().icon(&icons::PLAY).key("play").disabled(busy).on_press(move |app| app.launch_instance(&id)).build(theme))
    };
    if !is_server {
        actions = actions.child(
            Btn::icon_only(&icons::SPARKLES).ghost().key("tuneup").help(t_("instance.tuneupTitle")).on_press(tuneup::open).build(theme).label(t_("instance.tuneupLabel")),
        );
    }
    actions = actions
        .child(Btn::icon_only(&icons::FOLDER).ghost().key("open-folder").help(t_("instanceMenu.openFolder")).on_press(settings::open_game_folder).build(theme).label(t_("instanceMenu.openFolder")))
        .child(Btn::icon_only(&icons::EDIT).ghost().key("rename").help(t_("instance.rename")).on_press(open_rename).build(theme).label(t_("instance.rename")));

    let back = el()
        .key("back")
        .py(4.0)
        .mb(18.0)
        .self_start()
        .role("button")
        .label(t_("common.back"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.go(Route::Home))
        .child(ui::secondary(theme, t_("instance.home"), 13.0));

    let mut column = col()
        .w_full()
        .max_w(1040.0)
        .child(back)
        .child(row().key("header").gap(20.0).center_y().child(ui::instance_icon(app, inst, 96.0)).child(titles).child(actions));

    if busy {
        column = column.child(col().key("progress").mt(18.0).max_w(520.0).w_full().child(ui::progress(theme, progress_label(&runtime), to_percent(runtime.current, runtime.total))));
    } else if runtime.state == "error" {
        column = column.child(launch_error(theme, &runtime));
    }
    if is_server {
        column = column.child(server::address(app, theme)).child(server::share(app, theme, inst));
    }

    col()
        .key("banner")
        .w_full()
        .items_center()
        .pt(24.0)
        .px(32.0)
        .pb(20.0)
        .gradient(theme.bg_raised, theme.bg_app)
        .border_sides([0.0, 0.0, 1.0, 0.0], theme.border_subtle)
        .child(column)
}

fn progress_label(runtime: &LaunchRuntime) -> String {
    if let Some(message) = &runtime.message {
        return message.clone();
    }
    match runtime.stage.as_str() {
        "libraries" => t_("instance.stageLibraries"),
        "assets" => t_("instance.stageAssets"),
        "java" => t_("instance.stageJava"),
        _ => t_("instance.working"),
    }
}

/// Turn a raw crash into an actionable hint by scanning the recent log tail.
fn crash_hint(runtime: &LaunchRuntime) -> String {
    let tail: Vec<&str> = runtime.logs.iter().rev().take(250).rev().map(String::as_str).collect();
    let haystack = format!("{} {}", tail.join("\n"), runtime.message.as_deref().unwrap_or("")).to_lowercase();
    let any = |needles: &[&str]| needles.iter().any(|needle| haystack.contains(needle));
    let key = if any(&["outofmemoryerror", "java heap space", "gc overhead limit"]) {
        "instance.crashOom"
    } else if any(&[
        "could not reserve enough space for object heap",
        "could not create the java virtual machine",
        "error occurred during initialization of vm",
        "invalid maximum heap size",
        "too small maximum heap",
    ]) {
        "instance.crashJvm"
    } else if any(&["unsupportedclassversionerror", "class file version", "compiled by a more recent version of the java"]) {
        "instance.crashJava"
    } else if (haystack.contains("requires") && haystack.contains("which is missing"))
        || any(&["missing or unsupported mandatory dependencies", "mixin apply", "incompatible mod"])
    {
        "instance.crashMods"
    } else if any(&["glfw", "failed to create window", "could not create window", "could not create context", "pixel format", "no opengl"]) {
        "instance.crashGraphics"
    } else {
        "instance.crashGeneric"
    };
    t(key)
}

fn launch_error(theme: &Theme, runtime: &LaunchRuntime) -> El {
    let mut card = col()
        .key("launch-error")
        .mt(16.0)
        .max_w(640.0)
        .w_full()
        .px(14.0)
        .py(10.0)
        .gap(5.0)
        .fill(theme.danger.alpha(0.1))
        .border(1.0, theme.danger.alpha(0.3))
        .child(text(t_("instance.launchFailed")).font(theme.sans(13.0, 700.0)).color(theme.danger))
        .child(row().gap(6.0).center_y()
            .child(ui::colored_icon(&icons::SPARKLES, 13.0, theme.text))
            .child(text(crash_hint(runtime)).font(theme.sans(13.0, 500.0)).color(theme.text).wrap().grow(1.0).shrink()));
    if let Some(message) = &runtime.message {
        card = card.child(col().max_h(72.0).scroll(ScrollSpec { stick_bottom: false }).child(text(message.clone()).font(theme.mono(12.0)).color(theme.text_muted).wrap_anywhere()));
    }
    card
}

fn open_rename(app: &mut App) {
    let Some(name) = app.instance(&app.instance_page.id).map(|inst| inst.name.clone()) else { return };
    app.instance_page.rename = TextEdit::new(name);
    app.instance_page.rename_open = true;
}

fn confirm_rename(app: &mut App) {
    let name = app.instance_page.rename.text.trim().to_string();
    if name.is_empty() || app.instance_page.busy {
        return;
    }
    app.instance_page.busy = true;
    let id = app.instance_page.id.clone();
    app.update_instance(&id, UpdateInstance { name: Some(name), ..Default::default() }, |app, result| {
        app.instance_page.busy = false;
        match result {
            Ok(_) => app.instance_page.rename_open = false,
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn rename_modal(app: &App, theme: &Theme) -> El {
    let page = &app.instance_page;
    let field = text_field(theme, &page.rename, "", |app| &mut app.instance_page.rename).on_submit(|app: &mut App, _| confirm_rename(app));
    let body = col().child(ui::field_label(theme, t_("instance.name"))).child(boxed(theme, &page.rename, field.key("rename-input")));
    let footer = ui::buttons([
        Btn::new(t_("common.cancel")).ghost().key("rename-cancel").on_press(|app| app.instance_page.rename_open = false).build(theme),
        Btn::new(t_("common.save")).primary().key("rename-save").disabled(page.busy || page.rename.text.trim().is_empty()).on_press(confirm_rename).build(theme),
    ]);
    ui::modal(theme, "rename-modal", t_("instance.renameInstance"), 420.0, body, Some(footer), |app| app.instance_page.rename_open = false)
}

fn delete_modal(app: &App, theme: &Theme, inst: &Instance) -> El {
    let page = &app.instance_page;
    let (prefix, name) = (t_("instance.deleteConfirmPrefix"), inst.name.clone());
    let sentence = format!("{prefix}{name}{}", t_("instance.deleteConfirmSuffix"));
    let line = rune::rich(sentence, vec![rune::Span::new(prefix.len(), prefix.len() + name.len()).weight(700.0).color(theme.text)])
    .font(rune::tok::TextStyle { line_height: 22.0, ..theme.sans(14.0, 400.0) })
    .color(theme.text_secondary)
    .wrap();
    let footer = ui::buttons([
        Btn::new(t_("common.cancel")).ghost().key("delete-cancel").on_press(|app| app.instance_page.delete_open = false).build(theme),
        Btn::new(if page.busy { t_("instance.deleting") } else { t_("instance.delete") })
            .danger()
            .key("delete-confirm")
            .disabled(page.busy)
            .on_press(confirm_delete)
            .build(theme),
    ]);
    ui::modal(theme, "delete-modal", t_("instance.deleteInstance"), 420.0, col().w_full().child(line), Some(footer), |app| app.instance_page.delete_open = false)
}

fn confirm_delete(app: &mut App) {
    app.instance_page.busy = true;
    let id = app.instance_page.id.clone();
    app.fx.blocking(move |host| crate::commands::delete_instance(host.clone(), host.state(), id), |app, result| {
        app.instance_page.busy = false;
        match result {
            Ok(()) => {
                app.instance_page.delete_open = false;
                app.refresh_instances();
                app.go(Route::Home);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

// Pieces the tabs share.

/// An `UpdateInstance` that changes nothing; set the fields to change.
/// `.card-block`.
fn card_block(theme: &Theme) -> El {
    ui::bevel(col().w_full().fill(theme.bg_card).border(2.0, theme.border).px(20.0).py(18.0), 2.0, 0.04, 0.28)
}

/// A bare single-line text field bound to `edit`; wrap it with [`boxed`] for the `.input` look.
fn text_field(
    theme: &Theme,
    edit: &TextEdit,
    placeholder: impl Into<String>,
    access: impl for<'a> Fn(&'a mut App) -> &'a mut TextEdit + 'static,
) -> El {
    rune::text_input(edit, theme.sans(13.0, 400.0))
        .single_line()
        .placeholder(placeholder.into(), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(access)
        .grow(1.0)
}

fn boxed(theme: &Theme, edit: &TextEdit, field: El) -> El {
    ui::input_frame(theme, edit.focused).child(field)
}

/// `.icon-remove` / `.del`: a bare muted icon that turns red on hover.
fn icon_btn(theme: &Theme, key: impl Into<String>, icon: &'static Icon, size: f32, pad: f32, help: impl Into<String>, disabled: bool, on_press: impl Fn(&mut App) + 'static) -> El {
    let help = help.into();
    let button = el()
        .key(key.into())
        .padding(pad)
        .center()
        .role("button")
        .label(help.clone())
        .help(help)
        .child(ui::colored_icon(icon, size, theme.text_muted));
    if disabled {
        return button.opacity(0.5).disabled(true);
    }
    button.focusable(true).cursor(Cursor::Pointer).hover(theme.danger.alpha(0.12)).on_press(move |app: &mut App| on_press(app))
}

/// The address chip of ServerAddress / ServerShare: a mono host, a tag, a trailing icon.
fn chip(theme: &Theme, key: impl Into<String>, host: &str, tag: String, icon: &'static Icon, help: String) -> El {
    row()
        .key(key.into())
        .gap(8.0)
        .center_y()
        .px(10.0)
        .py(6.0)
        .fill(theme.bg_input)
        .border(2.0, theme.border)
        .role("button")
        .label(help.clone())
        .help(help)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover(theme.bg_hover)
        .child(text(host.to_string()).font(theme.mono(13.0)).color(theme.text))
        .child(tag_label(theme, tag))
        .child(ui::colored_icon(icon, 13.0, theme.text))
}

/// `.tag` / `.loc`: tiny uppercase pixel text in a 1 pt box.
fn tag_label(theme: &Theme, label: String) -> El {
    el().px(4.0).py(1.0).border(1.0, theme.border_subtle).child(
        text(label.to_uppercase()).font(rune::tok::TextStyle { tracking: 8.0 * 0.06, ..theme.pixel(8.0) }).color(theme.text_muted),
    )
}

/// `revealPath`: show `path` in the file manager.
fn reveal(path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg("-R").arg(path).spawn().map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn().map(|_| ())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let target = std::path::Path::new(path);
        open::that_detached(if target.is_dir() { target } else { target.parent().unwrap_or(target) })
    }
}

fn reveal_or_toast(app: &mut App, path: &str) {
    if let Err(e) = reveal(path) {
        app.toasts.error(e.to_string());
    }
}

