//! Cactus Launcher on Rune: the UI (`app`, `shell`, `screens`, `overlays`, `ui`) over the launcher
//! backend in `src/backend` (commands, stores, launching), which `host` connects to the app.

#![allow(dead_code)]

mod app;
#[path = "backend/host.rs"] mod host;
mod core;
mod icons;
mod overlays;
mod screens;
mod shell;
mod theme;
mod ui;

#[path = "backend/achievements.rs"] mod achievements;
#[path = "backend/auth/mod.rs"] mod auth;
#[path = "backend/board_auth.rs"] mod board_auth;
#[path = "backend/commands.rs"] mod commands;
#[path = "backend/content/mod.rs"] mod content;
#[path = "backend/error.rs"] mod error;
#[path = "backend/friends.rs"] mod friends;
#[path = "backend/http.rs"] mod http;
#[path = "backend/import_launcher.rs"] mod import_launcher;
#[path = "backend/instance/mod.rs"] mod instance;
#[path = "backend/launch/mod.rs"] mod launch;
#[path = "backend/loader/mod.rs"] mod loader;
#[path = "backend/minecraft/mod.rs"] mod minecraft;
#[path = "backend/modrinth/mod.rs"] mod modrinth;
#[path = "backend/news.rs"] mod news;
#[path = "backend/paths.rs"] mod paths;
#[path = "backend/players.rs"] mod players;
#[path = "backend/restore.rs"] mod restore;
#[path = "backend/server_browse.rs"] mod server_browse;
#[path = "backend/server_ping.rs"] mod server_ping;
#[path = "backend/servers_dat.rs"] mod servers_dat;
#[path = "backend/settings.rs"] mod settings;
#[path = "backend/snapshot.rs"] mod snapshot;
#[path = "backend/sources/mod.rs"] mod sources;
#[path = "backend/status.rs"] mod status;
#[path = "backend/stickers.rs"] mod stickers;
#[path = "backend/tuneup.rs"] mod tuneup;
#[path = "backend/tunnel.rs"] mod tunnel;
#[path = "backend/worlds.rs"] mod worlds;

fn main() {
    rune::run(rune::AppSpec::<app::App> {
        min_size: Some((900.0, 600.0)),
        fonts: &[core::assets::INTER_REGULAR, core::assets::INTER_MEDIUM, core::assets::INTER_SEMIBOLD, core::assets::PIXELIFY],
        families: (theme::SANS_FAMILY, "ui-monospace"),
        setup: Some(app::setup),
        poll: Some(app::poll),
        on_key: Some(app::on_key),
        scenes: SCENES,
        ..rune::AppSpec::new("Cactus Launcher", (1180.0, 760.0), app::init, app::view)
    });
}

/// `--scene <name>` for headless snapshots: one per route and per major overlay.
static SCENES: &[(&str, fn(&mut app::App))] = &[
    ("home", |_| {}),
    ("browse", |app| app.go(app::Route::Browse)),
    ("browse-instance", |app| {
        app.ui.browse_instance_id = app.instances.first().map(|instance| instance.id.clone());
        app.go(app::Route::Browse)
    }),
    ("servers", |app| app.go(app::Route::Servers)),
    ("community", |app| app.go(app::Route::Community)),
    ("share-creator", |app| app.go(app::Route::ShareCreator)),
    ("achievements", |app| app.go(app::Route::Achievements)),
    ("settings", |app| app.go(app::Route::Settings)),
    ("instance", |app| {
        if let Some(id) = app.instances.first().map(|instance| instance.id.clone()) {
            app.go(app::Route::Instance(id));
        }
    }),
    ("launching", |app| {
        if let Some(id) = app.instances.first().map(|instance| instance.id.clone()) {
            app.launch.entry(id).or_default().state = "preparing".into();
        }
    }),
    ("create", |app| app.ui.create_instance_open = true),
    ("accounts", |app| app.ui.accounts_open = true),
    ("palette", |app| app.ui.command_palette_open = true),
    ("changelog", |app| app.ui.changelog_open = true),
    ("crafting", |app| app.ui.crafting_open = true),
    ("onboarding", |app| app.ui.onboarded = false),
    ("toasts", |app| {
        app.toasts.success("Instance created");
        app.toasts.info("Checking for updates…");
        app.toasts.error("Couldn't reach Modrinth: connection timed out");
    }),
];
