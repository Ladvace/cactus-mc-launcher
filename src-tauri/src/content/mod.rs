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
    paths::write_atomic(
        &content_file(app, instance_id)?,
        serde_json::to_string_pretty(data)?.as_bytes(),
    )
}

static CONTENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialize read-modify-write on content.json so concurrent operations on the
/// same instance can't clobber each other. `mutate` runs under the lock against
/// a freshly-read copy, which is then written atomically. Any network/download
/// work must happen *before* calling this (don't hold the lock across it).
fn update_content<F>(app: &AppHandle, instance_id: &str, mutate: F) -> Result<()>
where
    F: FnOnce(&mut ContentFile) -> Result<()>,
{
    // Recover a poisoned lock (a panicked mutation) instead of bricking all
    // future content ops for the session.
    let _guard = CONTENT_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let mut data = read_content(app, instance_id)?;
    mutate(&mut data)?;
    write_content(app, instance_id, &data)
}

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

fn disk_name(file_name: &str, enabled: bool) -> String {
    if enabled {
        file_name.to_string()
    } else {
        format!("{file_name}.disabled")
    }
}

/// Validate a filename that came from a remote source before joining it onto a
/// directory. A name with a path separator, a `..` component, or an absolute
/// path could otherwise escape the instance folder (`Path::join` doesn't collapse
/// `..`, and an absolute join replaces the base entirely). We require the name to
/// be exactly its own single leaf component and reject anything else. Backslashes
/// are rejected too — a separator on Windows, and never wanted in a name here.
fn safe_file_name(name: &str) -> Result<&str> {
    let safe = !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && std::path::Path::new(name).file_name().and_then(|n| n.to_str()) == Some(name);
    if safe {
        Ok(name)
    } else {
        Err(AppError::Other(format!("unsafe filename from source: {name}")))
    }
}

fn item_path(app: &AppHandle, instance_id: &str, item: &ContentItem) -> Result<PathBuf> {
    let dir = target_dir(app, instance_id, &item.project_type)?;
    Ok(dir.join(disk_name(safe_file_name(&item.file_name)?, item.enabled)))
}

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
    // The filename comes from the remote source; refuse a path-traversal name
    // before it's written to disk or stored on the item.
    let dest = dir.join(safe_file_name(&file.filename)?);

    // Fetch the blob outside the lock (network); the link happens under it.
    let client = modrinth::client()?;
    let blob = cache::fetch(&client, app, &file.url, file.hashes.sha1.as_deref()).await?;

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

    update_content(app, instance_id, |data| {
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
        // Link in *after* removing older versions, so a concurrent install of the
        // same file can't delete the copy we just placed (the removal and the link
        // are one atomic step under the lock).
        cache::link_into(&blob, &dest)?;
        data.items.push(item.clone());
        Ok(())
    })?;

    Ok(item)
}

pub fn list(app: &AppHandle, instance_id: &str) -> Result<Vec<ContentItem>> {
    Ok(read_content(app, instance_id)?.items)
}

pub fn register_local(app: &AppHandle, instance_id: &str) -> Result<usize> {
    let mut added = 0usize;
    update_content(app, instance_id, |data| {
        let mut known: std::collections::HashSet<String> = data
            .items
            .iter()
            .map(|item| format!("{}/{}", item.project_type, item.file_name.to_lowercase()))
            .collect();

        for (subdir_name, project_type, exts) in [
            ("mods", "mod", &["jar"][..]),
            ("resourcepacks", "resourcepack", &["zip"][..]),
            ("shaderpacks", "shader", &["zip"][..]),
            ("datapacks", "datapack", &["zip"][..]),
        ] {
            let dir = paths::instance_game_dir(app, instance_id)?.join(subdir_name);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().map(|kind| kind.is_file()).unwrap_or(false) {
                    continue;
                }
                let raw = entry.file_name().to_string_lossy().to_string();
                let (file_name, enabled) = match raw.strip_suffix(".disabled") {
                    Some(base) => (base.to_string(), false),
                    None => (raw.clone(), true),
                };
                if file_name.is_empty() {
                    continue;
                }
                let ext = std::path::Path::new(&file_name)
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_lowercase);
                if !ext.as_deref().map(|value| exts.contains(&value)).unwrap_or(false) {
                    continue;
                }
                if !known.insert(format!("{project_type}/{}", file_name.to_lowercase())) {
                    continue;
                }
                let title = std::path::Path::new(&file_name)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&file_name)
                    .to_string();
                data.items.push(ContentItem {
                    project_id: None,
                    version_id: format!("local:{project_type}:{file_name}"),
                    project_type: project_type.into(),
                    title,
                    file_name,
                    icon_url: None,
                    enabled,
                    source: "local".into(),
                });
                added += 1;
            }
        }
        Ok(())
    })?;
    Ok(added)
}

