//! Per-instance restore points and safe content updates.
//!
//! A restore point is a small zip in `<instance>/backups/restore-<id>.zip`
//! holding the exact content refs (source/project/version/enabled) plus a copy
//! of the config overrides (`config/`, `options.txt`, …). Because the blob cache
//! is never evicted, restoring is essentially re-linking the recorded versions
//! rather than re-downloading. Updates auto-create one first, so a bad update is
//! always one click from being undone.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::content;
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::ModLoader;
use crate::paths;
use crate::sources::{self, Source};

/// How many *auto* restore points to keep per instance (manual ones are kept).
const KEEP_AUTO: usize = 10;
const MAX_OVERRIDE_FILE: u64 = 2_000_000;
const MAX_OVERRIDES_TOTAL: u64 = 12_000_000;

fn parse_source(text: &str) -> Source {
    match text {
        "curseforge" => Source::CurseForge,
        _ => Source::Modrinth,
    }
}

fn loader_str(loader: ModLoader) -> &'static str {
    match loader {
        ModLoader::Vanilla => "vanilla",
        ModLoader::Fabric => "fabric",
        ModLoader::Quilt => "quilt",
        ModLoader::Forge => "forge",
        ModLoader::NeoForge => "neoforge",
    }
}

// ---------------------------------------------------------------------------
// Restore points
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreContent {
    source: String,
    project_type: String,
    project_id: Option<String>,
    version_id: String,
    file_name: String,
    title: String,
    icon_url: Option<String>,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePoint {
    pub id: String,
    pub created: String,
    pub label: String,
    pub auto: bool,
    pub content_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredPoint {
    #[serde(flatten)]
    meta: RestorePoint,
    content: Vec<RestoreContent>,
}

fn backups_dir(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    let dir = paths::instance_dir(app, instance_id)?.join("backups");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn point_path(app: &AppHandle, instance_id: &str, id: &str) -> Result<PathBuf> {
    // `id` is launcher-generated (timestamp), but validate it can't traverse.
    if id.is_empty() || id.contains(['/', '\\', '.']) {
        return Err(AppError::Other("invalid restore point id".into()));
    }
    Ok(backups_dir(app, instance_id)?.join(format!("restore-{id}.zip")))
}

/// Snapshot the instance's current content + config into a new restore point.
pub fn create(app: &AppHandle, instance_id: &str, label: &str, auto: bool) -> Result<RestorePoint> {
    let items = content::list(app, instance_id)?;
    let content: Vec<RestoreContent> = items
        .iter()
        .map(|item| RestoreContent {
            source: item.source.clone(),
            project_type: item.project_type.clone(),
            project_id: item.project_id.clone(),
            version_id: item.version_id.clone(),
            file_name: item.file_name.clone(),
            title: item.title.clone(),
            icon_url: item.icon_url.clone(),
            enabled: item.enabled,
        })
        .collect();

    let now = chrono::Utc::now();
    // Timestamp id, made unique in the (rare) same-millisecond case.
    let base_id = now.format("%Y%m%d-%H%M%S-%3f").to_string();
    let mut id = base_id.clone();
    let mut n = 1;
    while point_path(app, instance_id, &id)?.exists() {
        id = format!("{base_id}-{n}");
        n += 1;
    }
    let meta = RestorePoint {
        id: id.clone(),
        created: now.to_rfc3339(),
        label: label.to_string(),
        auto,
        content_count: content.len(),
    };
    let stored = StoredPoint {
        meta: meta.clone(),
        content,
    };

    let game_dir = paths::instance_game_dir(app, instance_id)?;
    write_point(&point_path(app, instance_id, &id)?, &stored, &game_dir)?;

    if auto {
        prune_auto(app, instance_id)?;
    }
    Ok(meta)
}

fn write_point(path: &Path, stored: &StoredPoint, game_dir: &Path) -> Result<()> {
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path)?);
    zip.start_file("restore.json", opts)?;
    zip.write_all(serde_json::to_string(stored)?.as_bytes())?;

    // Bundle the config overrides (size-capped, like the snapshot export).
    let mut total: u64 = 0;
    for rel in override_files(game_dir) {
        let full = game_dir.join(&rel);
        let Ok(meta) = std::fs::metadata(&full) else {
            continue;
        };
        if meta.len() > MAX_OVERRIDE_FILE {
            continue;
        }
        if total + meta.len() > MAX_OVERRIDES_TOTAL {
            break;
        }
        let Ok(bytes) = std::fs::read(&full) else {
            continue;
        };
        total += bytes.len() as u64;
        let name = format!("overrides/{}", rel.to_string_lossy().replace('\\', "/"));
        zip.start_file(name, opts)?;
        zip.write_all(&bytes)?;
    }
    zip.finish()?;
    Ok(())
}

fn override_files(game_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in ["options.txt", "optionsshaders.txt", "servers.dat"] {
        if game_dir.join(name).is_file() {
            out.push(PathBuf::from(name));
        }
    }
    collect_rel(&game_dir.join("config"), game_dir, &mut out);
    out
}

fn collect_rel(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rel(&path, base, out);
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_path_buf());
        }
    }
}

