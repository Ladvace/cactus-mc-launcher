//! The Settings tab: instance facts, InstanceJavaSettings, storage and the danger zone.

use rune::{Cx, El, FileDialogOptions, TextEdit, Tx, col, row, text};

use super::{boxed, card_block, reveal_or_toast, t_, text_field};
use crate::app::{App, Ignore};
use crate::core::i18n::tf;
use crate::icons;
use crate::instance::{Instance, InstanceKind, UpdateInstance};
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    pub game_folder: String,
    pub moving: bool,
    pub max_mem: TextEdit,
    pub min_mem: TextEdit,
    pub jvm: TextEdit,
    pub java_path: TextEdit,
    pub width: TextEdit,
    pub height: TextEdit,
    pub saving: bool,
}

fn number_edit(value: Option<u32>) -> TextEdit {
    TextEdit::new(value.map(|n| n.to_string()).unwrap_or_default())
}

/// The Java overrides as the instance has them (InstanceJavaSettings' `lastId` effect).
pub fn init(app: &mut App, inst: &Instance) {
    let legacy = if inst.kind == InstanceKind::Server { inst.server_memory_mb } else { None };
    let settings = &mut app.instance_page.settings;
    settings.max_mem = number_edit(inst.max_memory_mb.or(legacy));
    settings.min_mem = number_edit(inst.min_memory_mb);
    settings.jvm = TextEdit::new(inst.jvm_args.clone().unwrap_or_default());
    settings.java_path = TextEdit::new(inst.java_path.clone().unwrap_or_default());
    settings.width = number_edit(inst.game_width);
    settings.height = number_edit(inst.game_height);
}

pub fn load_folder(app: &mut App) {
    let id = app.instance_page.id.clone();
    app.fx.blocking(move |host| (crate::commands::instance_folder(host.clone(), id.clone()), id), |app, (result, id)| {
        if app.instance_page.id == id {
            if let Ok(path) = result {
                app.instance_page.settings.game_folder = path;
            }
        }
    });
}

pub fn open_game_folder(app: &mut App) {
    let id = app.instance_page.id.clone();
    match crate::commands::instance_folder(app.fx.host.clone(), id) {
        Ok(path) => reveal_or_toast(app, &path),
        Err(e) => app.toasts.error(e.to_string()).ignore(),
    }
}

fn move_folder(app: &mut App, path: Option<String>) {
    let id = app.instance_page.id.clone();
    if app.instance_page.settings.moving || app.is_running(&id) {
        return;
    }
    app.instance_page.settings.moving = true;
    let custom = path.is_some();
    app.fx.blocking(
        move |host| crate::commands::set_instance_game_dir(host.clone(), host.state(), host.state(), host.state(), id, path),
        move |app, result| {
            app.instance_page.settings.moving = false;
            match result {
                Ok(_) => {
                    app.refresh_instances();
                    load_folder(app);
                    app.toasts.success(if custom { t_("instance.filesMoved") } else { t_("instance.movedToDefault") });
                }
                Err(e) => app.toasts.error(e.to_string()).ignore(),
            }
        },
    );
}

fn change_folder(app: &mut App, tx: &mut Tx) {
    if app.instance_page.settings.moving || app.is_running(&app.instance_page.id) {
        return;
    }
    let options = FileDialogOptions { title: t_("instance.pickFolderTitle"), ..Default::default() };
    tx.open_folder(options, |app: &mut App, result, _| {
        if let Ok(Some(path)) = result {
            move_folder(app, Some(path.to_string_lossy().into_owned()));
        }
    });
}