fn file_sha1(path: &std::path::Path) -> Option<String> {
    use sha1::{Digest, Sha1};
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(hex::encode(hasher.finalize()))
}

/// Match `local` items (loose files imported from another launcher) against
/// Modrinth by file hash, upgrading any match into a tracked item with a real
/// title, icon, project and version id — so it shows properly and can be updated.
/// Best-effort: unmatched items and any network failure leave items untouched.
pub async fn resolve_local(app: &AppHandle, instance_id: &str) -> Result<usize> {
    // Phase 1 (no lock): hash the local items and query Modrinth. Keyed by the
    // item's stable local version_id so the write can re-match after any
    // concurrent change.
    let snapshot = read_content(app, instance_id)?;
    let mut hashes: Vec<String> = Vec::new();
    let mut vids_by_hash: HashMap<String, Vec<String>> = HashMap::new();
    for item in &snapshot.items {
        if item.source != "local" || item.project_id.is_some() {
            continue;
        }
        let Some(sha1) = file_sha1(&item_path(app, instance_id, item)?) else {
            continue;
        };
        vids_by_hash.entry(sha1.clone()).or_default().push(item.version_id.clone());
        hashes.push(sha1);
    }
    hashes.sort();
    hashes.dedup();
    if hashes.is_empty() {
        return Ok(0);
    }

    let versions = modrinth::version_files(&hashes).await.unwrap_or_default();
    if versions.is_empty() {
        return Ok(0);
    }
    let project_ids: Vec<String> = versions
        .values()
        .map(|version| version.project_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let projects: HashMap<String, modrinth::Project> = modrinth::projects(&project_ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

    // local version_id -> the tracked item it should become.
    let mut upgrades: HashMap<String, ContentUpgrade> = HashMap::new();
    for (sha1, version) in &versions {
        let Some(vids) = vids_by_hash.get(sha1) else {
            continue;
        };
        // Two loose files with identical content would collapse onto one
        // version_id, making the second unmanageable; leave such duplicates local.
        let [vid] = &vids[..] else {
            continue;
        };
        let project = projects.get(&version.project_id);
        upgrades.insert(
            vid.clone(),
            ContentUpgrade {
                project_id: version.project_id.clone(),
                version_id: version.id.clone(),
                title: project.map(|p| p.title.clone()).filter(|t| !t.is_empty()),
                icon_url: project.and_then(|p| p.icon_url.clone()),
            },
        );
    }
    if upgrades.is_empty() {
        return Ok(0);
    }

    // Phase 2 (locked): re-read and apply the upgrades to whatever is still local.
    let mut matched = 0usize;
    update_content(app, instance_id, |data| {
        for item in &mut data.items {
            if item.source != "local" || item.project_id.is_some() {
                continue;
            }
            if let Some(upgrade) = upgrades.get(&item.version_id) {
                item.project_id = Some(upgrade.project_id.clone());
                item.version_id = upgrade.version_id.clone();
                item.source = "modrinth".into();
                if let Some(title) = &upgrade.title {
                    item.title = title.clone();
                }
                item.icon_url = upgrade.icon_url.clone();
                matched += 1;
            }
        }
        Ok(())
    })?;
    Ok(matched)
}

struct ContentUpgrade {
    project_id: String,
    version_id: String,
    title: Option<String>,
    icon_url: Option<String>,
}

#[cfg(unix)]
fn path_inode(path: &std::path::Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|meta| meta.ino())
}

// A stable per-file identity isn't available on non-Unix std (file_index is
// nightly-only), so hard-link/shared detection simply reports nothing there.
#[cfg(not(unix))]
fn path_inode(_path: &std::path::Path) -> Option<u64> {
    None
}

/// Version ids of this instance's content whose file is the same on-disk file
/// (hard link) as one in another instance — i.e. physically shared.
pub fn shared_version_ids(app: &AppHandle, instance_id: &str) -> Result<Vec<String>> {
    let mut inode_instances: HashMap<u64, std::collections::HashSet<String>> = HashMap::new();
    let store = app.state::<InstanceStore>();
    for instance in store.list() {
        let Ok(game) = paths::instance_game_dir(app, &instance.id) else {
            continue;
        };
        for sub in ["mods", "resourcepacks", "shaderpacks", "datapacks"] {
            let Ok(entries) = std::fs::read_dir(game.join(sub)) else {
                continue;
            };
            for entry in entries.flatten() {
                if let Some(inode) = path_inode(&entry.path()) {
                    inode_instances
                        .entry(inode)
                        .or_default()
                        .insert(instance.id.clone());
                }
            }
        }
    }

    let mut out = Vec::new();
    for item in list(app, instance_id)? {
        let Some(inode) = path_inode(&item_path(app, instance_id, &item)?) else {
            continue;
        };
        if inode_instances.get(&inode).map_or(0, |set| set.len()) >= 2 {
            out.push(item.version_id);
        }
    }
    Ok(out)
}

/// Delete every tracked content file and empty `content.json`. Blobs stay in the
/// shared cache, so re-installing (e.g. during a restore) is just a re-link.
pub fn clear(app: &AppHandle, instance_id: &str) -> Result<()> {
    let _guard = CONTENT_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let data = read_content(app, instance_id)?;
    for item in &data.items {
        if let Ok(path) = item_path(app, instance_id, item) {
            let _ = std::fs::remove_file(path);
        }
    }
    write_content(app, instance_id, &ContentFile::default())
}

pub fn set_enabled(
    app: &AppHandle,
    instance_id: &str,
    version_id: &str,
    enabled: bool,
) -> Result<()> {
    update_content(app, instance_id, |data| {
        let item = data
            .items
            .iter_mut()
            .find(|candidate| candidate.version_id == version_id)
            .ok_or_else(|| AppError::Other("content not found".into()))?;

        if item.enabled != enabled {
            let dir = target_dir(app, instance_id, &item.project_type)?;
            let name = safe_file_name(&item.file_name)?;
            let from = dir.join(disk_name(name, item.enabled));
            let to = dir.join(disk_name(name, enabled));
            if from.exists() {
                std::fs::rename(&from, &to)?;
            }
            item.enabled = enabled;
        }
        Ok(())
    })
}

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

/// Reject archive paths that try to escape the target directory. Uses a
/// component-based check (not string matching) so Windows-absolute, drive-root,
/// UNC and `..` paths are all refused — a bare `contains("..")` misses those.
fn safe_rel(path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    let path_buf = PathBuf::from(path);
    if path_buf.is_absolute() {
        return None;
    }
    for component in path_buf.components() {
        if matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::Prefix(_)
                | std::path::Component::RootDir
        ) {
            return None;
        }
    }
    Some(path_buf)
}

