use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::auth::{self, AccountInfo, AccountStore, AccountsState};
use crate::content::{self, ContentItem};
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::{CreateInstance, Instance, InstanceKind, UpdateInstance};
use crate::instance::ModLoader;
use crate::launch::{self, LaunchState, ServerState};
use crate::loader::{self, LoaderVersion};
use crate::minecraft::{self, VersionList};
use crate::modrinth::{SearchParams, SearchResults, Version as ModrinthVersion};
use crate::settings::{Settings, SettingsStore};
use crate::sources::{self, Source, SourceInfo};

#[tauri::command]
pub fn list_instances(store: State<'_, InstanceStore>) -> Vec<Instance> {
    store.list()
}

/// True when running inside a Flatpak sandbox (Flathub sets `FLATPAK_ID`). Used
/// to disable the built-in self-updater — Flatpak apps update through the store.
#[tauri::command]
pub fn is_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

#[tauri::command]
pub fn get_instance(store: State<'_, InstanceStore>, id: String) -> Result<Instance> {
    store.get(&id).ok_or(AppError::InstanceNotFound(id))
}

#[tauri::command]
pub fn create_instance(
    app: AppHandle,
    store: State<'_, InstanceStore>,
    payload: CreateInstance,
) -> Result<Instance> {
    let instance = Instance::new(
        store.unique_name(&payload.name),
        payload.kind,
        payload.mc_version,
        payload.loader,
        payload.loader_version,
        payload.icon,
    );
    store.save(&app, &instance)?;
    // `save` pins the game dir from the global setting, so return the stored copy.
    Ok(store.get(&instance.id).unwrap_or(instance))
}

#[tauri::command]
pub fn update_instance(
    app: AppHandle,
    store: State<'_, InstanceStore>,
    id: String,
    patch: UpdateInstance,
) -> Result<Instance> {
    let mut instance = store.get(&id).ok_or_else(|| AppError::InstanceNotFound(id.clone()))?;

    if let Some(name) = patch.name {
        instance.name = name;
    }
    if let Some(icon) = patch.icon {
        // An empty string is the "reset to default" signal — clear the icon.
        instance.icon = if icon.is_empty() { None } else { Some(icon) };
    }
    if let Some(group) = patch.group {
        // Empty string moves the instance out of any group.
        instance.group = if group.is_empty() { None } else { Some(group) };
    }
    if let Some(mc_version) = patch.mc_version {
        instance.mc_version = mc_version;
    }
    if let Some(loader) = patch.loader {
        instance.loader = loader;
    }
    if let Some(loader_version) = patch.loader_version {
        instance.loader_version = Some(loader_version);
    }
    if let Some(cover_image) = patch.cover_image {
        instance.cover_image = cover_image;
    }
    if let Some(mem) = patch.server_memory_mb {
        // 0 = clear the override and fall back to the global memory setting.
        instance.server_memory_mb = if mem == 0 { None } else { Some(mem) };
    }
    // Per-instance overrides: 0 / empty string clears back to the global value.
    if let Some(value) = patch.max_memory_mb {
        instance.max_memory_mb = (value != 0).then_some(value);
    }
    if let Some(value) = patch.min_memory_mb {
        instance.min_memory_mb = (value != 0).then_some(value);
    }
    if let Some(value) = patch.jvm_args {
        instance.jvm_args = (!value.trim().is_empty()).then_some(value);
    }
    if let Some(value) = patch.java_path {
        instance.java_path = (!value.trim().is_empty()).then_some(value);
    }
    if let Some(value) = patch.game_width {
        instance.game_width = (value != 0).then_some(value);
    }
    if let Some(value) = patch.game_height {
        instance.game_height = (value != 0).then_some(value);
    }
    if let Some(value) = patch.ngrok_authtoken {
        instance.ngrok_authtoken = (!value.trim().is_empty()).then_some(value);
    }

    store.save(&app, &instance)?;
    Ok(instance)
}

#[tauri::command]
pub fn delete_instance(
    app: AppHandle,
    store: State<'_, InstanceStore>,
    id: String,
) -> Result<()> {
    store.delete(&app, &id)
}