/// All restore points for an instance, newest first (metadata only).
pub fn list(app: &AppHandle, instance_id: &str) -> Result<Vec<RestorePoint>> {
    let dir = backups_dir(app, instance_id)?;
    let mut points = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with("restore-") && name.ends_with(".zip")) {
            continue;
        }
        if let Ok(stored) = read_point(&entry.path()) {
            points.push(stored.meta);
        }
    }
    points.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(points)
}

fn read_point(path: &Path) -> Result<StoredPoint> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut text = String::new();
    zip.by_name("restore.json")
        .map_err(|_| AppError::Other("restore point is missing its index".into()))?
        .read_to_string(&mut text)?;
    serde_json::from_str(&text)
        .map_err(|error| AppError::Other(format!("bad restore point: {error}")))
}

pub fn delete(app: &AppHandle, instance_id: &str, id: &str) -> Result<()> {
    std::fs::remove_file(point_path(app, instance_id, id)?)?;
    Ok(())
}

/// Roll the instance back to a restore point, in place: reset content to the
/// recorded set (re-linking cached blobs) and restore the config overrides.
pub async fn restore(app: &AppHandle, instance_id: &str, id: &str) -> Result<()> {
    let path = point_path(app, instance_id, id)?;
    let stored = read_point(&path)?;

    // Safety net: snapshot the current (working) set first, so even a botched
    // restore is itself undoable (its files are already cached → re-linkable).
    let _ = create(app, instance_id, "Before restoring", true);

    // Pre-flight: resolve + cache every recorded file BEFORE deleting anything.
    // If a version was delisted, opted out of distribution, or we're offline, we
    // abort here with the instance untouched — never half-wiped.
    let client = crate::modrinth::client()?;
    for entry in &stored.content {
        let version = sources::get_version(parse_source(&entry.source), &entry.version_id)
            .await
            .map_err(|error| AppError::Other(format!("Can't restore \"{}\": {error}", entry.title)))?;
        let file = version.primary_file().ok_or_else(|| {
            AppError::Other(format!("Can't restore \"{}\": no downloadable file", entry.title))
        })?;
        if file.url.is_empty() {
            return Err(AppError::Other(format!(
                "Can't restore \"{}\": it's no longer available for automatic download.",
                entry.title
            )));
        }
        content::cache::fetch(&client, app, &file.url, file.hashes.sha1.as_deref())
            .await
            .map_err(|error| AppError::Other(format!("Can't restore \"{}\": {error}", entry.title)))?;
    }

    // Everything is cached now — the swap is fast and can't fail on the network.
    content::clear(app, instance_id)?;
    for entry in &stored.content {
        content::install(
            app,
            instance_id,
            parse_source(&entry.source),
            &entry.version_id,
            &entry.project_type,
            &entry.title,
            entry.icon_url.clone(),
        )
        .await?;
        if !entry.enabled {
            let _ = content::set_enabled(app, instance_id, &entry.version_id, false);
        }
    }

    let game_dir = paths::instance_game_dir(app, instance_id)?;
    extract_overrides(&path, &game_dir)?;
    Ok(())
}

