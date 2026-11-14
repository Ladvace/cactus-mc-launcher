//! `ProjectModal.svelte`: a project's details, its latest compatible version for the chosen
//! instance, and installing it there; a modpack installs as a new instance instead.

use rune::{Color, Cursor, Cx, El, col, el, row, text};

use crate::app::{App, Route, to_percent};
use crate::content::ContentItem;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::instance::ModLoader;
use crate::modrinth::Version;
use crate::ui::{self, Btn, fmt};

use super::{Hit, picker};
use crate::sources::Source;

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub hit: Option<Hit>,
    pub selected_instance: String,
    pub versions: Vec<Version>,
    pub loading_versions: bool,
    pub version_error: Option<String>,
    pub installing: bool,
    pub done: bool,
    pub error: Option<String>,
    pub installed_item: Option<ContentItem>,
    pub mp_current: usize,
    pub mp_total: usize,
    pub mp_message: String,
    /// Drops answers for a project or instance no longer shown.
    generation: u64,
}

impl State {
    fn is_modpack(&self) -> bool {
        self.hit.as_ref().is_some_and(|hit| hit.project_type == "modpack")
    }
}

fn mod_on_vanilla(app: &App) -> bool {
    let project = &app.browse.project;
    !project.is_modpack()
        && project.hit.as_ref().is_some_and(|hit| hit.project_type == "mod")
        && app.instance(&project.selected_instance).is_some_and(|inst| inst.loader == ModLoader::Vanilla)
}

pub fn open(app: &mut App, hit: Hit) {
    let project = &mut app.browse.project;
    if project.hit.as_ref().map(|current| &current.project_id) != Some(&hit.project_id) {
        project.installing = false;
        project.done = false;
        project.error = None;
        project.installed_item = None;
        project.mp_current = 0;
        project.mp_total = 0;
        project.mp_message.clear();
    }
    project.hit = Some(hit);
    project.open = true;
    if !project.is_modpack() && project.selected_instance.is_empty() && !app.instances.is_empty() {
        let preferred = app.browse.from_instance.clone().filter(|id| app.instances.iter().any(|inst| &inst.id == id));
        app.browse.project.selected_instance = preferred.unwrap_or_else(|| app.instances[0].id.clone());
    }
    load(app);
}

/// The `$effect` that loads versions when the project or the target instance changes.
fn load(app: &mut App) {
    let Some(hit) = app.browse.project.hit.clone() else { return };
    let source = Source::from_id(&hit.source);
    if app.browse.project.is_modpack() {
        let project = &mut app.browse.project;
        project.generation += 1;
        let generation = project.generation;
        project.loading_versions = true;
        project.version_error = None;
        project.done = false;
        project.error = None;
        let id = hit.project_id.clone();
        app.fx.spawn(async move { crate::commands::content_versions(source, id, None, None).await }, move |app, result| {
            let project = &mut app.browse.project;
            if project.generation != generation {
                return;
            }
            match result {
                Ok(versions) => {
                    if versions.is_empty() {
                        project.version_error = Some(tr("browse.noDownloadableVersion"));
                    }
                    project.versions = versions;
                }
                Err(e) => {
                    project.versions.clear();
                    project.version_error = Some(e.to_string());
                }
            }
            project.loading_versions = false;
        });
        return;
    }
    let Some(inst) = app.instance(&app.browse.project.selected_instance).cloned() else { return };
    if mod_on_vanilla(app) {
        return;
    }
    let project = &mut app.browse.project;
    project.generation += 1;
    let generation = project.generation;
    project.loading_versions = true;
    project.version_error = None;
    project.done = false;
    project.error = None;
    project.installed_item = None;
    let loader = inst.loader.id();
    let filter = (hit.project_type == "mod" && inst.loader != ModLoader::Vanilla).then(|| loader.to_string());
    let (id, mc_version) = (hit.project_id.clone(), inst.mc_version.clone());
    app.fx.spawn(async move { crate::commands::content_versions(source, id, filter, Some(mc_version)).await }, move |app, result| {
        let project = &mut app.browse.project;
        if project.generation != generation {
            return;
        }
        match result {
            Ok(versions) => {
                if versions.is_empty() {
                    project.version_error = Some(tf("browse.noCompatibleVersion", &[("loader", loader), ("mcVersion", &inst.mc_version)]));
                }
                project.versions = versions;
                let project_id = hit.project_id.clone();
                app.fx.blocking(
                    move |host| crate::commands::list_content(host.clone(), inst.id),
                    move |app, result| {
                        let project = &mut app.browse.project;
                        if project.generation != generation {
                            return;
                        }
                        match result {
                            Ok(items) => project.installed_item = items.into_iter().find(|item| item.project_id.as_deref() == Some(&project_id)),
                            Err(e) => {
                                project.versions.clear();
                                project.version_error = Some(e.to_string());
                            }
                        }
                        project.loading_versions = false;
                    },
                );
            }
            Err(e) => {
                project.versions.clear();
                project.version_error = Some(e.to_string());
                project.loading_versions = false;
            }
        }
    });
}

