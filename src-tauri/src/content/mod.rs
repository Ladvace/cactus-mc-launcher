use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::{Instance, InstanceKind, ModLoader};
use crate::launch::download::{download_one, DownloadTask};

pub mod cache;
pub use cache::CacheStats;
use crate::modrinth;
use crate::paths;
use crate::sources::{self, Source};

/// A piece of content installed into an instance (mod, resource pack, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    pub project_id: Option<String>,
    pub version_id: String,
    /// "mod" | "resourcepack" | "shader" | "datapack"
    pub project_type: String,
    pub title: String,
    /// Base file name (without a `.disabled` suffix).
    pub file_name: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Provider this came from ("modrinth" | "curseforge").
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "modrinth".into()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ContentFile {
    #[serde(default)]
    items: Vec<ContentItem>,
}

/// The `content.json` sidecar for an instance.
fn content_file(app: &AppHandle, instance_id: &str) -> Result<PathBuf> {
    Ok(paths::instance_dir(app, instance_id)?.join("content.json"))
}

fn read_content(app: &AppHandle, instance_id: &str) -> Result<ContentFile> {
    let file = content_file(app, instance_id)?;
    if !file.exists() {
        return Ok(ContentFile::default());
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(file)?).unwrap_or_default())
}

fn write_content(app: &AppHandle, instance_id: &str, data: &ContentFile) -> Result<()> {
    std::fs::write(
        content_file(app, instance_id)?,
        serde_json::to_string_pretty(data)?,
    )?;
    Ok(())
}

/// The game subfolder a project type installs into.
fn subdir(project_type: &str) -> &'static str {
    match project_type {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        "datapack" => "datapacks",
        _ => "mods",
    }
}

fn target_dir(app: &AppHandle, instance_id: &str, project_type: &str) -> Result<PathBuf> {
    let dir = paths::instance_game_dir(app, instance_id)?.join(subdir(project_type));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The on-disk file name for an item: the base name, plus a `.disabled` suffix
/// when disabled.
fn disk_name(file_name: &str, enabled: bool) -> String {
    if enabled {
        file_name.to_string()
    } else {
        format!("{file_name}.disabled")
    }
}

/// The on-disk path of an item, accounting for the `.disabled` suffix.
fn item_path(app: &AppHandle, instance_id: &str, item: &ContentItem) -> Result<PathBuf> {
    let dir = target_dir(app, instance_id, &item.project_type)?;
    Ok(dir.join(disk_name(&item.file_name, item.enabled)))
}

/// Install a content version from a provider into an instance.
pub async fn install(
    app: &AppHandle,
    instance_id: &str,
    source: Source,
    version_id: &str,
    project_type: &str,
    title: &str,
    icon_url: Option<String>,
) -> Result<ContentItem> {
    let version = sources::get_version(source, version_id).await?;
    let file = version
        .primary_file()
        .ok_or_else(|| AppError::Other("this version has no downloadable file".into()))?;

    if file.url.is_empty() {
        return Err(AppError::Other(
            "This file can't be downloaded automatically — the author opted out \
             of third-party distribution on CurseForge. Download it from the \
             project's CurseForge page instead."
                .into(),
        ));
    }

    let dir = target_dir(app, instance_id, project_type)?;
    let dest = dir.join(&file.filename);

    // Deduplicated: fetched once into the shared cache, then hard-linked here.
    let client = modrinth::client()?;
    cache::install_one(&client, app, &file.url, &dest, file.hashes.sha1.as_deref()).await?;

    let item = ContentItem {
        project_id: Some(version.project_id.clone()),
        version_id: version.id.clone(),
        project_type: project_type.to_string(),
        title: title.to_string(),
        file_name: file.filename.clone(),
        icon_url,
        enabled: true,
        source: format!("{source:?}").to_lowercase(),
    };

    let mut data = read_content(app, instance_id)?;
    // Replace any existing entry for the same project (upgrade in place).
    if let Some(project_id) = &item.project_id {
        for old in data
            .items
            .iter()
            .filter(|existing| existing.project_id.as_ref() == Some(project_id))
        {
            let _ = std::fs::remove_file(item_path(app, instance_id, old)?);
        }
        data.items
            .retain(|existing| existing.project_id.as_ref() != Some(project_id));
    }
    data.items.push(item.clone());
    write_content(app, instance_id, &data)?;

    Ok(item)
}

/// List installed content for an instance.
pub fn list(app: &AppHandle, instance_id: &str) -> Result<Vec<ContentItem>> {
    Ok(read_content(app, instance_id)?.items)
}

/// Enable or disable an item (toggles its `.disabled` extension on disk).
pub fn set_enabled(
    app: &AppHandle,
    instance_id: &str,
    version_id: &str,
    enabled: bool,
) -> Result<()> {
    let mut data = read_content(app, instance_id)?;
    let item = data
        .items
        .iter_mut()
        .find(|candidate| candidate.version_id == version_id)
        .ok_or_else(|| AppError::Other("content not found".into()))?;

    if item.enabled != enabled {
        let dir = target_dir(app, instance_id, &item.project_type)?;
        let from = dir.join(disk_name(&item.file_name, item.enabled));
        let to = dir.join(disk_name(&item.file_name, enabled));
        if from.exists() {
            std::fs::rename(&from, &to)?;
        }
        item.enabled = enabled;
    }

    write_content(app, instance_id, &data)?;
    Ok(())
}

// --- Modpack (.mrpack) install ---------------------------------------------

#[derive(Debug, Deserialize)]
struct MrIndex {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default)]
    files: Vec<MrFile>,
}

