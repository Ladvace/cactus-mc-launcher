//! `CreateInstanceModal.svelte` and `funnyNames.ts`.

use std::rc::Rc;

use rune::{Cursor, Cx, El, TextEdit, col, el, row, text};

use super::widgets::{self, Opt, Select, chip, chip_icon, chip_text};
use crate::app::{App, Route};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::instance::{CreateInstance, InstanceKind, ModLoader};
use crate::loader::LoaderVersion;
use crate::minecraft::MinecraftVersion;
use crate::theme::Theme;
use crate::ui::{self, Btn};

/// The modal body's inner width: 480 less its border and 18 pt padding.
const BODY_W: f32 = 440.0;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Dropdown {
    #[default]
    None,
    Version,
    LoaderVersion,
}

#[derive(Default)]
pub struct State {
    name: TextEdit,
    kind: InstanceKind,
    eula: bool,
    loader: ModLoader,
    version: String,
    snapshots: bool,
    versions: Vec<MinecraftVersion>,
    versions_loading: bool,
    version_error: Option<String>,
    creating: bool,
    loader_versions: Vec<LoaderVersion>,
    loader_loading: bool,
    loader_error: Option<String>,
    loader_version: String,
    /// Bumped per loader-version request so a stale answer is dropped.
    loader_req: u64,
    dropdown: Dropdown,
    search: TextEdit,
}

fn initial_loader(app: &App) -> ModLoader {
    ModLoader::from_id(&app.settings.default_loader)
}

fn needs_loader_version(loader: ModLoader) -> bool {
    loader != ModLoader::Vanilla
}

pub fn reset(app: &mut App) {
    let loader = initial_loader(app);
    let form = &mut app.modals.create;
    form.name = TextEdit::default();
    form.kind = InstanceKind::Client;
    form.eula = false;
    form.loader = loader;
    form.snapshots = false;
    form.loader_versions.clear();
    form.loader_version.clear();
    form.loader_error = None;
    form.dropdown = Dropdown::None;
}

fn close(app: &mut App) {
    reset(app);
    app.ui.create_instance_open = false;
}

fn load_versions(app: &mut App) {
    let form = &mut app.modals.create;
    if form.versions_loading {
        return;
    }
    form.versions_loading = true;
    form.version_error = None;
    app.fx.spawn(async { crate::commands::get_minecraft_versions().await }, |app, result| {
        let form = &mut app.modals.create;
        form.versions_loading = false;
        match result {
            Ok(list) => {
                form.versions = list.versions;
                form.version = list.latest_release;
                load_loader_versions(app);
            }
            Err(e) => form.version_error = Some(e.to_string()),
        }
    });
}

/// The `$effect` on (loader, selectedVersion).
fn load_loader_versions(app: &mut App) {
    let form = &mut app.modals.create;
    form.loader_req += 1;
    form.loader_versions.clear();
    form.loader_error = None;
    form.loader_version.clear();
    if !needs_loader_version(form.loader) || form.version.is_empty() {
        form.loader_loading = false;
        return;
    }
    form.loader_loading = true;
    let (req, loader, mc) = (form.loader_req, form.loader, form.version.clone());
    let request_mc = mc.clone();
    app.fx.spawn(async move { crate::commands::get_loader_versions(loader, request_mc).await }, move |app, result| {
        let form = &mut app.modals.create;
        if form.loader_req != req {
            return;
        }
        form.loader_loading = false;
        match result {
            Ok(list) => {
                if list.is_empty() {
                    let name = loader.id();
                    form.loader_error = Some(tf("create.noBuilds", &[("loader", name), ("version", &mc)]));
                }
                form.loader_versions = list;
            }
            Err(e) => form.loader_error = Some(e.to_string()),
        }
    });
}

fn can_create(app: &App) -> bool {
    let form = &app.modals.create;
    !form.name.text.trim().is_empty()
        && !form.version.is_empty()
        && (!needs_loader_version(form.loader) || (!form.loader_loading && !form.loader_versions.is_empty()))
        && (form.kind == InstanceKind::Client || form.eula)
        && !form.creating
}