/// `parseInt`, with anything not positive clearing the override (0).
fn parse_override(edit: &TextEdit) -> u32 {
    let digits: String = edit.text.trim().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

fn save_java(app: &mut App, is_server: bool) {
    let settings = &app.instance_page.settings;
    let update = UpdateInstance {
        max_memory_mb: Some(parse_override(&settings.max_mem)),
        min_memory_mb: Some(parse_override(&settings.min_mem)),
        jvm_args: Some(settings.jvm.text.trim().to_string()),
        java_path: Some(settings.java_path.text.trim().to_string()),
        game_width: (!is_server).then(|| parse_override(&settings.width)),
        game_height: (!is_server).then(|| parse_override(&settings.height)),
        server_memory_mb: Some(0),
        ..Default::default()
    };
    app.instance_page.settings.saving = true;
    let id = app.instance_page.id.clone();
    app.update_instance(&id, update, |app, result| {
        app.instance_page.settings.saving = false;
        match result {
            Ok(_) => app.toasts.success(t_("instance.settingsSaved")).ignore(),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn clear_java(app: &mut App) {
    let settings = &mut app.instance_page.settings;
    for edit in [&mut settings.max_mem, &mut settings.min_mem, &mut settings.jvm, &mut settings.java_path, &mut settings.width, &mut settings.height] {
        *edit = TextEdit::default();
    }
}

/// java.ts `requiredJavaMajor`: ≤1.16 → 8, 1.17–1.20.4 → 17, 1.20.5+ → 21, else 21.
fn required_java_major(mc_version: &str) -> u32 {
    let Some(rest) = mc_version.strip_prefix("1.") else { return 21 };
    let mut parts = rest.split('.');
    let leading_digits = |part: Option<&str>| part.map(|part| part.chars().take_while(char::is_ascii_digit).collect::<String>());
    let Some(minor) = leading_digits(parts.next()).and_then(|digits| digits.parse::<u32>().ok()) else { return 21 };
    let patch = leading_digits(parts.next()).and_then(|digits| digits.parse::<u32>().ok()).unwrap_or(0);
    match minor {
        0..=16 => 8,
        17..=19 => 17,
        20 if patch >= 5 => 21,
        20 => 17,
        _ => 21,
    }
}

/// `.row`: label left, value right, a rule above all but the first.
fn info_row(theme: &Theme, first: bool, label: El, value: El) -> El {
    let info = row().w_full().justify_between().center_y().py(10.0).gap(12.0).child(label).child(value);
    if first { info } else { info.border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle) }
}

fn label(theme: &Theme, content: impl Into<String>) -> El {
    ui::secondary(theme, content, 13.0)
}

fn strong(theme: &Theme, content: impl Into<String>) -> El {
    text(content.into()).font(theme.sans(13.0, 700.0)).color(theme.text_secondary)
}

pub fn view(app: &App, theme: &Theme, inst: &Instance, cx: &mut Cx<App>) -> El {
    let _ = cx;
    let settings = &app.instance_page.settings;
    let running = app.is_running(&inst.id);

    let facts = card_block(theme)
        .key("card-instance")
        .child(ui::h3(theme, t_("instance.instanceHeading")).mb(14.0))
        .child(info_row(theme, true, label(theme, t_("instance.name")),
            Btn::new(t_("instance.rename")).ghost().sm().icon(&icons::EDIT).key("settings-rename").on_press(super::open_rename).build(theme)))
        .child(info_row(theme, false, label(theme, t_("instance.minecraftVersion")), strong(theme, inst.mc_version.clone())))
        .child(info_row(theme, false, label(theme, t_("instance.modLoader")), strong(theme, inst.loader.label())))
        .child(info_row(theme, false, label(theme, t_("instance.created")), strong(theme, ui::fmt::date(inst.created, &app.settings.date_format))));

    let locked = settings.moving || running;
    let mut folder_actions = row().gap(8.0).child(Btn::new(t_("instance.open")).ghost().sm().key("folder-open").on_press(open_game_folder).build(theme));
    if inst.game_dir.is_some() {
        folder_actions = folder_actions.child(Btn::new(t_("instance.reset")).ghost().sm().key("folder-reset").disabled(locked).on_press(|app| move_folder(app, None)).build(theme));
    }
    folder_actions = folder_actions.child(
        Btn::new(if settings.moving { t_("instance.moving") } else { t_("instance.change") }).ghost().sm().key("folder-change").disabled(locked).on_press_tx(change_folder).build(theme),
    );
    let mut storage = card_block(theme)
        .key("card-storage")
        .child(ui::h3(theme, t_("instance.storage")).mb(14.0))
        .child(
            col().w_full().gap(6.0).py(10.0)
                .child(row().gap(4.0).items_end().child(label(theme, t_("instance.gameFolder"))).child(ui::muted(theme, t_("instance.gameFolderHint"), 11.0)))
                .child(text(if settings.game_folder.is_empty() { "…".to_string() } else { settings.game_folder.clone() }).font(theme.pixel(11.0)).color(theme.accent).wrap_anywhere()),
        )
        .child(info_row(theme, false, label(theme, if inst.game_dir.is_some() { t_("instance.customLocation") } else { t_("instance.defaultLocation") }), folder_actions));
    if running {
        storage = storage.child(ui::muted(theme, t_("instance.stopToMove"), 12.0).mt(8.0));
    }

    let danger = card_block(theme)
        .key("card-danger")
        .border(2.0, theme.danger.alpha(0.25))
        .child(ui::h3(theme, t_("instance.dangerZone")).mb(14.0))
        .child(info_row(theme, true, label(theme, t_("instance.deleteDescription")),
            Btn::new(t_("instance.delete")).danger().sm().icon(&icons::TRASH).key("delete").on_press(|app| app.instance_page.delete_open = true).build(theme)));

    col().w_full().gap(20.0).child(facts).child(java(app, theme, inst)).child(storage).child(danger)
}

/// `InstanceJavaSettings.svelte`.
fn java(app: &App, theme: &Theme, inst: &Instance) -> El {
    let is_server = inst.kind == InstanceKind::Server;
    let global_settings = &app.settings;
    let settings = &app.instance_page.settings;
    let major = required_java_major(&inst.mc_version);
    let global = |value: String| tf("instance.globalValue", &[("value", &value)]);
    let java_placeholder = match global_settings.java_paths.get(&major).map(|path| path.trim()).filter(|path| !path.is_empty()) {
        Some(path) => tf("instance.javaPerMajor", &[("major", &major.to_string()), ("path", path)]),
        None => match global_settings.java_path.as_deref().map(str::trim).filter(|path| !path.is_empty()) {
            Some(path) => global(path.to_string()),
            None => tf("instance.managedJava", &[("major", &major.to_string())]),
        },
    };
    let jvm_placeholder = if global_settings.jvm_args.trim().is_empty() { "e.g. -XX:+UseG1GC".to_string() } else { global(global_settings.jvm_args.clone()) };

    let field = |key: &str, field_label: El, edit: &TextEdit, placeholder: String, access: fn(&mut App) -> &mut TextEdit| {
        col().gap(5.0).child(field_label).child(boxed(theme, edit, text_field(theme, edit, placeholder, access).key(key.to_string())))
    };
    let small_label = |key: &str| ui::secondary(theme, t_(key), 12.5);

    let mut grid = col()
        .gap(12.0)
        .child(row().gap(16.0).equal_columns(2)
            .child(field("max-mem", small_label("instance.maxMemory"), &settings.max_mem, global(global_settings.max_memory_mb.to_string()), |app| &mut app.instance_page.settings.max_mem))
            .child(field("min-mem", small_label("instance.minMemory"), &settings.min_mem, global(global_settings.min_memory_mb.to_string()), |app| &mut app.instance_page.settings.min_mem)))
        .child(field("jvm-args", small_label("instance.jvmArgs"), &settings.jvm, jvm_placeholder, |app| &mut app.instance_page.settings.jvm))
        .child(field(
            "java-path",
            row().gap(4.0).child(small_label("instance.javaPath")).child(ui::muted(theme, tf("instance.javaVersionHint", &[("major", &major.to_string())]), 11.0)),
            &settings.java_path,
            java_placeholder,
            |app| &mut app.instance_page.settings.java_path,
        ));
    if !is_server {
        grid = grid.child(row().gap(16.0).equal_columns(2)
            .child(field("win-width", small_label("instance.windowWidth"), &settings.width, global(global_settings.game_width.to_string()), |app| &mut app.instance_page.settings.width))
            .child(field("win-height", small_label("instance.windowHeight"), &settings.height, global(global_settings.game_height.to_string()), |app| &mut app.instance_page.settings.height)));
    }

    card_block(theme)
        .key("card-java")
        .child(row().gap(10.0).flex_wrap().center_y().mb(14.0).child(ui::h3(theme, t_("instance.javaMemory"))).child(ui::muted(theme, t_("instance.overridesHint"), 12.0)))
        .child(grid)
        .child(
            row().justify_end().gap(8.0).mt(16.0)
                .child(Btn::new(t_("instance.clearAll")).ghost().sm().key("java-clear").disabled(settings.saving).on_press(clear_java).build(theme))
                .child(Btn::new(if settings.saving { t_("instance.saving") } else { t_("common.save") }).primary().sm().key("java-save").disabled(settings.saving)
                    .on_press(move |app| save_java(app, is_server)).build(theme)),
        )
}