fn extract_overrides(pack: &Path, game_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(pack)?;
    let mut zip = zip::ZipArchive::new(file)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_string();
        let Some(rel) = name.strip_prefix("overrides/") else {
            continue;
        };
        if rel.is_empty() || entry.is_dir() || name.ends_with('/') {
            continue;
        }
        let Some(rel) = safe_rel(rel) else { continue };
        let dest = game_dir.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn safe_rel(path: &str) -> Option<PathBuf> {
    let path_buf = PathBuf::from(path);
    if path_buf.is_absolute() {
        return None;
    }
    for component in path_buf.components() {
        if matches!(component, Component::ParentDir | Component::Prefix(_) | Component::RootDir) {
            return None;
        }
    }
    Some(path_buf)
}

fn prune_auto(app: &AppHandle, instance_id: &str) -> Result<()> {
    let mut autos: Vec<RestorePoint> = list(app, instance_id)?
        .into_iter()
        .filter(|p| p.auto)
        .collect();
    // Newest first already; drop everything past the keep window.
    for old in autos.split_off(autos.len().min(KEEP_AUTO)) {
        let _ = delete(app, instance_id, &old.id);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Update detection + application
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdate {
    /// The currently-installed version id (the row's key).
    pub version_id: String,
    pub project_id: String,
    pub project_type: String,
    pub source: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub latest_version_id: String,
    pub latest_number: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub updated: usize,
    pub skipped: Vec<String>,
    pub restore_point_id: String,
}

/// Concurrently check every installed, provider-backed item for a newer version
/// compatible with the instance's Minecraft version + loader.
pub async fn check_updates(app: &AppHandle, instance_id: &str) -> Result<Vec<ContentUpdate>> {
    let instance = app
        .state::<InstanceStore>()
        .get(instance_id)
        .ok_or_else(|| AppError::Other(format!("instance '{instance_id}' not found")))?;
    let mc_version = instance.mc_version.clone();
    let loader = loader_str(instance.loader).to_string();

    let items: Vec<_> = content::list(app, instance_id)?
        .into_iter()
        .filter(|item| item.project_id.is_some())
        .collect();

    let results = stream::iter(items)
        .map(|item| {
            let mc_version = mc_version.clone();
            let loader = loader.clone();
            async move {
                let project_id = item.project_id.clone()?;
                let source = parse_source(&item.source);
                // Only mods on a modded instance filter by loader.
                let loader_filter =
                    (item.project_type == "mod" && loader != "vanilla").then_some(loader.as_str());
                let versions =
                    sources::get_versions(source, &project_id, loader_filter, Some(&mc_version))
                        .await
                        .ok()?;
                let latest = versions.into_iter().next()?;
                if latest.id == item.version_id {
                    return None;
                }
                Some(ContentUpdate {
                    version_id: item.version_id,
                    project_id,
                    project_type: item.project_type,
                    source: item.source,
                    title: item.title,
                    icon_url: item.icon_url,
                    latest_version_id: latest.id,
                    latest_number: latest.version_number,
                })
            }
        })
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await;

    Ok(results.into_iter().flatten().collect())
}

/// Apply a set of updates after auto-creating a restore point. Failures are
/// collected (by title) rather than aborting the whole batch; the restore point
/// lets the user undo everything regardless.
pub async fn apply_updates(
    app: &AppHandle,
    instance_id: &str,
    updates: Vec<ContentUpdate>,
) -> Result<ApplyResult> {
    let label = if updates.len() == 1 {
        format!("Before updating {}", updates[0].title)
    } else {
        format!("Before updating {} items", updates.len())
    };
    let point = create(app, instance_id, &label, true)?;

    let mut updated = 0;
    let mut skipped = Vec::new();
    for update in &updates {
        match content::install(
            app,
            instance_id,
            parse_source(&update.source),
            &update.latest_version_id,
            &update.project_type,
            &update.title,
            update.icon_url.clone(),
        )
        .await
        {
            Ok(_) => updated += 1,
            Err(_) => skipped.push(update.title.clone()),
        }
    }

    Ok(ApplyResult {
        updated,
        skipped,
        restore_point_id: point.id,
    })
}