fn create(app: &mut App) {
    if !can_create(app) {
        return;
    }
    let form = &mut app.modals.create;
    form.creating = true;
    let payload = CreateInstance {
        name: form.name.text.trim().to_string(),
        kind: form.kind,
        mc_version: form.version.clone(),
        loader: form.loader,
        loader_version: (form.loader != ModLoader::Vanilla && !form.loader_version.is_empty()).then(|| form.loader_version.clone()),
        icon: None,
    };
    app.fx.blocking(move |host| crate::commands::create_instance(host.clone(), host.state(), payload), |app, result| {
        app.modals.create.creating = false;
        match result {
            Ok(inst) => {
                app.refresh_instances();
                close(app);
                app.go(Route::Instance(inst.id));
            }
            Err(e) => app.modals.create.version_error = Some(e.to_string()),
        }
    });
}

pub fn view(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) {
    let form = &app.modals.create;
    if form.versions.is_empty() && !form.versions_loading && form.version_error.is_none() {
        app.fx.post(|app| {
            if app.modals.create.versions.is_empty() {
                load_versions(app);
            }
        });
    }
    let theme = app.theme();
    let mut overlays = Vec::new();
    let body = col()
        .key("form")
        .gap(18.0)
        .w(BODY_W)
        .child(kind_toggle(&theme, form.kind))
        .child(name_field(&theme, form))
        .child(loader_field(&theme, form, &mut overlays))
        .child(version_field(&theme, form, &mut overlays))
        .children((form.kind == InstanceKind::Server).then(|| eula(&theme, form.eula)));
    let footer = ui::buttons([
        Btn::new(tr("common.cancel")).ghost().key("cancel").on_press(close).build(&theme),
        Btn::new(if form.creating { tr("common.creating") } else { tr("common.create") })
            .primary()
            .key("create")
            .disabled(!can_create(app))
            .on_press(create)
            .build(&theme),
    ]);
    layer.push(ui::modal(&theme, "create-instance", tr("create.title"), 480.0, body, Some(footer), close));
    layer.extend(overlays);
}

fn kind_toggle(theme: &Theme, kind: InstanceKind) -> El {
    let tab = |tab_kind: InstanceKind, key: &str, icon: &'static rune::Icon, label: String| {
        let on = kind == tab_kind;
        chip(
            theme,
            key,
            on,
            row().grow(1.0).center().gap(8.0).padding(12.0).child(chip_icon(theme, icon, 16.0, on)).child(chip_text(theme, label, 13.0, 600.0, on)),
        )
        .role("tab")
        .on_press(move |app: &mut App| app.modals.create.kind = tab_kind)
    };
    row()
        .key("kind")
        .role("tablist")
        .label(tr("create.instanceType"))
        .gap(8.0)
        .child(tab(InstanceKind::Client, "client", &icons::CUBE, tr("create.client")))
        .child(tab(InstanceKind::Server, "server", &icons::GLOBE, tr("create.server")))
}