#[tauri::command]
pub fn create_server_from(
    app: AppHandle,
    store: State<'_, InstanceStore>,
    id: String,
) -> Result<Instance> {
    let source = store.get(&id).ok_or_else(|| AppError::InstanceNotFound(id.clone()))?;

    let mut server = Instance::new(
        store.unique_name(&format!("{} (Server)", source.name)),
        InstanceKind::Server,
        source.mc_version.clone(),
        source.loader,
        source.loader_version.clone(),
        source.icon.clone(),
    );
    server.group = source.group.clone();
    store.save(&app, &server)?;

    // Content manifest lives with the record; mods/config live in the game dir.
    let source_manifest = crate::paths::instance_dir(&app, &id)?.join("content.json");
    if source_manifest.exists() {
        std::fs::copy(
            &source_manifest,
            crate::paths::instance_dir(&app, &server.id)?.join("content.json"),
        )?;
    }
    let source_game = crate::paths::instance_game_dir(&app, &id)?;
    let server_game = crate::paths::instance_game_dir(&app, &server.id)?;
    for sub in ["mods", "config"] {
        let from = source_game.join(sub);
        if from.exists() {
            copy_tree(&from, &server_game.join(sub))?;
        }
    }

    Ok(store.get(&server.id).unwrap_or(server))
}