#[derive(Debug, Deserialize)]
struct MrFile {
    path: String,
    #[serde(default)]
    hashes: MrHashes,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    env: Option<MrEnv>,
}

#[derive(Debug, Default, Deserialize)]
struct MrHashes {
    #[serde(default)]
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MrEnv {
    #[serde(default)]
    client: Option<String>,
}

/// The content type for a pack file, from its top-level directory. `None` for
/// files that aren't tracked content (configs, etc.).
fn content_type_for(path: &str) -> Option<&'static str> {
    match path.trim_start_matches("./").split('/').next()? {
        "mods" => Some("mod"),
        "resourcepacks" => Some("resourcepack"),
        "shaderpacks" => Some("shader"),
        "datapacks" => Some("datapack"),
        _ => None,
    }
}

/// Parse `(project_id, version_id)` from a Modrinth CDN URL
/// (`https://cdn.modrinth.com/data/<project>/versions/<version>/<file>`).
fn parse_modrinth_ids(url: &str) -> Option<(String, String)> {
    let mut parts = url.split("/data/").nth(1)?.split('/');
    let project = parts.next()?.to_string();
    if parts.next()? != "versions" {
        return None;
    }
    Some((project, parts.next()?.to_string()))
}

/// Reject archive paths that try to escape the target directory.
fn safe_rel(path: &str) -> Option<PathBuf> {
    if path.is_empty() || path.starts_with('/') || path.contains("..") {
        return None;
    }
    Some(PathBuf::from(path))
}

fn emit_progress(
    app: &AppHandle,
    instance_id: Option<&str>,
    current: usize,
    total: usize,
    message: &str,
) {
    let _ = app.emit(
        "modpack-progress",
        serde_json::json!({
            "instanceId": instance_id,
            "current": current,
            "total": total,
            "message": message,
        }),
    );
}

/// Fetch a small image and encode it as a data URI so instance icons persist
/// offline. Returns `None` on any failure or if the image is unexpectedly large.
async fn fetch_icon_data_uri(client: &reqwest::Client, url: &str) -> Option<String> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/png")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_string();
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > 2_000_000 {
        return None;
    }
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{content_type};base64,{encoded}"))
}