fn install(app: &mut App) {
    if app.instance(&app.browse.project.selected_instance).is_none() {
        return;
    }
    let project = &mut app.browse.project;
    let (Some(hit), Some(version)) = (project.hit.clone(), project.versions.first().cloned()) else { return };
    project.installing = true;
    project.error = None;
    let generation = project.generation;
    let (host, instance_id) = (app.fx.host.clone(), project.selected_instance.clone());
    let source = Source::from_id(&hit.source);
    app.fx.spawn(
        async move {
            crate::commands::install_content(host, instance_id, source, version.id, hit.project_type, hit.title, hit.icon_url).await
        },
        move |app, result| {
            let project = &mut app.browse.project;
            if project.generation == generation {
                match result {
                    Ok(item) => {
                        project.installed_item = Some(item);
                        project.done = true;
                    }
                    Err(e) => project.error = Some(e.to_string()),
                }
            }
            project.installing = false;
        },
    );
}

fn install_modpack(app: &mut App) {
    let project = &mut app.browse.project;
    let (Some(hit), Some(version)) = (project.hit.clone(), project.versions.first().cloned()) else { return };
    project.installing = true;
    project.error = None;
    project.mp_current = 0;
    project.mp_total = 0;
    project.mp_message = tr("browse.starting");
    // installStore.beginStart: the dock shows a loader before the instance exists.
    let token = app.install.begin_start();
    let host = app.fx.host.clone();
    let source = Source::from_id(&hit.source);
    app.fx.spawn(async move { crate::commands::install_modpack(host, source, version.id, hit.icon_url).await }, move |app, result| {
        app.install.end_start(token);
        app.browse.project.installing = false;
        match result {
            Ok(inst) => {
                app.refresh_instances();
                close(app);
                if app.settings.open_instance_on_install {
                    app.go(Route::Instance(inst.id));
                }
            }
            Err(e) => {
                app.browse.project.error = Some(e.to_string());
                app.install.pending = None;
            }
        }
    });
}

fn close(app: &mut App) {
    let project = &mut app.browse.project;
    project.versions.clear();
    project.version_error = None;
    project.error = None;
    project.done = false;
    project.mp_current = 0;
    project.mp_total = 0;
    project.open = false;
    app.browse.picker.open = false;
}