#[tauri::command]
pub fn instance_folder(app: AppHandle, id: String) -> Result<String> {
    let dir = crate::paths::instance_game_dir(&app, &id)?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn set_instance_game_dir(
    app: AppHandle,
    store: State<'_, InstanceStore>,
    launch: State<'_, LaunchState>,
    servers: State<'_, ServerState>,
    id: String,
    path: Option<String>,
) -> Result<Instance> {
    if launch.is_running(&id) || servers.is_running(&id) {
        return Err(AppError::Other(
            "Stop the instance before moving its files.".into(),
        ));
    }
    let mut instance = store.get(&id).ok_or_else(|| AppError::InstanceNotFound(id.clone()))?;

    let old = crate::paths::instance_game_dir(&app, &id)?;
    let custom_dir = path.as_deref().map(str::trim).filter(|dir| !dir.is_empty());
    let target = match custom_dir {
        Some(dir) => std::path::PathBuf::from(dir),
        None => crate::paths::default_game_dir(&app, &id)?,
    };

    if old != target {
        move_tree(&old, &target)?;
    }
    instance.game_dir = custom_dir.map(|_| target.to_string_lossy().into_owned());
    store.save(&app, &instance)?;
    Ok(instance)
}

fn move_tree(old: &std::path::Path, target: &std::path::Path) -> Result<()> {
    if !old.exists() {
        std::fs::create_dir_all(target)?;
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !target.exists() && std::fs::rename(old, target).is_ok() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    copy_tree(old, target)?;
    std::fs::remove_dir_all(old)?;
    Ok(())
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    settings: Settings,
) -> Result<()> {
    store.save(&app, settings)
}

#[tauri::command]
pub async fn get_minecraft_versions() -> Result<VersionList> {
    minecraft::fetch_versions().await
}

#[tauri::command]
pub async fn get_loader_versions(
    loader: ModLoader,
    mc_version: String,
) -> Result<Vec<LoaderVersion>> {
    loader::list_versions(loader, &mc_version).await
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    launch: State<'_, LaunchState>,
    servers: State<'_, ServerState>,
    instances: State<'_, InstanceStore>,
    settings: State<'_, SettingsStore>,
    id: String,
) -> Result<()> {
    // Reject a double-launch: `is_instance_running` only turns true once the
    // process spawns, which is after a potentially minutes-long prepare phase,
    // so guard the whole window (including prepare) via `try_begin_start`.
    if servers.is_running(&id) || !launch.try_begin_start(&id) {
        return Err(AppError::Other(
            "This instance is already launching or running.".into(),
        ));
    }
    let Some(instance) = instances.get(&id) else {
        launch.finish_start(&id);
        return Err(AppError::InstanceNotFound(id));
    };
    let settings = settings.get();
    let result = match instance.kind {
        InstanceKind::Server => launch::server::launch(app, instance, settings).await,
        InstanceKind::Client => launch::launch(app, instance, settings).await,
    };
    launch.finish_start(&id);
    result
}

#[tauri::command]
pub fn stop_instance(
    launch: State<'_, LaunchState>,
    servers: State<'_, ServerState>,
    id: String,
) -> Result<()> {
    if !servers.stop(&id) {
        launch.kill(&id);
    }
    Ok(())
}

#[tauri::command]
pub fn send_server_command(
    servers: State<'_, ServerState>,
    id: String,
    command: String,
) -> Result<()> {
    servers.send(&id, command);
    Ok(())
}

#[tauri::command]
pub fn is_instance_running(
    launch: State<'_, LaunchState>,
    servers: State<'_, ServerState>,
    id: String,
) -> bool {
    launch.is_running(&id) || servers.is_running(&id)
}

#[tauri::command]
pub fn read_server_properties(app: AppHandle, id: String) -> Result<String> {
    let file = crate::paths::instance_game_dir(&app, &id)?.join("server.properties");
    Ok(std::fs::read_to_string(file).unwrap_or_default())
}

#[tauri::command]
pub fn write_server_properties(app: AppHandle, id: String, content: String) -> Result<()> {
    let file = crate::paths::instance_game_dir(&app, &id)?.join("server.properties");
    std::fs::write(file, content)?;
    Ok(())
}

#[tauri::command]
pub fn list_worlds(app: AppHandle, id: String) -> Result<Vec<crate::worlds::WorldInfo>> {
    crate::worlds::list(&app, &id)
}

#[tauri::command]
pub fn backend_base() -> String {
    crate::http::backend_base().unwrap_or_default()
}

#[tauri::command]
pub async fn get_news(force: bool) -> Result<Vec<crate::news::NewsItem>> {
    crate::news::get(force).await
}

/// The player's Minecraft friends list (Mojang API, via the account token).
#[tauri::command]
pub async fn get_friends(app: AppHandle) -> Result<crate::friends::FriendsList> {
    crate::friends::list(&app).await
}

/// Add/accept (`add = true`) or remove/decline (`add = false`) a friend by
/// name or profile id. Returns the updated list.
#[tauri::command]
pub async fn friend_update(
    app: AppHandle,
    name: Option<String>,
    profile_id: Option<String>,
    add: bool,
) -> Result<crate::friends::FriendsList> {
    crate::friends::update(&app, name, profile_id, add).await
}

/// Read the account's friends feature settings (enabled / accept invites).
#[tauri::command]
pub async fn get_friend_prefs(app: AppHandle) -> Result<crate::friends::FriendsPrefs> {
    crate::friends::get_prefs(&app).await
}

/// Turn the friends feature and invite-acceptance on/off for the account.
#[tauri::command]
pub async fn set_friend_prefs(
    app: AppHandle,
    friends_enabled: bool,
    accept_invites: bool,
) -> Result<crate::friends::FriendsPrefs> {
    crate::friends::set_prefs(&app, friends_enabled, accept_invites).await
}

#[tauri::command]
pub async fn get_achievements(
    app: AppHandle,
) -> Result<crate::achievements::AchievementsPayload> {
    tauri::async_runtime::spawn_blocking(move || crate::achievements::compute(&app))
        .await
        .map_err(|err| AppError::Other(format!("achievements scan failed: {err}")))?
}

#[tauri::command]
pub fn backup_world(app: AppHandle, id: String, folder: String) -> Result<String> {
    crate::worlds::backup(&app, &id, &folder)
}

#[tauri::command]
pub fn delete_world(app: AppHandle, id: String, folder: String) -> Result<()> {
    crate::worlds::delete(&app, &id, &folder)
}

/// Best-effort primary LAN IPv4 of this machine (for sharing a server address).
/// Opens a UDP socket toward a public IP to learn the outbound interface — no
/// packets are actually sent. Returns None if it can't be determined.
#[tauri::command]
pub fn get_local_ip() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|addr| addr.ip().to_string())
}

