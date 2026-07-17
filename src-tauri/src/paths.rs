use crate::error::Result;
use crate::instance::store::InstanceStore;
use crate::settings::SettingsStore;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Root application data directory (e.g. `~/Library/Application Support/<id>` on macOS).
pub fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory holding one subfolder per instance. Each instance folder contains
/// an `instance.json` metadata file plus (eventually) its game files.
pub fn instances_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = data_dir(app)?.join("instances");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Folder for a single instance.
pub fn instance_dir(app: &AppHandle, id: &str) -> Result<PathBuf> {
    Ok(instances_dir(app)?.join(id))
}

/// Shared directory for downloaded assets, libraries, and version metadata
/// (reused across instances to save disk space).
pub fn meta_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = data_dir(app)?.join("meta");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/versions/<id>/` — per-version JSON and client jar.
pub fn version_dir(app: &AppHandle, version_id: &str) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("versions").join(version_id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/libraries/` — maven-style library tree (shared across instances).
pub fn libraries_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("libraries");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/assets/` — object store + indexes (shared across instances).
pub fn assets_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("assets");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/content-cache/` — content-addressed store of downloaded mods,
/// resource packs and shaders, shared (hard-linked) across all instances so
/// identical files are stored only once.
pub fn content_cache_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("content-cache");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/java/` — managed Java runtimes (one folder per component).
pub fn java_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("java");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Per-instance extracted natives (specific to the exact version/OS).
pub fn natives_dir(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    let dir = instance_dir(app, instance_id)?.join("natives");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The default game directory under the record folder (used when an instance
/// has no custom `game_dir`).
pub fn default_game_dir(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    Ok(instance_dir(app, instance_id)?.join("minecraft"))
}

/// The game working directory for an instance (its `.minecraft` equivalent):
/// saves, mods, config, resourcepacks live here. Honours the instance's custom
/// `game_dir` when set, otherwise falls back to the default under its folder.
pub fn instance_game_dir(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    let custom = app
        .state::<InstanceStore>()
        .get(instance_id)
        .and_then(|instance| instance.game_dir)
        .filter(|path| !path.trim().is_empty());
    let dir = match custom {
        Some(path) => PathBuf::from(path),
        None => default_game_dir(app, instance_id)?,
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The game directory a NEW instance should use, honouring the global
/// instances-folder setting. Resolved once at creation and stored on the
/// instance, so a later change to the global setting never moves it.
/// Returns `None` when the default location should be used (empty setting).
pub fn new_instance_game_dir(app: &AppHandle, instance_id: &str) -> Option<String> {
    let base = app.state::<SettingsStore>().get().instances_dir;
    let base = base.trim();
    if base.is_empty() {
        return None;
    }
    Some(
        PathBuf::from(base)
            .join(instance_id)
            .join("minecraft")
            .to_string_lossy()
            .into_owned(),
    )
}

/// Global settings file.
pub fn settings_file(app: &AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("settings.json"))
}