pub fn on_event(app: &mut App, name: &str, payload: &serde_json::Value) {
    let project = &mut app.browse.project;
    if name == "modpack-progress" && project.open && project.is_modpack() {
        let count = |key: &str| payload.get(key).and_then(|value| value.as_u64()).unwrap_or(0) as usize;
        project.mp_current = count("current");
        project.mp_total = count("total");
        project.mp_message = payload.get("message").and_then(|value| value.as_str()).unwrap_or("").to_string();
    }
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    let project = &app.browse.project;
    let Some(hit) = project.hit.as_ref().filter(|_| project.open) else { return };
    let theme = app.theme();
    let is_modpack = project.is_modpack();
    let mut picker_layer = Vec::new();

    let icon = match hit.icon_url.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => ui::image(app, url, 60.0, 60.0).fill(theme.bg_card),
        None => el().size(60.0, 60.0).center().fill(theme.bg_card).child(ui::colored_icon(&icons::PACKAGE, 28.0, theme.text_muted)),
    };
    let icon = el().key("icon").size(64.0, 64.0).border(2.0, Color::BLACK.alpha(0.3)).child(icon);
    let head = row()
        .key("head")
        .gap(14.0)
        .child(icon)
        .child(
            col()
                .grow(1.0)
                .shrink()
                .child(ui::heading(&theme, hit.title.clone(), 17.0))
                .child(ui::muted(&theme, tf("browse.byAuthor", &[("author", &hit.author)]), 12.5).mt(3.0).mb(8.0))
                .child(
                    row()
                        .gap(12.0)
                        .center_y()
                        .child(
                            row()
                                .gap(5.0)
                                .center_y()
                                .child(ui::colored_icon(&icons::PACKAGE, 13.0, theme.text_secondary))
                                .child(ui::secondary(&theme, tf("browse.downloads", &[("count", &fmt::count(hit.downloads))]), 12.5)),
                        )
                        .child(
                            text(super::capitalize(&hit.project_type))
                                .font(theme.sans(12.5, 400.0))
                                .color(theme.text_muted)
                                .px(8.0)
                                .py(2.0)
                                .fill(theme.bg_card)
                                .radius(10.0),
                        ),
                ),
        );
    let desc = text(hit.description.clone())
        .key("desc")
        .font(rune::tok::TextStyle { line_height: 21.6, ..theme.sans(13.5, 400.0) })
        .color(theme.text_secondary)
        .wrap()
        .my(16.0);

    let warn = |message: String| text(message).font(rune::tok::TextStyle { line_height: 18.75, ..theme.sans(12.5, 400.0) }).color(theme.warning).wrap();
    let muted = |message: String| text(message).font(theme.sans(12.5, 400.0)).color(theme.text_muted);
    let mut boxed = super::select::sunken(col().key("install-box").padding(14.0).fill(theme.bg_input).border(2.0, theme.border));
    if is_modpack {
        let mut note = vec![
            text(tr("browse.installsAsA")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary),
            text(tr("browse.newInstance")).font(theme.sans(13.0, 700.0)).color(theme.text_secondary),
        ];
        let suffix = match project.versions.first() {
            Some(latest) => match latest.game_versions.first() {
                Some(mc) => tf("browse.mpVersionMc", &[("version", &latest.version_number), ("mc", mc)]),
                None => tf("browse.mpVersion", &[("version", &latest.version_number)]),
            },
            None => String::new(),
        };
        note.push(text(format!("{suffix}.")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary));
        boxed = boxed.child(row().key("mp-note").flex_wrap().mb(8.0).children(note));
        if project.loading_versions {
            boxed = boxed.child(muted(tr("browse.findingLatestVersion")));
        } else if let Some(error) = &project.version_error {
            boxed = boxed.child(warn(error.clone()));
        }
        if project.installing {
            let label = if project.mp_message.is_empty() { tr("browse.installing") } else { project.mp_message.clone() };
            boxed = boxed.child(col().key("mp-progress").mt(12.0).child(ui::progress(&theme, label, to_percent(project.mp_current, project.mp_total))));
        }
    } else if app.instances.is_empty() {
        boxed = boxed.child(text(tr("browse.createInstanceFirst")).font(theme.sans(13.0, 400.0)).color(theme.text_muted).wrap());
    } else {
        boxed = boxed.child(ui::field_label(&theme, tr("browse.installTo"))).child(picker::view(
            app,
            cx,
            &mut picker_layer,
            &project.selected_instance,
            |app, id| {
                if app.browse.project.selected_instance != id {
                    app.browse.project.selected_instance = id;
                    load(app);
                }
            },
        ));
        if mod_on_vanilla(app) {
            boxed = boxed.child(warn(tr("browse.modNeedsLoader")).mt(10.0));
        } else {
            let mut line = row().key("version-line").mt(10.0).gap(10.0).center_y().flex_wrap();
            if project.loading_versions {
                line = line.child(muted(tr("browse.findingCompatibleVersion")));
            } else if let Some(error) = &project.version_error {
                line = line.child(warn(error.clone()));
            } else if let Some(latest) = project.versions.first() {
                line = line.child(
                    row()
                        .child(muted(tr("browse.latestMatch")))
                        .child(text(latest.version_number.clone()).font(theme.sans(12.5, 700.0)).color(theme.text_muted)),
                );
                let installed = project.installed_item.as_ref();
                if installed.is_some_and(|item| item.version_id == latest.id) {
                    line = line.child(pill(&theme, format!("✓ {}", tr("browse.installed")), theme.accent));
                } else if installed.is_some() {
                    line = line.child(pill(&theme, tr("browse.updateAvailable"), theme.warning));
                }
            }
            boxed = boxed.child(line);
            if !project.versions.is_empty() && !can_download(project) {
                let title = hit.title.clone();
                boxed = boxed.child(
                    row()
                        .key("cf-optout")
                        .mt(10.0)
                        .flex_wrap()
                        .child(text(tr("browse.cfOptout")).font(theme.sans(12.5, 400.0)).color(theme.text_muted).wrap().shrink())
                        .child(
                            text(tr("browse.curseforgePage"))
                                .font(theme.sans(12.5, 400.0))
                                .color(theme.accent)
                                .role("link")
                                .cursor(Cursor::Pointer)
                                .on_press(move |_: &mut App| {
                                    let query: String = url_encode(&title);
                                    let _ = open::that(format!("https://www.curseforge.com/minecraft/search?search={query}"));
                                }),
                        )
                        .child(muted(".".into())),
                );
            }
        }
    }
    let mut body = col().key("project").child(head).child(desc).child(boxed);
    if let Some(error) = &project.error {
        body = body.child(text(error.clone()).font(theme.sans(13.0, 400.0)).color(theme.danger).wrap().mt(10.0));
    }

    let mut footer = vec![Btn::new(tr("common.close")).key("close-btn").ghost().disabled(project.installing).on_press(close).build(&theme)];
    if is_modpack {
        footer.push(
            Btn::new(if project.installing { tr("browse.installing") } else { tr("browse.installModpack") })
                .key("install-modpack")
                .primary()
                .disabled(project.installing || project.loading_versions || project.versions.is_empty())
                .on_press(install_modpack)
                .build(&theme),
        );
    } else if project.done {
        let up_to_date = is_up_to_date(project);
        footer.push(
            text(format!("✓ {}", if up_to_date { tr("browse.installed") } else { tr("common.done") }))
                .key("ok-pill")
                .font(theme.sans(13.0, 600.0))
                .color(theme.accent),
        );
    } else {
        let label = if project.installing {
            tr("browse.installing")
        } else if is_up_to_date(project) {
            tr("browse.reinstall")
        } else if project.installed_item.is_some() {
            tr("browse.update")
        } else {
            tr("browse.install")
        };
        footer.push(
            Btn::new(label)
                .key("install")
                .primary()
                .disabled(project.installing || project.loading_versions || project.versions.is_empty() || !can_download(project) || mod_on_vanilla(app))
                .on_press(install)
                .build(&theme),
        );
    }
    layer.push(ui::modal(&theme, "project-modal", hit.title.clone(), 640.0, body, Some(ui::buttons(footer)), close));
    layer.extend(picker_layer);
}

fn pill(theme: &crate::theme::Theme, label: String, color: Color) -> El {
    text(label).font(theme.pixel(11.0)).color(color).px(9.0).py(3.0).border(2.0, color)
}

fn can_download(project: &State) -> bool {
    project.versions.first().and_then(|latest| latest.files.iter().find(|file| file.primary).or(latest.files.first())).is_some_and(|file| !file.url.is_empty())
}

fn is_up_to_date(project: &State) -> bool {
    matches!((&project.installed_item, project.versions.first()), (Some(installed), Some(latest)) if installed.version_id == latest.id)
}

/// `encodeURIComponent`.
fn url_encode(input: &str) -> String {
    input.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

