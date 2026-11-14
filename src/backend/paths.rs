use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::settings::SettingsStore;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::host::{Host};

/// Accept a repo/archive-relative path only if it stays inside its base — every
/// component must be `Normal` or `.` (so `..`, absolute paths, Windows drive
/// prefixes, and drive-relative roots like `\Windows` are all refused). Returns
/// the validated path. The single guard behind archive extraction (modpacks,
/// snapshots, restore points) and maven library layout.
pub fn safe_relative_path(path: &str) -> Option<PathBuf> {
    let path_buf = PathBuf::from(path);
    let safe = !path_buf.as_os_str().is_empty()
        && path_buf
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    safe.then_some(path_buf)
}

static WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Replace a file atomically: write a sibling temp file, then `rename` it over
/// the target (an atomic swap, so a crash never leaves a half-written file).
/// `private` creates the temp `0600` on Unix; `durable` `fsync`s before the
/// rename (survives power loss, at some latency cost).
fn atomic_write(path: &Path, contents: &[u8], private: bool, durable: bool) -> Result<()> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| AppError::Other("cannot write a file with no parent directory".into()))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("state");
    let seq = WRITE_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{name}.tmp.{}.{seq}", std::process::id()));
    // A stale temp (from a crashed run reusing this pid) could have the wrong
    // mode, so start from a clean create.
    let _ = std::fs::remove_file(&tmp);

    let written = (|| -> Result<()> {
        use std::io::Write;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        #[cfg(not(unix))]
        let _ = private;
        let mut file = opts.open(&tmp)?;
        file.write_all(contents)?;
        if durable {
            file.sync_all()?;
        }
        Ok(())
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return written;
    }

    if let Err(error) = rename_replace(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error.into());
    }
    if durable {
        let _ = fsync_dir(dir);
    }
    Ok(())
}

/// `rename` over an existing file, retrying briefly — on Windows a file that
/// antivirus or another process has momentarily open can't be replaced yet.
fn rename_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last = None;
    for attempt in 0..5u32 {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last = Some(error);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(20 * (attempt as u64 + 1)));
                }
            }
        }
    }
    Err(last.expect("loop runs at least once"))
}

#[cfg(unix)]
fn fsync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}
#[cfg(not(unix))]
fn fsync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Atomically write a file that may contain secrets, owner-only (0600) on Unix
/// and `fsync`ed for durability. NOTE: on Windows no explicit ACL is applied yet
/// — confidentiality there rests on the per-user `%APPDATA%` location; a proper
/// owner-only DACL (Win32 `SetFileSecurity`) is still TODO.
pub fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    atomic_write(path, contents, true, true)
}

/// Atomically replace a (non-secret) state file — corruption-safe on a crash,
/// without the `fsync` cost, for frequently-written files.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    atomic_write(path, contents, false, false)
}

/// The OS default app data dir (e.g. `~/Library/Application Support/<id>`). The
/// data-location pointer always lives here, even when data is redirected.
pub fn default_data_dir(app: &Host) -> Result<PathBuf> {
    let dir = app.app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn location_pointer(app: &Host) -> Result<PathBuf> {
    Ok(default_data_dir(app)?.join("data-location.txt"))
}

/// Root application data directory. Honours a user-chosen location (see
/// `set_data_location`), falling back to the OS default when unset or when the
/// chosen location is currently unavailable (e.g. an unplugged drive).
pub fn data_dir(app: &Host) -> Result<PathBuf> {
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
pub fn set_data_location(app: &Host, dir: &str) -> Result<()> {
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

pub fn instances_dir(app: &Host) -> Result<PathBuf> {
    let dir = data_dir(app)?.join("instances");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Reject an instance id that isn't a single safe path component. IDs are joined
/// onto the instances dir; a crafted id (`../…`, an absolute path, a separator)
/// from a compromised webview would otherwise let instance-scoped commands read
/// or write outside the data dir. Real ids are generated UUIDs, so a legitimate
/// instance is never rejected.
pub fn validate_instance_id(id: &str) -> Result<()> {
    let safe = !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && std::path::Path::new(id).file_name().and_then(|name| name.to_str()) == Some(id);
    if safe {
        Ok(())
    } else {
        Err(AppError::Other(format!("invalid instance id: {id}")))
    }
}

pub fn instance_dir(app: &Host, id: &str) -> Result<PathBuf> {
    validate_instance_id(id)?;
    Ok(instances_dir(app)?.join(id))
}

pub fn meta_dir(app: &Host) -> Result<PathBuf> {
    let dir = data_dir(app)?.join("meta");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn version_dir(app: &Host, version_id: &str) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("versions").join(version_id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn libraries_dir(app: &Host) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("libraries");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn assets_dir(app: &Host) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("assets");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/content-cache/` — content-addressed store of downloaded mods,
/// resource packs and shaders, shared (hard-linked) across all instances so
/// identical files are stored only once.
pub fn content_cache_dir(app: &Host) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("content-cache");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn java_dir(app: &Host) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("java");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// `meta/log-configs/` — Mojang's per-version log4j configuration files (the
/// Log4Shell mitigation), shared across instances by their manifest id.
pub fn log_configs_dir(app: &Host) -> Result<PathBuf> {
    let dir = meta_dir(app)?.join("log-configs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn natives_dir(app: &Host, instance_id: &str) -> Result<PathBuf> {
    let dir = instance_dir(app, instance_id)?.join("natives");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn default_game_dir(app: &Host, instance_id: &str) -> Result<PathBuf> {
    Ok(instance_dir(app, instance_id)?.join("minecraft"))
}

pub fn instance_game_dir(app: &Host, instance_id: &str) -> Result<PathBuf> {
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
pub fn new_instance_game_dir(app: &Host, instance_id: &str) -> Option<String> {
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

pub fn settings_file(app: &Host) -> Result<PathBuf> {
    Ok(data_dir(app)?.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::{safe_relative_path, validate_instance_id};

    #[test]
    fn safe_rel_accepts_relative_rejects_escapes() {
        assert!(safe_relative_path("mods/cool.jar").is_some());
        assert!(safe_relative_path("config/sub/a.toml").is_some());
        assert!(safe_relative_path("").is_none());
        assert!(safe_relative_path("../evil").is_none());
        assert!(safe_relative_path("a/../../evil").is_none());
        assert!(safe_relative_path("/etc/passwd").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn safe_rel_rejects_windows_drive_relative_root() {
        // Not is_absolute() on Windows (no drive), but escapes to the drive root.
        assert!(safe_relative_path(r"\Windows\System32\x").is_none());
        assert!(safe_relative_path(r"C:\Windows").is_none());
    }

    #[test]
    fn validate_instance_id_accepts_uuids_rejects_traversal() {
        assert!(validate_instance_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_instance_id("").is_err());
        assert!(validate_instance_id("..").is_err());
        assert!(validate_instance_id("../../etc").is_err());
        assert!(validate_instance_id("a/b").is_err());
        assert!(validate_instance_id(r"a\b").is_err());
        assert!(validate_instance_id("/abs").is_err());
    }
}