/// Hosts a modpack may download files from — the Modrinth `.mrpack` spec's
/// allow-list. Modrinth enforces the same set on upload, so legitimate packs
/// only ever use these.
const ALLOWED_PACK_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "objects.githubusercontent.com",
    "gitlab.com",
    "user-content.gitlab-static.net",
];

/// Validate a modpack file's download entry before it's fetched. An imported
/// pack is fully untrusted, so we require: an HTTPS URL, a host on the allow-list
/// (blocks the launcher being used as an SSRF proxy into internal/loopback/cloud
/// -metadata services), and a content hash (so every file is pinned rather than
/// silently swappable). Returns the validated `(url, sha1)` or a descriptive
/// error that fails the whole import closed.
pub(crate) fn validate_pack_download(
    downloads: &[String],
    sha1: Option<String>,
) -> Result<(String, String)> {
    let url = downloads
        .first()
        .ok_or_else(|| AppError::Other("a modpack file has no download URL".into()))?;
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| AppError::Other(format!("modpack has an invalid download URL: {url}")))?;
    if parsed.scheme() != "https" {
        return Err(AppError::Other(format!(
            "refusing non-HTTPS modpack download: {url}"
        )));
    }
    let host = parsed.host_str().unwrap_or_default();
    if !ALLOWED_PACK_HOSTS.contains(&host) {
        return Err(AppError::Other(format!(
            "modpack download from untrusted host '{host}' was blocked"
        )));
    }
    let sha1 = sha1
        .ok_or_else(|| AppError::Other(format!("modpack file '{url}' is missing its sha1 hash")))?;
    Ok((url.clone(), sha1))
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

    let game_dir = paths::instance_game_dir(app, &instance.id)?;
    let mut tasks: Vec<DownloadTask> = Vec::new();
    for file in &index.files {
        let client_supported = file
            .env
            .as_ref()
            .and_then(|env| env.client.as_deref())
            .map(|client_env| client_env != "unsupported")
            .unwrap_or(true);
        if !client_supported {
            continue;
        }
        let rel = safe_rel(&file.path)
            .ok_or_else(|| AppError::Other(format!("unsafe path in modpack: {}", file.path)))?;
        let (url, sha1) = validate_pack_download(&file.downloads, file.hashes.sha1.clone())?;
        tasks.push(DownloadTask {
            url,
            dest: game_dir.join(rel),
            sha1: Some(sha1),
            executable: false,
        });
    }

    {
        let app_cb = app.clone();
        let id = instance.id.clone();
        let concurrency = crate::settings::clamp_concurrency(
            app.state::<crate::settings::SettingsStore>().get().max_concurrent_downloads,
        );
        cache::install_all(&client, app, tasks, concurrency, move |cur, total| {
            emit_progress(&app_cb, Some(&id), cur, total, "Downloading mods…");
        })
        .await?;
    }

    emit_progress(app, Some(&instance.id), 0, 0, "Applying overrides…");
    apply_overrides(&mrpack, &game_dir)?;

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

