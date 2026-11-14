//! `LauncherImport.svelte`: bring instances over from Prism, MultiMC, CurseForge, Modrinth…

use std::collections::BTreeMap;

use rune::{Color, Cursor, El, col, el, row, text};

use super::widgets::boxed;
use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::import_launcher::{DetectedLauncher, ExternalInstance};
use crate::instance::ModLoader;
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Importing,
    Done,
    Error,
}

#[derive(Default)]
pub struct State {
    scanned: bool,
    scanning: bool,
    launchers: Vec<DetectedLauncher>,
    /// Index into `launchers`.
    selected: Option<usize>,
    instances: Vec<ExternalInstance>,
    loading: bool,
    delete_source: bool,
    status: BTreeMap<usize, Status>,
}

fn close(app: &mut App) {
    app.ui.launcher_import_open = false;
    app.modals.import = State::default();
}

fn scan(app: &mut App) {
    let importer = &mut app.modals.import;
    if importer.scanned {
        return;
    }
    importer.scanned = true;
    importer.scanning = true;
    importer.launchers.clear();
    app.fx.blocking(|_| crate::commands::detect_launchers(), |app, list| {
        app.modals.import.scanning = false;
        app.modals.import.launchers = list;
    });
}

fn open_launcher(app: &mut App, i: usize) {
    let importer = &mut app.modals.import;
    let Some(id) = importer.launchers.get(i).map(|launcher| launcher.id.clone()) else { return };
    importer.selected = Some(i);
    importer.instances.clear();
    importer.status.clear();
    importer.delete_source = false;
    importer.loading = true;
    app.fx.blocking(move |_| crate::commands::list_external_instances(id), move |app, list| {
        let importer = &mut app.modals.import;
        if importer.selected == Some(i) {
            importer.loading = false;
            importer.instances = list;
        }
    });
}

fn back(app: &mut App) {
    let importer = &mut app.modals.import;
    importer.selected = None;
    importer.instances.clear();
    importer.status.clear();
}

fn import_one(app: &mut App, i: usize) {
    let importer = &mut app.modals.import;
    if matches!(importer.status.get(&i), Some(Status::Importing | Status::Done)) {
        return;
    }
    let Some(inst) = importer.instances.get(i).cloned() else { return };
    importer.status.insert(i, Status::Importing);
    let wipe = importer.delete_source && inst.source_dir.is_some();
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::import_external_instance(host, inst, wipe).await }, move |app, result| match result {
        Ok(created) => {
            app.modals.import.status.insert(i, Status::Done);
            app.toasts.success(tf("launcherImport.imported", &[("name", &created.name)]));
            app.refresh_instances();
        }
        Err(e) => {
            app.modals.import.status.insert(i, Status::Error);
            app.toasts.error(e.to_string());
        }
    });
}

fn loader_label(loader: &str) -> &'static str {
    match ModLoader::from_id(loader) {
        ModLoader::Vanilla => "",
        loader => loader.label(),
    }
}

/// `BRAND`: a launcher's colour and initials when it has no icon.
fn brand(theme: &Theme, id: &str) -> (Color, &'static str) {
    let (hex, initials) = match id {
        "prism" => ("#4f8cc9", "Pr"),
        "multimc" => ("#5fa85f", "MM"),
        "polymc" => ("#8b6cf0", "Po"),
        "curseforge" => ("#f16436", "CF"),
        "modrinth" => ("#1bd96a", "Mo"),
        "vanilla" => ("#7aa64f", "MC"),
        _ => return (theme.accent, "?"),
    };
    (rune::color::hex(hex).unwrap_or(theme.accent), initials)
}

fn skeletons(theme: &Theme) -> El {
    col().key("skeletons").gap(8.0).children((0..3).map(|i| ui::skeleton(theme, None, 46.0).key(format!("skel{i}"))))
}

pub fn view(app: &App, layer: &mut Vec<El>) {
    let importer = &app.modals.import;
    if !importer.scanned {
        app.fx.post(scan);
    }
    let theme = app.theme();
    let body = match importer.selected.and_then(|i| importer.launchers.get(i)) {
        Some(launcher) => instances(app, &theme, importer, launcher),
        None if importer.scanning || !importer.scanned => skeletons(&theme),
        None if importer.launchers.is_empty() => col()
            .key("empty")
            .items_center()
            .gap(10.0)
            .py(28.0)
            .child(ui::colored_icon(&icons::SEARCH, 22.0, theme.text_muted))
            .child(text(tr("launcherImport.noLaunchers")).font(theme.sans(13.0, 400.0)).color(theme.text_muted).wrap().align(rune::Align::Center)),
        None => launchers(app, &theme, importer),
    };
    let footer = Btn::new(tr("common.done")).ghost().key("done").on_press(close).build(&theme);
    layer.push(ui::modal(&theme, "launcher-import", tr("launcherImport.title"), 480.0, col().w(440.0).child(body), Some(footer), close));
}