#[tauri::command]
pub fn read_ops(app: AppHandle, id: String) -> Result<Vec<crate::players::OpEntry>> {
    crate::players::read_ops(&app, &id)
}

#[tauri::command]
pub fn read_whitelist(app: AppHandle, id: String) -> Result<Vec<crate::players::PlayerEntry>> {
    crate::players::read_whitelist(&app, &id)
}

#[tauri::command]
pub async fn add_op(app: AppHandle, id: String, name: String, level: u8) -> Result<()> {
    crate::players::add_op(&app, &id, &name, level).await
}

#[tauri::command]
pub fn remove_op(app: AppHandle, id: String, name: String) -> Result<()> {
    crate::players::remove_op(&app, &id, &name)
}

#[tauri::command]
pub async fn add_whitelist(app: AppHandle, id: String, name: String) -> Result<()> {
    crate::players::add_whitelist(&app, &id, &name).await
}

#[tauri::command]
pub fn remove_whitelist(app: AppHandle, id: String, name: String) -> Result<()> {
    crate::players::remove_whitelist(&app, &id, &name)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JavaSetupEvent {
    label: String,
    current: usize,
    total: usize,
}

#[tauri::command]
pub async fn setup_java(app: AppHandle) -> Result<Vec<String>> {
    let client = crate::http::client()?;

    let app_cb = app.clone();
    let installed = launch::java::setup_common(&app, &client, move |label, current, total| {
        let _ = app_cb.emit(
            "java-setup",
            JavaSetupEvent {
                label: label.to_string(),
                current,
                total,
            },
        );
    })
    .await?;

    Ok(installed)
}

#[tauri::command]
pub fn resolved_java_paths(app: AppHandle) -> std::collections::HashMap<u32, String> {
    let mut out = std::collections::HashMap::new();
    for major in [8u32, 17, 21] {
        if let Some(path) = launch::java::managed_java_path(&app, major) {
            out.insert(major, path);
        }
    }
    out
}

#[tauri::command]
pub async fn login_microsoft(app: AppHandle) -> Result<AccountInfo> {
    auth::login(&app).await
}

/// Cancel an in-progress Microsoft device-code sign-in.
#[tauri::command]
pub fn cancel_login() {
    auth::cancel_login();
}

#[tauri::command]
pub fn get_accounts(store: State<'_, AccountStore>) -> AccountsState {
    store.state()
}

#[tauri::command]
pub fn set_active_account(
    app: AppHandle,
    store: State<'_, AccountStore>,
    id: Option<String>,
) -> Result<()> {
    store.set_active(&app, id)
}

#[tauri::command]
pub fn remove_account(
    app: AppHandle,
    store: State<'_, AccountStore>,
    id: String,
) -> Result<()> {
    store.remove(&app, &id)
}

#[tauri::command]
pub fn list_sources() -> Vec<SourceInfo> {
    sources::available()
}

#[tauri::command]
pub async fn search_content(source: Source, params: SearchParams) -> Result<SearchResults> {
    sources::search(source, params).await
}

#[tauri::command]
pub async fn get_content_categories() -> Result<Vec<crate::modrinth::Category>> {
    crate::modrinth::get_categories().await
}

#[tauri::command]
pub async fn content_versions(
    source: Source,
    project_id: String,
    loader: Option<String>,
    game_version: Option<String>,
) -> Result<Vec<ModrinthVersion>> {
    sources::get_versions(source, &project_id, loader.as_deref(), game_version.as_deref()).await
}

#[tauri::command]
pub async fn install_content(
    app: AppHandle,
    instance_id: String,
    source: Source,
    version_id: String,
    project_type: String,
    title: String,
    icon_url: Option<String>,
) -> Result<ContentItem> {
    content::install(&app, &instance_id, source, &version_id, &project_type, &title, icon_url).await
}

#[tauri::command]
pub fn list_content(app: AppHandle, instance_id: String) -> Result<Vec<ContentItem>> {
    content::list(&app, &instance_id)
}

#[tauri::command]
pub fn set_content_enabled(
    app: AppHandle,
    instance_id: String,
    version_id: String,
    enabled: bool,
) -> Result<()> {
    content::set_enabled(&app, &instance_id, &version_id, enabled)
}

#[tauri::command]
pub fn remove_content(app: AppHandle, instance_id: String, version_id: String) -> Result<()> {
    content::remove(&app, &instance_id, &version_id)
}

// --- Content updates + restore points ------------------------------------

#[tauri::command]
pub async fn check_content_updates(
    app: AppHandle,
    instance_id: String,
) -> Result<Vec<crate::restore::ContentUpdate>> {
    crate::restore::check_updates(&app, &instance_id).await
}

#[tauri::command]
pub async fn apply_content_updates(
    app: AppHandle,
    instance_id: String,
    updates: Vec<crate::restore::ContentUpdate>,
) -> Result<crate::restore::ApplyResult> {
    crate::restore::apply_updates(&app, &instance_id, updates).await
}

#[tauri::command]
pub fn list_restore_points(
    app: AppHandle,
    instance_id: String,
) -> Result<Vec<crate::restore::RestorePoint>> {
    crate::restore::list(&app, &instance_id)
}

#[tauri::command]
pub fn create_restore_point(
    app: AppHandle,
    instance_id: String,
    label: String,
) -> Result<crate::restore::RestorePoint> {
    crate::restore::create(&app, &instance_id, &label, false)
}

#[tauri::command]
pub async fn restore_instance(app: AppHandle, instance_id: String, id: String) -> Result<()> {
    crate::restore::restore(&app, &instance_id, &id).await
}

#[tauri::command]
pub fn delete_restore_point(app: AppHandle, instance_id: String, id: String) -> Result<()> {
    crate::restore::delete(&app, &instance_id, &id)
}

#[tauri::command]
pub async fn install_modpack(
    app: AppHandle,
    source: Source,
    version_id: String,
    icon_url: Option<String>,
) -> Result<Instance> {
    match source {
        Source::Modrinth => content::install_modpack(&app, &version_id, icon_url).await,
        Source::CurseForge => content::install_cf_modpack(&app, &version_id, icon_url).await,
    }
}

#[tauri::command]
pub async fn search_stickers(
    settings: State<'_, SettingsStore>,
    query: String,
    offset: u32,
) -> Result<Vec<crate::stickers::Sticker>> {
    let key = settings.get().giphy_api_key;
    crate::stickers::search(&key, &query, offset).await
}

#[tauri::command]
pub async fn download_image(url: String) -> Result<String> {
    crate::stickers::download_data_uri(&url).await
}

#[tauri::command]
pub fn content_cache_stats(app: AppHandle) -> Result<content::CacheStats> {
    content::cache::stats(&app)
}

/// Empty the shared content cache. Instances keep their own copies (the cache
/// blobs are extra hard links), so this is safe — only future installs re-fetch.
#[tauri::command]
pub fn clear_content_cache(app: AppHandle) -> Result<content::CacheStats> {
    let dir = crate::paths::content_cache_dir(&app)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    crate::paths::content_cache_dir(&app)?; // recreate empty
    content::cache::stats(&app)
}

/// Factory reset: delete every instance, all shared downloads (meta/), and
/// settings, then reset in-memory state. The frontend should reload afterwards.
/// Instances stored in a custom folder outside the app data dir aren't removed.
#[tauri::command]
pub fn reset_app_data(
    app: AppHandle,
    instances: State<'_, InstanceStore>,
    settings: State<'_, SettingsStore>,
) -> Result<()> {
    for dir in [crate::paths::instances_dir(&app)?, crate::paths::meta_dir(&app)?] {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
    }
    let settings_file = crate::paths::settings_file(&app)?;
    if settings_file.exists() {
        std::fs::remove_file(&settings_file)?;
    }
    settings.save(&app, Settings::default())?;
    instances.load(&app)?;
    Ok(())
}

#[tauri::command]
pub fn get_data_dir(app: AppHandle) -> Result<String> {
    Ok(crate::paths::data_dir(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn set_data_dir(
    app: AppHandle,
    instances: State<'_, InstanceStore>,
    settings: State<'_, SettingsStore>,
    path: Option<String>,
) -> Result<()> {
    let old = crate::paths::data_dir(&app)?;
    let default = crate::paths::default_data_dir(&app)?;
    let target = match path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => std::path::PathBuf::from(p),
        None => default.clone(),
    };
    if old == target {
        return Ok(());
    }
    if target.starts_with(&old) {
        return Err(AppError::Other(
            "Choose a folder outside the current data folder.".into(),
        ));
    }

    std::fs::create_dir_all(&target)?;
    for entry in std::fs::read_dir(&old)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "data-location.txt" {
            continue; // the pointer stays in the default dir
        }
        move_entry(&entry.path(), &target.join(&name))?;
    }

    let location = if target == default {
        String::new()
    } else {
        target.to_string_lossy().into_owned()
    };
    crate::paths::set_data_location(&app, &location)?;

    settings.load(&app)?;
    instances.load(&app)?;
    Ok(())
}

fn move_entry(src: &std::path::Path, dest: &std::path::Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    if src.is_dir() {
        std::fs::create_dir_all(dest)?;
        copy_tree(src, dest)?;
        std::fs::remove_dir_all(src)?;
    } else {
        std::fs::copy(src, dest)?;
        std::fs::remove_file(src)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn export_setup(
    app: AppHandle,
    instance_id: String,
    format: String,
    note: Option<String>,
) -> Result<crate::snapshot::ExportResult> {
    crate::snapshot::export(&app, &instance_id, &format, note).await
}

#[tauri::command]
pub async fn import_setup(
    app: AppHandle,
    bytes: Vec<u8>,
) -> Result<crate::snapshot::ImportResult> {
    crate::snapshot::import(&app, bytes).await
}

#[tauri::command]
pub async fn board_login(
    app: AppHandle,
    api_base: String,
) -> Result<crate::board_auth::BoardSession> {
    crate::board_auth::login(&app, &api_base).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn publish_setup(
    app: AppHandle,
    instance_id: String,
    format: String,
    api_base: String,
    access_token: String,
    board_handle: Option<String>,
    name: Option<String>,
    changelog: Option<String>,
) -> Result<String> {
    crate::snapshot::publish(
        &app,
        &instance_id,
        &format,
        &api_base,
        &access_token,
        board_handle,
        name,
        changelog,
    )
    .await
}

/// Whether an instance can be shared by code — i.e. every CurseForge item can
/// still be re-downloaded. Opt-out mods (no distributable file) can't, so a
/// shared code would import broken; those titles are returned.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareCheck {
    ok: bool,
    opt_out: Vec<String>,
}

#[tauri::command]
pub async fn instance_share_check(app: AppHandle, instance_id: String) -> Result<ShareCheck> {
    let mut opt_out = Vec::new();
    for item in content::list(&app, &instance_id)? {
        // Only CurseForge content can be distribution-opted-out.
        if item.source != "curseforge" {
            continue;
        }
        if let Ok(version) = sources::get_version(Source::CurseForge, &item.version_id).await {
            let downloadable = version
                .primary_file()
                .map(|file| !file.url.is_empty())
                .unwrap_or(false);
            if !downloadable {
                opt_out.push(item.title);
            }
        }
    }
    Ok(ShareCheck {
        ok: opt_out.is_empty(),
        opt_out,
    })
}

/// Change the active Microsoft account's Minecraft skin. `variant` is
/// "classic" or "slim". `bytes` is a 64×64 (or 64×32) PNG skin.
#[tauri::command]
pub async fn set_skin(app: AppHandle, bytes: Vec<u8>, variant: String) -> Result<()> {
    let client = crate::http::client()?;
    let variant = if variant == "slim" { "slim" } else { "classic" };

    let mut token = crate::auth::active_valid_account(&app, &client)
        .await?
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?
        .mc_access_token;

    for attempt in 0..2 {
        let form = reqwest::multipart::Form::new()
            .text("variant", variant.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(bytes.clone())
                    .file_name("skin.png")
                    .mime_str("image/png")?,
            );
        let resp = client
            .post("https://api.minecraftservices.com/minecraft/profile/skins")
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(());
        }
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
            if let Some(account) = app.state::<AccountStore>().active_account() {
                token = crate::auth::refresh_account(&app, &client, &account)
                    .await?
                    .mc_access_token;
                continue;
            }
        }
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("couldn't set skin ({status}): {text}")));
    }
    Ok(())
}