fn name_field(theme: &Theme, form: &State) -> El {
    let dice = el()
        .key("dice")
        .w(42.0)
        .self_stretch()
        .center()
        .fill(theme.bg_input)
        .box_shadow(rune::Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: rune::Color::BLACK.alpha(0.28), inset: true })
        .role("button")
        .label(tr("create.rollRandomName"))
        .help(tr("create.rollRandomName"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .press_scale(0.9)
        .on_press(|app: &mut App| app.modals.create.name.replace_text(random_instance_name()))
        .child(ui::colored_icon(&icons::SHUFFLE, 16.0, theme.text_secondary).group_color(theme.accent));
    col()
        .key("name")
        .child(ui::field_label(theme, tr("create.name")))
        .child(
            row()
                .gap(8.0)
                .child(widgets::field(theme, &form.name, tr("create.namePlaceholder"), |app| &mut app.modals.create.name, Some(create)).grow(1.0))
                .child(widgets::boxed(theme, dice, false)),
        )
}

/// The chip's loader logo, coloured like `chip_text` beside it.
fn loader_glyph(theme: &Theme, loader: ModLoader, on: bool) -> El {
    let glyph = ui::loader_icon(loader, 16.0, if on { theme.accent } else { theme.text_secondary });
    if on { glyph } else { glyph.group_color(theme.text) }
}

fn loader_field(theme: &Theme, form: &State, layer: &mut Vec<El>) -> El {
    // `repeat(auto-fit, minmax(84px, 1fr))` in 440 pt: four columns.
    let cols = 4;
    let w = (BODY_W - 8.0 * (cols as f32 - 1.0)) / cols as f32;
    let mut grid = col().gap(8.0);
    for chunk in ModLoader::ALL.chunks(cols) {
        let mut line = row().gap(8.0);
        for &loader in chunk {
            let (id, label) = (loader.id(), loader.label());
            let on = form.loader == loader;
            line = line.child(
                chip(
                    theme,
                    format!("loader-{id}"),
                    on,
                    row().w(w).center().gap(7.0).px(12.0).py(11.0).child(loader_glyph(theme, loader, on)).child(chip_text(theme, label, 13.0, 500.0, on)),
                )
                .on_press(move |app: &mut App| {
                    if app.modals.create.loader != loader {
                        app.modals.create.loader = loader;
                        load_loader_versions(app);
                    }
                }),
            );
        }
        grid = grid.child(line);
    }
    let mut field = col().key("loader").child(ui::field_label(theme, tr("create.modLoader"))).child(grid);
    if needs_loader_version(form.loader) {
        let mut loader_version = col().key("loader-version").mt(14.0).child(ui::field_label(theme, tr("create.loaderVersion")));
        loader_version = if form.loader_loading {
            loader_version.child(status(theme, tr("create.loadingLoaderBuilds"), false))
        } else if let Some(error) = &form.loader_error {
            loader_version.child(status(theme, error.clone(), true))
        } else {
            let mut opts = vec![Opt::new("", tr("create.latestStable"))];
            opts.extend(form.loader_versions.iter().map(|build| {
                Opt::new(build.version.clone(), format!("{}{}", build.version, if build.stable { "" } else { " (beta)" }))
            }));
            loader_version.child(
                Select {
                    key: "ci-loader-version",
                    options: &opts,
                    value: &form.loader_version,
                    open: form.dropdown == Dropdown::LoaderVersion,
                    width: BODY_W,
                    toggle: |app| toggle(app, Dropdown::LoaderVersion),
                    close: |app| app.modals.create.dropdown = Dropdown::None,
                    choose: Rc::new(|app: &mut App, version: &str| app.modals.create.loader_version = version.to_string()),
                    search: None,
                    header: None,
                }
                .build(theme, layer),
            )
        };
        if matches!(form.loader, ModLoader::Forge | ModLoader::NeoForge) {
            loader_version = loader_version.child(hint(theme, tr("create.installerNote"), theme.text_muted));
        }
        field = field.child(loader_version);
    }
    field
}

fn toggle(app: &mut App, dropdown: Dropdown) {
    let form = &mut app.modals.create;
    form.dropdown = if form.dropdown == dropdown { Dropdown::None } else { dropdown };
    form.search = TextEdit::default();
}

fn version_field(theme: &Theme, form: &State, layer: &mut Vec<El>) -> El {
    let mut field = col().key("version").child(ui::field_label(theme, tr("create.minecraftVersion")));
    if form.versions_loading {
        return field.child(status(theme, tr("create.loadingVersions"), false));
    }
    if let Some(error) = &form.version_error {
        return field.child(status(theme, error.clone(), true)).child(
            row().child(Btn::new(tr("common.retry")).ghost().key("retry").on_press(load_versions).build(theme)),
        );
    }
    let opts: Vec<Opt> = form
        .versions
        .iter()
        .filter(|version| form.snapshots || version.kind == "release")
        .map(|version| {
            let release = version.kind == "release";
            Opt {
                value: version.id.clone(),
                label: if release { version.id.clone() } else { format!("{} ({})", version.id, version.kind) },
                muted: !release,
            }
        })
        .collect();
    let snap = row()
        .key("snapshots")
        .gap(6.0)
        .center_y()
        .role("switch")
        .checked(form.snapshots)
        .label(tr("create.showSnapshots"))
        .help(tr("create.showSnapshots"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.modals.create.snapshots = !app.modals.create.snapshots)
        .child(text(tr("create.snapshots")).font(theme.sans(11.0, 400.0)).color(theme.text_secondary))
        .child(widgets::mini_switch(theme, form.snapshots));
    field = field.child(
        Select {
            key: "ci-version",
            options: &opts,
            value: &form.version,
            open: form.dropdown == Dropdown::Version,
            width: BODY_W,
            toggle: |app| toggle(app, Dropdown::Version),
            close: |app| app.modals.create.dropdown = Dropdown::None,
            choose: Rc::new(|app: &mut App, version: &str| {
                if app.modals.create.version != version {
                    app.modals.create.version = version.to_string();
                    load_loader_versions(app);
                }
            }),
            search: Some((&form.search, |app| &mut app.modals.create.search)),
            header: Some(snap),
        }
        .build(theme, layer),
    );
    field
}

fn eula(theme: &Theme, accepted: bool) -> El {
    let check = row()
        .key("eula-check")
        .gap(8.0)
        .role("checkbox")
        .checked(accepted)
        .label(tr("create.eulaAgree"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.modals.create.eula = !app.modals.create.eula)
        .child(ui::checkbox(theme, accepted).a11y_hidden())
        .child(
            row()
                .shrink()
                .flex_wrap()
                .child(text(format!("{} ", tr("create.eulaAgree"))).font(theme.sans(13.0, 400.0)).color(theme.text))
                .child(
                    text(tr("create.minecraftEula"))
                        .font(theme.sans(13.0, 400.0))
                        .color(theme.accent)
                        .key("eula-link")
                        .role("link")
                        .cursor(Cursor::Pointer)
                        .on_press(|_: &mut App| {
                            let _ = open::that("https://www.minecraft.net/eula");
                        }),
                )
                .child(text(".").font(theme.sans(13.0, 400.0)).color(theme.text)),
        );
    col()
        .key("eula")
        .padding(12.0)
        .fill(theme.bg_input)
        .border(2.0, theme.border)
        .child(check)
        .child(hint(theme, tr("create.serverHeadless"), theme.text_muted))
}

/// `.status`: a framed line of loading or error text.
fn status(theme: &Theme, msg: String, error: bool) -> El {
    el().padding(10.0)
        .px(12.0)
        .fill(theme.bg_input)
        .border(2.0, theme.border)
        .mb(if error { 8.0 } else { 0.0 })
        .child(text(msg).font(theme.sans(13.0, 400.0)).color(if error { theme.danger } else { theme.text_secondary }).wrap())
}

fn hint(theme: &Theme, msg: String, color: rune::Color) -> El {
    text(msg).font(theme.sans(12.0, 400.0)).color(color).wrap().mt(8.0)
}

// funnyNames.ts

const ADJECTIVES: [&str; 29] = [
    "Chunky", "Sneaky", "Blocky", "Explosive", "Mossy", "Enchanted", "Cursed", "Glowing", "Frosty", "Pixelated",
    "Legendary", "Grumpy", "Turbo", "Soggy", "Spicy", "Wobbly", "Feral", "Dizzy", "Sleepy", "Rowdy", "Golden", "Molten",
    "Haunted", "Squeaky", "Crunchy", "Diamond", "Redstone", "Obsidian", "Nether",
];

const NOUNS: [&str; 29] = [
    "Creeper", "Enderman", "Llama", "Piglin", "Villager", "Bastion", "Beacon", "Cavern", "Fortress", "Sheep", "Goat",
    "Axolotl", "Warden", "Allay", "Slime", "Zombie", "Wolf", "Parrot", "Bee", "Ghast", "Blaze", "Golem", "Phantom",
    "Turtle", "Panda", "Fox", "Dolphin", "Shulker", "Ravager",
];

const SPECIALS: [&str; 14] = [
    "Diamonds Are Forever", "Creeper Aw Man", "One More Night Mining", "Just One More Diamond", "The Nether Regrets",
    "Dirt Hut Deluxe", "Respawn & Repeat", "404 Diamonds Not Found", "Cobblestone Cathedral", "Certified Creeper Farm",
    "Elytra or Bust", "Bed Wars Survivor", "Peaceful Mode Liar", "It's Just A Phase (Nether)",
];

pub fn random_instance_name() -> String {
    let seed = uuid::Uuid::new_v4().as_u128();
    let pick = |arr: &[&str], shift: u32| arr[((seed >> shift) % arr.len() as u128) as usize].to_string();
    if (seed % 100) < 18 {
        return pick(&SPECIALS, 16);
    }
    format!("{} {}", pick(&ADJECTIVES, 32), pick(&NOUNS, 64))
}
