use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

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

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_instances(store: State<'_, InstanceStore>) -> Vec<Instance> {
    store.list()
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
        payload.name,
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

/// Create a dedicated-server instance from a client instance: same Minecraft
/// version and loader, with its mods and configs copied over.
#[tauri::command]
pub fn create_server_from(
    app: AppHandle,
    store: State<'_, InstanceStore>,
    id: String,
) -> Result<Instance> {
    let source = store.get(&id).ok_or_else(|| AppError::InstanceNotFound(id.clone()))?;

    let mut server = Instance::new(
        format!("{} (Server)", source.name),
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

/// The instance's game directory, for revealing it in the file manager.
#[tauri::command]
pub fn instance_folder(app: AppHandle, id: String) -> Result<String> {
    let dir = crate::paths::instance_game_dir(&app, &id)?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Move an instance's game data to `path` (or back to the default when null),
/// remembering the location. Refuses while the instance is running.
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
    let has_custom = path
        .as_deref()
        .map(|p| !p.trim().is_empty())
        .unwrap_or(false);
    let target = if has_custom {
        std::path::PathBuf::from(path.as_deref().unwrap().trim())
    } else {
        crate::paths::default_game_dir(&app, &id)?
    };

    if old != target {
        move_tree(&old, &target)?;
    }
    instance.game_dir = if has_custom {
        Some(target.to_string_lossy().into_owned())
    } else {
        None
    };
    store.save(&app, &instance)?;
    Ok(instance)
}

/// Move a directory tree, falling back to copy+remove across volumes / into an
/// existing target.
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
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&dest)?;
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Minecraft metadata
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_minecraft_versions() -> Result<VersionList> {
    minecraft::fetch_versions().await
}

/// List available loader builds for a Minecraft version (Fabric/Quilt).
#[tauri::command]
pub async fn get_loader_versions(
    loader: ModLoader,
    mc_version: String,
) -> Result<Vec<LoaderVersion>> {
    loader::list_versions(loader, &mc_version).await
}

// ---------------------------------------------------------------------------
// Launching
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    instances: State<'_, InstanceStore>,
    settings: State<'_, SettingsStore>,
    id: String,
) -> Result<()> {
    let instance = instances.get(&id).ok_or(AppError::InstanceNotFound(id))?;
    let settings = settings.get();
    match instance.kind {
        InstanceKind::Server => launch::server::launch(app, instance, settings).await,
        InstanceKind::Client => launch::launch(app, instance, settings).await,
    }
}

#[tauri::command]
pub fn stop_instance(
    launch: State<'_, LaunchState>,
    servers: State<'_, ServerState>,
    id: String,
) -> Result<()> {
    // Servers stop gracefully (issue `stop`); clients are killed.
    if !servers.stop(&id) {
        launch.kill(&id);
    }
    Ok(())
}

/// Send a line to a running server's console (stdin). No-op if not running.
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

// ---------------------------------------------------------------------------
// Server configuration (server.properties)
// ---------------------------------------------------------------------------

/// Read a server's `server.properties` (empty string if it doesn't exist yet).
#[tauri::command]
pub fn read_server_properties(app: AppHandle, id: String) -> Result<String> {
    let file = crate::paths::instance_game_dir(&app, &id)?.join("server.properties");
    Ok(std::fs::read_to_string(file).unwrap_or_default())
}

/// Overwrite a server's `server.properties`.
#[tauri::command]
pub fn write_server_properties(app: AppHandle, id: String, content: String) -> Result<()> {
    let file = crate::paths::instance_game_dir(&app, &id)?.join("server.properties");
    std::fs::write(file, content)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Worlds
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_worlds(app: AppHandle, id: String) -> Result<Vec<crate::worlds::WorldInfo>> {
    crate::worlds::list(&app, &id)
}

/// Zip a world into the instance's `backups/` folder; returns the zip path.
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

// ---------------------------------------------------------------------------
// Server players (ops / whitelist)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Java setup
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JavaSetupEvent {
    label: String,
    current: usize,
    total: usize,
}

/// Pre-install the common managed Java runtimes (8, 17, 21). Streams progress
/// via the `java-setup` event and returns the labels of installed runtimes.
#[tauri::command]
pub async fn setup_java(app: AppHandle) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("cactus-launcher/", env!("CARGO_PKG_VERSION")))
        .build()?;

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

// ---------------------------------------------------------------------------
// Accounts / Microsoft auth
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn login_microsoft(app: AppHandle) -> Result<AccountInfo> {
    auth::login(&app).await
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

// ---------------------------------------------------------------------------
// Modrinth content
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_sources() -> Vec<SourceInfo> {
    sources::available()
}

#[tauri::command]
pub async fn search_content(source: Source, params: SearchParams) -> Result<SearchResults> {
    sources::search(source, params).await
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

/// Install a Modrinth modpack version as a new instance (streams progress via
/// the `modpack-progress` event).
#[tauri::command]
pub async fn install_modpack(
    app: AppHandle,
    source: Source,
    version_id: String,
    icon_url: Option<String>,
) -> Result<Instance> {
    match source {
        Source::Modrinth => content::install_modpack(&app, &version_id, icon_url).await,
        Source::Ftb => content::install_ftb_modpack(&app, &version_id, icon_url).await,
        Source::CurseForge => Err(AppError::Other(
            "CurseForge modpack install isn't supported yet".into(),
        )),
    }
}

/// Search animated stickers (trending when the query is empty).
#[tauri::command]
pub async fn search_stickers(
    settings: State<'_, SettingsStore>,
    query: String,
    offset: u32,
) -> Result<Vec<crate::stickers::Sticker>> {
    let key = crate::stickers::effective_key(&settings.get().giphy_api_key);
    crate::stickers::search(&key, &query, offset).await
}

/// Whether the sticker picker is enabled — from the settings key or a baked-in
/// `.env` `GIPHY_API_KEY`. Lets the UI enable stickers without a settings key.
#[tauri::command]
pub fn giphy_configured(settings: State<'_, SettingsStore>) -> bool {
    crate::stickers::is_configured(&settings.get().giphy_api_key)
}

/// Download an image URL and return it as a data URI (used to store a chosen
/// sticker as an offline instance icon).
#[tauri::command]
pub async fn download_image(url: String) -> Result<String> {
    crate::stickers::download_data_uri(&url).await
}

/// Stats about the shared, deduplicated content cache (files, size, disk saved).
#[tauri::command]
pub fn content_cache_stats(app: AppHandle) -> Result<content::CacheStats> {
    content::cache::stats(&app)
}

// ---------------------------------------------------------------------------
// Snapshots (share / export-import)
// ---------------------------------------------------------------------------

/// Export an instance's full setup to a `.cactuspack` or `.mrpack` file.
#[tauri::command]
pub async fn export_setup(
    app: AppHandle,
    instance_id: String,
    format: String,
    note: Option<String>,
) -> Result<crate::snapshot::ExportResult> {
    crate::snapshot::export(&app, &instance_id, &format, note).await
}

/// Import a snapshot (raw `.cactuspack` bytes) as a new instance.
#[tauri::command]
pub async fn import_setup(
    app: AppHandle,
    bytes: Vec<u8>,
) -> Result<crate::snapshot::ImportResult> {
    crate::snapshot::import(&app, bytes).await
}

// ---------------------------------------------------------------------------
// Streamer service (sign-in + publish)
// ---------------------------------------------------------------------------

/// Sign in to the boards service using the player's Minecraft account
/// (Mojang hasJoined handshake). Returns a backend session token.
#[tauri::command]
pub async fn board_login(
    app: AppHandle,
    api_base: String,
) -> Result<crate::board_auth::BoardSession> {
    crate::board_auth::login(&app, &api_base).await
}

/// Export an instance and publish it — to a board you own (when `board_handle`
/// is set) or as a standalone shareable snapshot. Returns the snapshot id.
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

// ---------------------------------------------------------------------------
// Character skin
// ---------------------------------------------------------------------------

/// Change the active Microsoft account's Minecraft skin. `variant` is
/// "classic" or "slim". `bytes` is a 64×64 (or 64×32) PNG skin.
#[tauri::command]
pub async fn set_skin(
    app: AppHandle,
    store: State<'_, AccountStore>,
    bytes: Vec<u8>,
    variant: String,
) -> Result<()> {
    let account = store
        .active_account()
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?;
    let _ = &app;

    let variant = if variant == "slim" { "slim" } else { "classic" };
    let form = reqwest::multipart::Form::new()
        .text("variant", variant.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(bytes)
                .file_name("skin.png")
                .mime_str("image/png")?,
        );

    let resp = reqwest::Client::new()
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&account.mc_access_token)
        .multipart(form)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("couldn't set skin ({status}): {text}")));
    }
    Ok(())
}

/// A cape the account owns.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    id: String,
    alias: String,
    url: String,
    active: bool,
}

/// List the capes on the active Microsoft account.
#[tauri::command]
pub async fn get_capes(store: State<'_, AccountStore>) -> Result<Vec<Cape>> {
    let account = store
        .active_account()
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?;
    let profile: serde_json::Value = reqwest::Client::new()
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

/// Set the active cape (`Some(id)`) or hide it (`None`).
#[tauri::command]
pub async fn set_cape(store: State<'_, AccountStore>, cape_id: Option<String>) -> Result<()> {
    let account = store
        .active_account()
        .ok_or_else(|| AppError::Other("Add a Microsoft account first.".into()))?;
    let client = reqwest::Client::new();
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