/// Install a Modrinth modpack version as a brand-new instance: downloads the
/// `.mrpack`, reads its index, creates the instance, downloads every file, and
/// applies the pack's `overrides`.
pub async fn install_modpack(
    app: &AppHandle,
    version_id: &str,
    icon_url: Option<String>,
) -> Result<Instance> {
    emit_progress(app, None, 0, 0, "Downloading modpack…");
    let version = modrinth::get_version(version_id).await?;
    let file = version
        .primary_file()
        .ok_or_else(|| AppError::Other("this modpack has no file".into()))?;

    let client = modrinth::client()?;
    let tmp = paths::meta_dir(app)?.join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let mrpack = tmp.join(format!("{version_id}.mrpack"));
    download_one(
        &client,
        &DownloadTask {
            url: file.url.clone(),
            dest: mrpack.clone(),
            sha1: file.hashes.sha1.clone(),
            executable: false,
        },
    )
    .await?;

    // Read the pack index from the zip.
    let index: MrIndex = {
        let file = std::fs::File::open(&mrpack)?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut entry = zip
            .by_name("modrinth.index.json")
            .map_err(|_| AppError::Other("modpack is missing modrinth.index.json".into()))?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        serde_json::from_str(&text)?
    };

    // Resolve Minecraft version + loader from dependencies.
    let mc_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| AppError::Other("modpack does not specify a Minecraft version".into()))?;

    let (loader, loader_version) = if let Some(version) = index.dependencies.get("fabric-loader") {
        (ModLoader::Fabric, Some(version.clone()))
    } else if let Some(version) = index.dependencies.get("quilt-loader") {
        (ModLoader::Quilt, Some(version.clone()))
    } else if let Some(version) = index.dependencies.get("neoforge") {
        (ModLoader::NeoForge, Some(version.clone()))
    } else if let Some(version) = index.dependencies.get("forge") {
        (ModLoader::Forge, Some(version.clone()))
    } else {
        (ModLoader::Vanilla, None)
    };

    let name = index.name.clone().unwrap_or_else(|| version.name.clone());
    let icon = match icon_url.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => fetch_icon_data_uri(&client, url).await,
        None => None,
    };
    let instance = Instance::new(name, InstanceKind::Client, mc_version, loader, loader_version, icon);
    app.state::<InstanceStore>().save(app, &instance)?;

    // Download the pack's files (client-relevant only).
    let game_dir = paths::instance_game_dir(app, &instance.id)?;
    let tasks: Vec<DownloadTask> = index
        .files
        .iter()
        .filter(|file| {
            file.env
                .as_ref()
                .and_then(|env| env.client.as_deref())
                .map(|client_env| client_env != "unsupported")
                .unwrap_or(true)
        })
        .filter_map(|file| {
            let rel = safe_rel(&file.path)?;
            let url = file.downloads.first()?.clone();
            Some(DownloadTask {
                url,
                dest: game_dir.join(rel),
                sha1: file.hashes.sha1.clone(),
                executable: false,
            })
        })
        .collect();

    {
        let app_cb = app.clone();
        let id = instance.id.clone();
        cache::install_all(&client, app, tasks, 12, move |cur, total| {
            emit_progress(&app_cb, Some(&id), cur, total, "Downloading mods…");
        })
        .await?;
    }

    // Apply overrides (files bundled directly in the pack).
    emit_progress(app, Some(&instance.id), 0, 0, "Applying overrides…");
    apply_overrides(&mrpack, &game_dir)?;

    // Record the pack's mods/packs so the instance's Content tab lists them
    // (and they can be toggled, updated, or removed like any other content).
    let mut content = read_content(app, &instance.id)?;
    for file in &index.files {
        let client_ok = file
            .env
            .as_ref()
            .and_then(|env| env.client.as_deref())
            .map(|client_env| client_env != "unsupported")
            .unwrap_or(true);
        let Some(project_type) = content_type_for(&file.path) else { continue };
        let Some(rel) = safe_rel(&file.path) else { continue };
        if !client_ok {
            continue;
        }
        let file_name = rel.file_name().unwrap_or_default().to_string_lossy().to_string();
        if file_name.is_empty() {
            continue;
        }
        let ids = file.downloads.first().and_then(|url| parse_modrinth_ids(url));
        let (project_id, version_id) = match ids {
            Some((project, version)) => (Some(project), version),
            // No Modrinth ids (external download) — key on the hash/name instead.
            None => (None, file.hashes.sha1.clone().unwrap_or_else(|| file_name.clone())),
        };
        let title = file_name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&file_name);
        content.items.push(ContentItem {
            project_id,
            version_id,
            project_type: project_type.to_string(),
            title: title.to_string(),
            file_name,
            icon_url: None,
            enabled: true,
            source: "modrinth".into(),
        });
    }
    write_content(app, &instance.id, &content)?;

    let _ = std::fs::remove_file(&mrpack);
    emit_progress(app, Some(&instance.id), 1, 1, "Done");
    Ok(instance)
}

/// Extract `overrides/` and `client-overrides/` from the pack into the game dir.
fn apply_overrides(mrpack: &PathBuf, game_dir: &std::path::Path) -> Result<()> {
    let file = std::fs::File::open(mrpack)?;
    let mut zip = zip::ZipArchive::new(file)?;

    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)?;
        let name = entry.name().to_string();
        let rel = name
            .strip_prefix("overrides/")
            .or_else(|| name.strip_prefix("client-overrides/"));
        let Some(rel) = rel else { continue };
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

/// Remove an item and its file from the instance.
pub fn remove(app: &AppHandle, instance_id: &str, version_id: &str) -> Result<()> {
    let mut data = read_content(app, instance_id)?;
    if let Some(pos) = data.items.iter().position(|item| item.version_id == version_id) {
        let item = data.items.remove(pos);
        let _ = std::fs::remove_file(item_path(app, instance_id, &item)?);
        write_content(app, instance_id, &data)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_rel_blocks_escapes() {
        assert!(safe_rel("").is_none());
        assert!(safe_rel("/etc/passwd").is_none());
        assert!(safe_rel("../secrets").is_none());
        assert!(safe_rel("a/../b").is_none());
        assert!(safe_rel("mods/cool.jar").is_some());
        assert!(safe_rel("config/sub/file.toml").is_some());
    }

    #[test]
    fn subdir_maps_project_types() {
        assert_eq!(subdir("resourcepack"), "resourcepacks");
        assert_eq!(subdir("shader"), "shaderpacks");
        assert_eq!(subdir("datapack"), "datapacks");
        assert_eq!(subdir("mod"), "mods");
        assert_eq!(subdir("anything-else"), "mods");
    }

    #[test]
    fn content_type_from_pack_path() {
        assert_eq!(content_type_for("mods/sodium.jar"), Some("mod"));
        assert_eq!(content_type_for("resourcepacks/x.zip"), Some("resourcepack"));
        assert_eq!(content_type_for("shaderpacks/x.zip"), Some("shader"));
        assert_eq!(content_type_for("config/foo.toml"), None);
    }

    #[test]
    fn parses_modrinth_cdn_ids() {
        assert_eq!(
            parse_modrinth_ids("https://cdn.modrinth.com/data/AABBCCDD/versions/11223344/sodium.jar"),
            Some(("AABBCCDD".into(), "11223344".into()))
        );
        assert_eq!(parse_modrinth_ids("https://example.com/some/other.jar"), None);
    }
}
