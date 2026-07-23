use crate::error::Result;
use crate::instance::store::InstanceStore;
use crate::settings::SettingsStore;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Write a file containing secrets (tokens, API keys) and restrict it to the
/// owner (`0600` on Unix). On Windows the per-user profile ACL already keeps it
/// private, so the mode is a no-op there.
pub fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    std::fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// The OS default app data dir (e.g. `~/Library/Application Support/<id>`). The
/// data-location pointer always lives here, even when data is redirected.
pub fn default_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn location_pointer(app: &AppHandle) -> Result<PathBuf> {
    Ok(default_data_dir(app)?.join("data-location.txt"))
}

/// Root application data directory. Honours a user-chosen location (see
/// `set_data_location`), falling back to the OS default when unset or when the
/// chosen location is currently unavailable (e.g. an unplugged drive).
pub fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    if let Ok(custom) = std::fs::read_to_string(location_pointer(app)?) {
        let custom = custom.trim();
        if !custom.is_empty() {
            let path = PathBuf::from(custom);
            if path.exists() {
                return Ok(path);
            }
        }
    }
    default_data_dir(app)
}

/// Redirect the data dir to `dir` (empty clears it back to the default).
pub fn set_data_location(app: &AppHandle, dir: &str) -> Result<()> {
    let pointer = location_pointer(app)?;
    if dir.trim().is_empty() {
        if pointer.exists() {
            std::fs::remove_file(pointer)?;
        }
    } else {
        std::fs::write(pointer, dir.trim())?;
    }
    Ok(())
}

pub fn instances_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = data_dir(app)?.join("instances");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn instance_dir(app: &AppHandle, id: &str) -> Result<PathBuf> {
    Ok(instances_dir(app)?.join(id))
}

pub fn meta_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = data_dir(app)?.join("meta");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn version_dir(app: &AppHandle, version_id: &str) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("versions").join(version_id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn libraries_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("libraries");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

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

pub fn java_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("java");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn natives_dir(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    let dir = instance_dir(app, instance_id)?.join("natives");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn default_game_dir(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    Ok(instance_dir(app, instance_id)?.join("minecraft"))
}

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

pub fn settings_file(app: &AppHandle) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("settings.json"))
}