#[derive(Deserialize)]
struct CfManifest {
    #[serde(default)]
    name: Option<String>,
    minecraft: CfMinecraft,
    #[serde(default)]
    files: Vec<CfManifestFile>,
}

#[derive(Deserialize)]
struct CfMinecraft {
    version: String,
    #[serde(rename = "modLoaders", default)]
    mod_loaders: Vec<CfModLoader>,
}

#[derive(Deserialize)]
struct CfModLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
}

/// Parse a CurseForge `modLoaders` id (e.g. `fabric-0.15.0`) into our loader.
fn parse_cf_loader(id: &str) -> (ModLoader, Option<String>) {
    let (name, version) = id.split_once('-').unwrap_or((id, ""));
    let loader = match name {
        "fabric" => ModLoader::Fabric,
        "quilt" => ModLoader::Quilt,
        "neoforge" => ModLoader::NeoForge,
        "forge" => ModLoader::Forge,
        _ => ModLoader::Vanilla,
    };
    let version = (!version.is_empty()).then(|| version.to_string());
    (loader, version)
}

pub async fn install_cf_modpack(
    app: &AppHandle,
    version_id: &str,
    icon_url: Option<String>,
) -> Result<Instance> {
    use futures::stream::{self, StreamExt};

    emit_progress(app, None, 0, 0, "Downloading modpack…");
    let version = sources::curseforge::get_version(version_id).await?;
    let file = version
        .primary_file()
        .ok_or_else(|| AppError::Other("this modpack has no file".into()))?;
    if file.url.is_empty() {
        return Err(AppError::Other(
            "This modpack can't be downloaded automatically (author opted out on CurseForge)."
                .into(),
        ));
    }

    let client = crate::http::client()?;
    let tmp = paths::meta_dir(app)?.join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let pack = tmp.join(&file.filename);
    download_one(
        &client,
        &DownloadTask {
            url: file.url.clone(),
            dest: pack.clone(),
            sha1: file.hashes.sha1.clone(),
            executable: false,
        },
    )
    .await?;

    let manifest: CfManifest = {
        let f = std::fs::File::open(&pack)?;
        let mut zip = zip::ZipArchive::new(f)?;
        let mut entry = zip
            .by_name("manifest.json")
            .map_err(|_| AppError::Other("modpack is missing manifest.json".into()))?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        serde_json::from_str(&text)?
    };

    let (loader, loader_version) = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .map(|l| parse_cf_loader(&l.id))
        .unwrap_or((ModLoader::Vanilla, None));

    let name = manifest.name.clone().unwrap_or_else(|| version.name.clone());
    let icon = match icon_url.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => fetch_icon_data_uri(&client, url).await,
        None => None,
    };
    let instance = Instance::new(
        name,
        InstanceKind::Client,
        manifest.minecraft.version.clone(),
        loader,
        loader_version,
        icon,
    );
    app.state::<InstanceStore>().save(app, &instance)?;
    let game_dir = paths::instance_game_dir(app, &instance.id)?;
    let mods_dir = game_dir.join("mods");

    emit_progress(app, Some(&instance.id), 0, 0, "Resolving mods…");
    let concurrency =
        crate::settings::clamp_concurrency(app.state::<crate::settings::SettingsStore>().get().max_concurrent_downloads);
    let resolved: Vec<Option<(DownloadTask, ContentItem)>> = stream::iter(manifest.files)
        .map(|f| {
            let mods_dir = mods_dir.clone();
            async move {
                let composite = format!("{}:{}", f.project_id, f.file_id);
                let v = sources::curseforge::get_version(&composite).await.ok()?;
                let cf_file = v.primary_file()?;
                if cf_file.url.is_empty() {
                    return None; // author opted out — skip
                }
                let title = cf_file.filename.rsplit_once('.').map_or(cf_file.filename.as_str(), |(s, _)| s);
                Some((
                    DownloadTask {
                        url: cf_file.url.clone(),
                        dest: mods_dir.join(&cf_file.filename),
                        sha1: cf_file.hashes.sha1.clone(),
                        executable: false,
                    },
                    ContentItem {
                        project_id: Some(f.project_id.to_string()),
                        version_id: composite,
                        project_type: "mod".into(),
                        title: title.to_string(),
                        file_name: cf_file.filename.clone(),
                        icon_url: None,
                        enabled: true,
                        source: "curseforge".into(),
                    },
                ))
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let (tasks, items): (Vec<_>, Vec<_>) = resolved.into_iter().flatten().unzip();
    {
        let app_cb = app.clone();
        let id = instance.id.clone();
        cache::install_all(&client, app, tasks, concurrency, move |cur, total| {
            emit_progress(&app_cb, Some(&id), cur, total, "Downloading mods…");
        })
        .await?;
    }

    emit_progress(app, Some(&instance.id), 0, 0, "Applying overrides…");
    apply_overrides(&pack, &game_dir)?;

    let mut content = read_content(app, &instance.id)?;
    content.items.extend(items);
    write_content(app, &instance.id, &content)?;

    let _ = std::fs::remove_file(&pack);
    emit_progress(app, Some(&instance.id), 1, 1, "Done");
    Ok(instance)
}

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

pub fn remove(app: &AppHandle, instance_id: &str, version_id: &str) -> Result<()> {
    update_content(app, instance_id, |data| {
        if let Some(pos) = data.items.iter().position(|item| item.version_id == version_id) {
            let item = data.items.remove(pos);
            let _ = std::fs::remove_file(item_path(app, instance_id, &item)?);
        }
        Ok(())
    })
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
    fn safe_file_name_rejects_traversal() {
        assert_eq!(safe_file_name("sodium.jar").unwrap(), "sodium.jar");
        assert!(safe_file_name("").is_err());
        assert!(safe_file_name("..").is_err());
        assert!(safe_file_name(".").is_err());
        assert!(safe_file_name("../evil.jar").is_err());
        assert!(safe_file_name("/etc/passwd").is_err());
        assert!(safe_file_name("sub/dir/mod.jar").is_err());
        assert!(safe_file_name("..\\evil.jar").is_err());
    }

    #[test]
    fn pack_download_validation() {
        let sha = || Some("abc123".to_string());
        // Allowed: https + allow-listed host + a hash.
        assert!(validate_pack_download(
            &["https://cdn.modrinth.com/data/x/y.jar".into()],
            sha()
        )
        .is_ok());
        assert!(validate_pack_download(
            &["https://github.com/o/r/releases/download/v1/y.jar".into()],
            sha()
        )
        .is_ok());
        // Blocked: non-https, untrusted host, SSRF targets, and missing hash.
        assert!(validate_pack_download(&["http://cdn.modrinth.com/x.jar".into()], sha()).is_err());
        assert!(validate_pack_download(&["https://evil.example/x.jar".into()], sha()).is_err());
        assert!(validate_pack_download(&["https://127.0.0.1/x.jar".into()], sha()).is_err());
        assert!(
            validate_pack_download(&["https://169.254.169.254/latest/meta-data".into()], sha())
                .is_err()
        );
        assert!(
            validate_pack_download(&["https://cdn.modrinth.com/x.jar".into()], None).is_err(),
            "a file with no hash must be rejected"
        );
        assert!(validate_pack_download(&[], sha()).is_err());
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
    fn parses_cf_modloader_ids() {
        assert_eq!(parse_cf_loader("fabric-0.15.0"), (ModLoader::Fabric, Some("0.15.0".into())));
        assert_eq!(parse_cf_loader("forge-47.2.0"), (ModLoader::Forge, Some("47.2.0".into())));
        assert_eq!(parse_cf_loader("neoforge-21.0.1"), (ModLoader::NeoForge, Some("21.0.1".into())));
        assert_eq!(parse_cf_loader("quilt-1.2.3"), (ModLoader::Quilt, Some("1.2.3".into())));
        assert_eq!(parse_cf_loader("something"), (ModLoader::Vanilla, None));
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
