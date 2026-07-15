use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::auth::{self, AccountInfo, AccountStore, AccountsState};
use crate::content::{self, ContentItem};
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::{CreateInstance, Instance, UpdateInstance};
use crate::instance::ModLoader;
use crate::launch::{self, LaunchState};
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
        payload.mc_version,
        payload.loader,
        payload.loader_version,
        payload.icon,
    );
    store.save(&app, &instance)?;
    Ok(instance)
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
        instance.icon = Some(icon);
    }
    if let Some(group) = patch.group {
        instance.group = Some(group);
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
    launch::launch(app, instance, settings).await
}

#[tauri::command]
pub fn stop_instance(state: State<'_, LaunchState>, id: String) -> Result<()> {
    state.kill(&id);
    Ok(())
}

#[tauri::command]
pub fn is_instance_running(state: State<'_, LaunchState>, id: String) -> bool {
    state.is_running(&id)
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
        .user_agent(concat!("rust-mc-launcher/", env!("CARGO_PKG_VERSION")))
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
    version_id: String,
    icon_url: Option<String>,
) -> Result<Instance> {
    content::install_modpack(&app, &version_id, icon_url).await
}