fn launchers(app: &App, theme: &Theme, importer: &State) -> El {
    let mut rows = col().key("launchers").gap(8.0);
    for (i, launcher) in importer.launchers.iter().enumerate() {
        let mark = match launcher.icon.as_deref() {
            Some(src) => ui::image(app, src, 34.0, 34.0),
            None => {
                let (color, label) = brand(theme, &launcher.id);
                el().size(34.0, 34.0)
                    .center()
                    .fill(theme.bg_raised)
                    .border(2.0, color)
                    .child(text(label).font(rune::tok::TextStyle { weight: 700.0, tracking: 0.22, ..theme.pixel(11.0) }).color(color))
            }
        };
        let face = row()
            .key(format!("launcher-{}", launcher.id))
            .role("button")
            .label(launcher.name.clone())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .gap(12.0)
            .center_y()
            .px(16.0)
            .py(14.0)
            .fill(theme.bg_input)
            .on_press(move |app: &mut App| open_launcher(app, i))
            .child(mark)
            .child(
                col()
                    .gap(2.0)
                    .grow(1.0)
                    .min_w(0.0)
                    .child(text(launcher.name.clone()).font(theme.sans(13.0, 600.0)).color(theme.text).truncate())
                    .child(
                        text(tf("launcherImport.instanceCount", &[("count", &launcher.instance_count.to_string())]))
                            .font(theme.sans(11.0, 400.0))
                            .color(theme.text_muted),
                    ),
            )
            .child(ui::colored_icon(&icons::CHEVRON_RIGHT, 16.0, theme.text_secondary));
        rows = rows.child(boxed(theme, face, false));
    }
    rows
}

fn instances(app: &App, theme: &Theme, importer: &State, launcher: &DetectedLauncher) -> El {
    let _ = app;
    let crumbs = row()
        .key("crumbs")
        .gap(8.0)
        .center_y()
        .mb(12.0)
        .child(
            row()
                .key("all-launchers")
                .role("link")
                .gap(3.0)
                .center_y()
                .cursor(Cursor::Pointer)
                .hover_group()
                .on_press(back)
                .child(ui::colored_icon(&icons::CHEVRON_LEFT, 14.0, theme.text_secondary).group_color(theme.accent))
                .child(text(tr("launcherImport.allLaunchers")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).group_color(theme.accent)),
        )
        .child(text("/").font(theme.sans(13.0, 400.0)).color(theme.text_muted))
        .child(text(launcher.name.clone()).font(theme.sans(13.0, 700.0)).color(theme.text));
    let option = if launcher.can_delete {
        row()
            .key("delete-source")
            .gap(8.0)
            .center_y()
            .mb(12.0)
            .role("checkbox")
            .checked(importer.delete_source)
            .cursor(Cursor::Pointer)
            .on_press(|app: &mut App| app.modals.import.delete_source = !app.modals.import.delete_source)
            .child(ui::checkbox(theme, importer.delete_source).a11y_hidden())
            .child(text(tr("launcherImport.deleteAfter")).font(theme.sans(12.0, 400.0)).color(theme.text_secondary).wrap().shrink())
    } else {
        row()
            .key("shared-hint")
            .gap(6.0)
            .center_y()
            .mb(12.0)
            .child(ui::colored_icon(&icons::GLOBE, 12.0, theme.text_muted))
            .child(text(tr("launcherImport.sharedDirHint")).font(theme.sans(11.0, 400.0)).color(theme.text_muted).wrap().shrink())
    };
    let list = if importer.loading {
        skeletons(theme)
    } else if importer.instances.is_empty() {
        text(tr("launcherImport.noInstances")).font(theme.sans(13.0, 400.0)).color(theme.text_muted).key("none")
    } else {
        let mut rows = col().key("instances").gap(8.0);
        for (i, inst) in importer.instances.iter().enumerate() {
            let mut sub = row().gap(6.0).center_y().child(text(inst.mc_version.clone()).font(theme.sans(11.0, 400.0)).color(theme.text_muted));
            let loader_name = loader_label(&inst.loader);
            if !loader_name.is_empty() {
                sub = sub.child(
                    el().px(6.0)
                        .py(1.0)
                        .fill(theme.bg_raised)
                        .border(1.0, theme.border_subtle)
                        .child(text(loader_name).font(theme.sans(10.0, 400.0)).color(theme.text_secondary)),
                );
            }
            let action = match importer.status.get(&i) {
                Some(Status::Done) => row()
                    .gap(5.0)
                    .center_y()
                    .child(ui::colored_icon(&icons::CHECK, 14.0, theme.accent))
                    .child(text(tr("launcherImport.done")).font(theme.sans(12.0, 400.0)).color(theme.accent)),
                status => Btn::new(match status {
                    Some(Status::Importing) => tr("common.importing"),
                    Some(Status::Error) => tr("launcherImport.retry"),
                    _ => tr("common.import"),
                })
                .primary()
                .sm()
                .key(format!("import-{i}"))
                .disabled(status == Some(&Status::Importing))
                .on_press(move |app| import_one(app, i))
                .build(theme),
            };
            rows = rows.child(
                row()
                    .key(format!("ext-{i}"))
                    .gap(12.0)
                    .center_y()
                    .px(12.0)
                    .py(10.0)
                    .fill(theme.bg_input)
                    .border(2.0, theme.border)
                    .child(
                        col()
                            .gap(2.0)
                            .grow(1.0)
                            .min_w(0.0)
                            .child(text(inst.name.clone()).font(theme.sans(13.0, 600.0)).color(theme.text).truncate())
                            .child(sub),
                    )
                    .child(action),
            );
        }
        rows
    };
    col().key("selected").child(crumbs).child(option).child(list)
}