/// Reset the active Microsoft account's skin back to the default (Steve/Alex).
#[tauri::command]
pub async fn reset_skin(app: AppHandle) -> Result<()> {
    let client = crate::http::client()?;
    let url = "https://api.minecraftservices.com/minecraft/profile/skins/active";
    let mut token = crate::auth::active_valid_account(&app, &client)
        .await?
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?
        .mc_access_token;

    for attempt in 0..2 {
        let resp = client.delete(url).bearer_auth(&token).send().await?;
        if resp.status().is_success() {
            return Ok(());
        }
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
            if let Some(account) = app.state::<AccountStore>().active_account() {
                token = crate::auth::refresh_account(&app, &client, &account)
                    .await?
                    .mc_access_token;
                continue;
            }
        }
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("couldn't reset skin ({status}): {text}")));
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    id: String,
    alias: String,
    url: String,
    active: bool,
}

#[tauri::command]
pub async fn get_capes(app: AppHandle) -> Result<Vec<Cape>> {
    let client = crate::http::client()?;
    let account = crate::auth::active_valid_account(&app, &client)
        .await?
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?;
    let profile: serde_json::Value = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&account.mc_access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let capes = profile
        .get("capes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let out = capes
        .iter()
        .filter_map(|cape| {
            Some(Cape {
                id: cape.get("id")?.as_str()?.to_string(),
                alias: cape
                    .get("alias")
                    .and_then(|alias| alias.as_str())
                    .unwrap_or("Cape")
                    .to_string(),
                url: cape.get("url")?.as_str()?.to_string(),
                active: cape.get("state").and_then(|state| state.as_str()) == Some("ACTIVE"),
            })
        })
        .collect();
    Ok(out)
}

#[tauri::command]
pub async fn set_cape(app: AppHandle, cape_id: Option<String>) -> Result<()> {
    let client = crate::http::client()?;
    let account = crate::auth::active_valid_account(&app, &client)
        .await?
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?;
    let url = "https://api.minecraftservices.com/minecraft/profile/capes/active";
    let resp = match cape_id {
        Some(id) => {
            client
                .put(url)
                .bearer_auth(&account.mc_access_token)
                .json(&serde_json::json!({ "capeId": id }))
                .send()
                .await?
        }
        None => client.delete(url).bearer_auth(&account.mc_access_token).send().await?,
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("couldn't set cape ({status}): {text}")));
    }
    Ok(())
}

/// Inspect the machine + instance and return a tailored performance plan.
/// `mode` is "performance" (default) or "visuals" (adds shader support).
#[tauri::command]
pub async fn tuneup_recommend(
    app: AppHandle,
    instance_id: String,
    mode: Option<String>,
) -> Result<crate::tuneup::TuneupPlan> {
    crate::tuneup::recommend(&app, &instance_id, mode.as_deref().unwrap_or("performance")).await
}

#[tauri::command]
pub async fn tuneup_apply(
    app: AppHandle,
    instance_id: String,
    selection: crate::tuneup::TuneupSelection,
) -> Result<usize> {
    crate::tuneup::apply(&app, &instance_id, selection).await
}

#[tauri::command]
pub fn add_server_to_instance(
    app: AppHandle,
    instance_id: String,
    name: String,
    address: String,
) -> Result<()> {
    let game_dir = crate::paths::instance_game_dir(&app, &instance_id)?;
    crate::servers_dat::add_server(&game_dir.join("servers.dat"), &name, &address)
}
