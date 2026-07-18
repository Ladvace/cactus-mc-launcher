//! World (save) discovery, backup and deletion for an instance. Works for both
//! client saves (`<game>/saves/*`) and server worlds (`<game>/<level-name>`).

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::AppHandle;

use crate::error::{AppError, Result};
use crate::paths;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    /// Display name (the folder name).
    pub name: String,
    /// Path relative to the instance game dir (e.g. "world" or "saves/My World").
    pub folder: String,
    /// Absolute path (for revealing in the file manager).
    pub path: String,
    pub size_bytes: u64,
    /// RFC3339 timestamp of the last save (from `level.dat`), if available.
    pub last_modified: Option<String>,
    /// "saves" (client singleplayer) or "server" (root-level world).
    pub location: String,
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(file_type) if file_type.is_dir() => total += dir_size(&entry.path()),
                Ok(file_type) if file_type.is_file() => {
                    total += entry.metadata().map(|meta| meta.len()).unwrap_or(0)
                }
                _ => {}
            }
        }
    }
    total
}

fn make_info(game: &Path, dir: &Path, location: &str) -> WorldInfo {
    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder = dir
        .strip_prefix(game)
        .unwrap_or(dir)
        .to_string_lossy()
        .to_string();
    let last_modified = std::fs::metadata(dir.join("level.dat"))
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339());
    WorldInfo {
        name,
        folder,
        path: dir.to_string_lossy().to_string(),
        size_bytes: dir_size(dir),
        last_modified,
        location: location.to_string(),
    }
}

/// A folder holds a world if it contains a `level.dat`.
fn is_world(dir: &Path) -> bool {
    dir.is_dir() && dir.join("level.dat").exists()
}

pub fn list(app: &AppHandle, id: &str) -> Result<Vec<WorldInfo>> {
    let game = paths::instance_game_dir(app, id)?;
    let mut out = Vec::new();

    // Client singleplayer saves.
    let saves = game.join("saves");
    if saves.is_dir() {
        for entry in std::fs::read_dir(&saves)?.flatten() {
            if is_world(&entry.path()) {
                out.push(make_info(&game, &entry.path(), "saves"));
            }
        }
    }

    // Server worlds live at the game-dir root (world, world_nether, …).
    for entry in std::fs::read_dir(&game)?.flatten() {
        let path = entry.path();
        if entry.file_name() == "saves" {
            continue;
        }
        if is_world(&path) {
            out.push(make_info(&game, &path, "server"));
        }
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// Resolve and validate a world folder inside the instance's game dir.
fn resolve_world(game: &Path, folder: &str) -> Result<PathBuf> {
    if folder.is_empty() || folder.contains("..") {
        return Err(AppError::Other("invalid world folder".into()));
    }
    let dir = game.join(folder);
    if !is_world(&dir) {
        return Err(AppError::Other("that folder isn't a world".into()));
    }
    // Defence in depth: ensure the resolved path stays within the game dir.
    let canon = dir.canonicalize()?;
    let game_canon = game.canonicalize()?;
    if !canon.starts_with(&game_canon) {
        return Err(AppError::Other("world is outside the instance".into()));
    }
    Ok(dir)
}

pub fn delete(app: &AppHandle, id: &str, folder: &str) -> Result<()> {
    let game = paths::instance_game_dir(app, id)?;
    let dir = resolve_world(&game, folder)?;
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Zip the world into `<instance>/backups/<name>-<timestamp>.zip`; returns the path.
pub fn backup(app: &AppHandle, id: &str, folder: &str) -> Result<String> {
    let game = paths::instance_game_dir(app, id)?;
    let dir = resolve_world(&game, folder)?;

    let backups = paths::instance_dir(app, id)?.join("backups");
    std::fs::create_dir_all(&backups)?;

    let base: String = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "world".into())
        .chars()
        .map(|ch| if ch.is_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' })
        .collect();
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let out = backups.join(format!("{base}-{stamp}.zip"));

    zip_dir(&dir, &out)?;
    Ok(out.to_string_lossy().to_string())
}

fn zip_dir(src: &Path, out: &Path) -> Result<()> {
    let file = std::fs::File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let root = src
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "world".into());
    add_dir(&mut zip, src, &root, opts)?;
    zip.finish()?;
    Ok(())
}

fn add_dir(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    rel_prefix: &str,
    opts: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(base)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = format!("{rel_prefix}/{name}");
        if path.is_dir() {
            add_dir(zip, &path, &rel, opts)?;
        } else if path.is_file() {
            zip.start_file(&rel, opts)?;
            let mut file = std::fs::File::open(&path)?;
            std::io::copy(&mut file, zip)?;
            let _ = zip.flush();
        }
    }
    Ok(())
}
