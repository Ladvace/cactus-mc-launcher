//! Instance snapshots — portable export/import of an instance's full setup
//! (loader, content references, and config overrides). This is the local,
//! backend-free foundation of the streamer share feature: a snapshot is exactly
//! what a hosted profile will later store and serve.
//!
//! Format `.drakepack` (a zip):
//!   - `index.json`   — loader + content refs (source/project/version) for BOTH
//!                       Modrinth and CurseForge, so mixed packs survive.
//!   - `overrides/…`  — bundled config files (options.txt, keybinds, config/…).
//! Mods themselves are references, not files — the importer resolves and
//! downloads them through the shared, deduplicated content cache.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::content::{self, ContentItem};
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::{Instance, ModLoader};
use crate::launch::download::DownloadTask;
use crate::paths;
use crate::sources::Source;

const FORMAT_VERSION: u32 = 1;

/// A single content reference inside a snapshot.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotContent {
    pub source: String,
    pub project_type: String,
    pub project_id: Option<String>,
    pub version_id: String,
    pub file_name: String,
    pub title: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// The `index.json` of a `.drakepack`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotIndex {
    pub format_version: u32,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub content: Vec<SnapshotContent>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Result of importing a snapshot.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub instance: Instance,
    pub installed: usize,
    /// Titles of content that couldn't be resolved/downloaded (e.g. CurseForge
    /// distribution opt-outs).
    pub skipped: Vec<String>,
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

fn parse_loader(s: &str) -> ModLoader {
    match s {
        "fabric" => ModLoader::Fabric,
        "quilt" => ModLoader::Quilt,
        "forge" => ModLoader::Forge,
        "neoforge" => ModLoader::NeoForge,
        _ => ModLoader::Vanilla,
    }
}

fn parse_source(s: &str) -> Source {
    match s {
        "curseforge" => Source::CurseForge,
        "ftb" => Source::Ftb,
        _ => Source::Modrinth,
    }
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = s.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "setup".into()
    } else {
        trimmed.to_lowercase()
    }
}

fn emit(app: &AppHandle, phase: &str, current: usize, total: usize) {
    let _ = app.emit(
        "snapshot-progress",
        serde_json::json!({ "phase": phase, "current": current, "total": total }),
    );
}

// --- Overrides (config files bundled into the pack) -------------------------

/// Config files to include in a snapshot (small, non-copyrighted text/data).
fn override_files(game_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for f in ["options.txt", "optionsshaders.txt", "servers.dat"] {
        if game_dir.join(f).is_file() {
            out.push(PathBuf::from(f));
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

fn subdir_for(project_type: &str) -> &'static str {
    match project_type {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        "datapack" => "datapacks",
        _ => "mods",
    }
}

/// Result of exporting a snapshot.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    /// Content that couldn't be represented in the chosen format (e.g. non-
    /// Modrinth mods when exporting `.mrpack`).
    pub skipped: Vec<String>,
}

type Zip = zip::ZipWriter<std::fs::File>;

// Keep shared packs small: configs are a convenience, not the point (the mods
// are references). Skip any single override over 2 MB and stop once the bundled
// configs pass ~12 MB total.
const MAX_OVERRIDE_FILE: u64 = 2_000_000;
const MAX_OVERRIDES_TOTAL: u64 = 12_000_000;

/// Add an instance's config `overrides/` tree to an open zip (size-capped).
fn add_overrides(
    zip: &mut Zip,
    opts: zip::write::SimpleFileOptions,
    game_dir: &Path,
) -> Result<()> {
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
        let Ok(data) = std::fs::read(&full) else {
            continue;
        };
        total += data.len() as u64;
        let name = format!("overrides/{}", rel.to_string_lossy().replace('\\', "/"));
        zip.start_file(name, opts)
            .map_err(|e| AppError::Other(format!("zip: {e}")))?;
        zip.write_all(&data)?;
    }
    Ok(())
}

/// Export an instance's setup. `format` is `"drakepack"` (native, full fidelity)
/// or `"mrpack"` (Modrinth interop; Modrinth-hosted content only).
pub async fn export(
    app: &AppHandle,
    instance_id: &str,
    format: &str,
    note: Option<String>,
) -> Result<ExportResult> {
    let instance = app
        .state::<InstanceStore>()
        .get(instance_id)
        .ok_or_else(|| AppError::InstanceNotFound(instance_id.to_string()))?;
    let items = content::list(app, instance_id)?;
    let game_dir = paths::instance_game_dir(app, instance_id)?;

    let exports = paths::data_dir(app)?.join("exports");
    std::fs::create_dir_all(&exports)?;
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let base = sanitize(&instance.name);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    if format == "mrpack" {
        let out_path = exports.join(format!("{base}-{ts}.mrpack"));
        let skipped = export_mrpack(&instance, &items, &game_dir, &out_path, opts).await?;
        Ok(ExportResult {
            path: out_path.to_string_lossy().to_string(),
            skipped,
        })
    } else {
        let out_path = exports.join(format!("{base}-{ts}.drakepack"));
        export_drakepack(&instance, items, &game_dir, &out_path, opts, note)?;
        Ok(ExportResult {
            path: out_path.to_string_lossy().to_string(),
            skipped: Vec::new(),
        })
    }
}

fn export_drakepack(
    instance: &Instance,
    items: Vec<ContentItem>,
    game_dir: &Path,
    out_path: &Path,
    opts: zip::write::SimpleFileOptions,
    note: Option<String>,
) -> Result<()> {
    let content: Vec<SnapshotContent> = items
        .into_iter()
        .map(|i| SnapshotContent {
            source: i.source,
            project_type: i.project_type,
            project_id: i.project_id,
            version_id: i.version_id,
            file_name: i.file_name,
            title: i.title,
            icon_url: i.icon_url,
            enabled: i.enabled,
        })
        .collect();

    let index = SnapshotIndex {
        format_version: FORMAT_VERSION,
        name: instance.name.clone(),
        mc_version: instance.mc_version.clone(),
        loader: loader_str(instance.loader).to_string(),
        loader_version: instance.loader_version.clone(),
        content,
        note,
    };

    let mut zip = zip::ZipWriter::new(std::fs::File::create(out_path)?);
    zip.start_file("index.json", opts)
        .map_err(|e| AppError::Other(format!("zip: {e}")))?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
    add_overrides(&mut zip, opts, game_dir)?;
    zip.finish().map_err(|e| AppError::Other(format!("zip: {e}")))?;
    Ok(())
}

async fn export_mrpack(
    instance: &Instance,
    items: &[ContentItem],
    game_dir: &Path,
    out_path: &Path,
    opts: zip::write::SimpleFileOptions,
) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let mut skipped = Vec::new();

    for item in items {
        // .mrpack references files by direct URL + hash; only Modrinth content
        // resolves cleanly (CurseForge opt-outs / missing hashes can't).
        if item.source != "modrinth" {
            skipped.push(item.title.clone());
            continue;
        }
        let version = match crate::sources::get_version(Source::Modrinth, &item.version_id).await {
            Ok(v) => v,
            Err(_) => {
                skipped.push(item.title.clone());
                continue;
            }
        };
        let Some(file) = version.primary_file() else {
            skipped.push(item.title.clone());
            continue;
        };
        if file.url.is_empty() || (file.hashes.sha1.is_none() && file.hashes.sha512.is_none()) {
            skipped.push(item.title.clone());
            continue;
        }
        files.push(MrFile {
            path: format!("{}/{}", subdir_for(&item.project_type), file.filename),
            hashes: MrHashes {
                sha1: file.hashes.sha1.clone(),
                sha512: file.hashes.sha512.clone(),
            },
            downloads: vec![file.url.clone()],
            file_size: file.size,
            env: Some(MrEnv {
                client: "required".into(),
                server: "optional".into(),
            }),
        });
    }

    let mut dependencies = std::collections::HashMap::new();
    dependencies.insert("minecraft".to_string(), instance.mc_version.clone());
    if let Some(key) = mrpack_loader_key(instance.loader) {
        dependencies.insert(
            key.to_string(),
            instance.loader_version.clone().unwrap_or_default(),
        );
    }

    let index = MrIndex {
        format_version: 1,
        game: "minecraft".into(),
        version_id: Some("1.0.0".into()),
        name: Some(instance.name.clone()),
        files,
        dependencies,
    };

    let mut zip = zip::ZipWriter::new(std::fs::File::create(out_path)?);
    zip.start_file("modrinth.index.json", opts)
        .map_err(|e| AppError::Other(format!("zip: {e}")))?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
    add_overrides(&mut zip, opts, game_dir)?;
    zip.finish().map_err(|e| AppError::Other(format!("zip: {e}")))?;
    Ok(skipped)
}

fn mrpack_loader_key(loader: ModLoader) -> Option<&'static str> {
    match loader {
        ModLoader::Fabric => Some("fabric-loader"),
        ModLoader::Quilt => Some("quilt-loader"),
        ModLoader::Forge => Some("forge"),
        ModLoader::NeoForge => Some("neoforge"),
        ModLoader::Vanilla => None,
    }
}

/// Export an instance and upload it to the hosted service as the caller's
/// current snapshot. Returns the new snapshot id.
#[allow(clippy::too_many_arguments)]
pub async fn publish(
    app: &AppHandle,
    instance_id: &str,
    format: &str,
    api_base: &str,
    access_token: &str,
    board_handle: Option<String>,
    name: Option<String>,
    changelog: Option<String>,
) -> Result<String> {
    let instance = app
        .state::<InstanceStore>()
        .get(instance_id)
        .ok_or_else(|| AppError::InstanceNotFound(instance_id.to_string()))?;

    let exported = export(app, instance_id, format, changelog.clone()).await?;
    let bytes = std::fs::read(&exported.path)?;

    let base = api_base.trim_end_matches('/');
    let loader = loader_str(instance.loader);
    let loader_version = instance.loader_version.clone().unwrap_or_default();
    let display_name = name.unwrap_or_else(|| instance.name.clone());
    let mut params: Vec<(&str, &str)> = vec![
        ("format", format),
        ("name", &display_name),
        ("mcVersion", &instance.mc_version),
        ("modLoader", loader),
        ("modLoaderVersion", &loader_version),
    ];
    if let Some(bh) = board_handle.as_deref().filter(|s| !s.is_empty()) {
        params.push(("boardHandle", bh));
    }
    if let Some(ch) = changelog.as_deref().filter(|c| !c.is_empty()) {
        params.push(("changelog", ch));
    }

    let resp = reqwest::Client::new()
        .post(format!("{base}/v1/snapshots"))
        .query(&params)
        .bearer_auth(access_token)
        .header("Content-Type", "application/zip")
        .body(bytes)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("publish failed ({status}): {text}")));
    }

    let json: serde_json::Value = resp.json().await?;
    json.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Other("no snapshot id in response".into()))
}

// --- Import -----------------------------------------------------------------

/// Import a snapshot (raw bytes of a `.drakepack` or `.mrpack`) as a new
/// instance. The format is detected from the archive contents.
pub async fn import(app: &AppHandle, bytes: Vec<u8>) -> Result<ImportResult> {
    // Stash the upload so we can open it as a zip (and re-open for overrides).
    let tmp = paths::meta_dir(app)?.join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S%f");
    let pack_path = tmp.join(format!("import-{ts}.pack"));
    std::fs::write(&pack_path, &bytes)?;

    let result = if has_entry(&pack_path, "index.json")? {
        import_drakepack(app, &pack_path).await
    } else if has_entry(&pack_path, "modrinth.index.json")? {
        import_mrpack(app, &pack_path).await
    } else {
        Err(AppError::Other(
            "unrecognized pack — expected a .drakepack or .mrpack".into(),
        ))
    };

    let _ = std::fs::remove_file(&pack_path);
    result
}

/// Whether a zip contains an entry with the exact given name.
fn has_entry(pack: &Path, name: &str) -> Result<bool> {
    let f = std::fs::File::open(pack)?;
    let zip = zip::ZipArchive::new(f).map_err(|e| AppError::Other(format!("bad zip: {e}")))?;
    let found = zip.file_names().any(|n| n == name);
    Ok(found)
}

async fn import_drakepack(app: &AppHandle, pack_path: &Path) -> Result<ImportResult> {
    let index = read_index(pack_path)?;

    let instance = Instance::new(
        index.name.clone(),
        index.mc_version.clone(),
        parse_loader(&index.loader),
        index.loader_version.clone(),
        None,
    );
    app.state::<InstanceStore>().save(app, &instance)?;

    // Resolve + install each content ref through the shared cache. Sequential so
    // the per-instance content.json stays consistent; failures are collected.
    let total = index.content.len();
    emit(app, "resolving", 0, total);
    let mut installed = 0usize;
    let mut skipped = Vec::new();
    for (i, c) in index.content.iter().enumerate() {
        match content::install(
            app,
            &instance.id,
            parse_source(&c.source),
            &c.version_id,
            &c.project_type,
            &c.title,
            c.icon_url.clone(),
        )
        .await
        {
            Ok(_) => installed += 1,
            Err(_) => skipped.push(c.title.clone()),
        }
        emit(app, "installing", i + 1, total);
    }

    let game_dir = paths::instance_game_dir(app, &instance.id)?;
    extract_overrides(pack_path, &game_dir)?;
    emit(app, "done", total, total);

    Ok(ImportResult {
        instance,
        installed,
        skipped,
    })
}

async fn import_mrpack(app: &AppHandle, pack_path: &Path) -> Result<ImportResult> {
    let index = read_mr_index(pack_path)?;

    let mc = index
        .dependencies
        .get("minecraft")
        .cloned()
        .unwrap_or_default();
    let (loader, loader_version) = mr_loader_from_deps(&index.dependencies);
    let name = index.name.clone().unwrap_or_else(|| "Imported pack".into());

    let instance = Instance::new(name, mc, loader, loader_version, None);
    app.state::<InstanceStore>().save(app, &instance)?;
    let game_dir = paths::instance_game_dir(app, &instance.id)?;

    // Client-relevant files -> download tasks (deduped through the cache).
    let tasks: Vec<DownloadTask> = index
        .files
        .iter()
        .filter(|f| {
            f.env
                .as_ref()
                .map(|e| e.client != "unsupported")
                .unwrap_or(true)
        })
        .filter_map(|f| {
            let rel = safe_rel(&f.path)?;
            let url = f.downloads.first()?.clone();
            Some(DownloadTask {
                url,
                dest: game_dir.join(rel),
                sha1: f.hashes.sha1.clone(),
                executable: false,
            })
        })
        .collect();

    let total = tasks.len();
    emit(app, "installing", 0, total);
    let client = crate::modrinth::client()?;
    let app_cb = app.clone();
    content::cache::install_all(&client, app, tasks, 12, move |cur, t| {
        emit(&app_cb, "installing", cur, t);
    })
    .await?;

    extract_overrides(pack_path, &game_dir)?;
    emit(app, "done", total, total);

    Ok(ImportResult {
        instance,
        installed: total,
        skipped: Vec::new(),
    })
}

fn read_index(pack: &Path) -> Result<SnapshotIndex> {
    let text = read_zip_text(pack, "index.json")?;
    serde_json::from_str(&text).map_err(|e| AppError::Other(format!("bad snapshot index: {e}")))
}

fn read_mr_index(pack: &Path) -> Result<MrIndex> {
    let text = read_zip_text(pack, "modrinth.index.json")?;
    serde_json::from_str(&text).map_err(|e| AppError::Other(format!("bad .mrpack index: {e}")))
}

fn read_zip_text(pack: &Path, entry: &str) -> Result<String> {
    let f = std::fs::File::open(pack)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| AppError::Other(format!("bad zip: {e}")))?;
    let mut e = zip
        .by_name(entry)
        .map_err(|_| AppError::Other(format!("missing {entry}")))?;
    let mut s = String::new();
    e.read_to_string(&mut s)?;
    Ok(s)
}

fn mr_loader_from_deps(
    deps: &std::collections::HashMap<String, String>,
) -> (ModLoader, Option<String>) {
    for (key, loader) in [
        ("fabric-loader", ModLoader::Fabric),
        ("quilt-loader", ModLoader::Quilt),
        ("neoforge", ModLoader::NeoForge),
        ("forge", ModLoader::Forge),
    ] {
        if let Some(v) = deps.get(key) {
            return (loader, (!v.is_empty()).then(|| v.clone()));
        }
    }
    (ModLoader::Vanilla, None)
}

// --- Modrinth .mrpack index (subset) ----------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrIndex {
    format_version: u32,
    game: String,
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    files: Vec<MrFile>,
    #[serde(default)]
    dependencies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrFile {
    path: String,
    #[serde(default)]
    hashes: MrHashes,
    downloads: Vec<String>,
    #[serde(default)]
    file_size: u64,
    #[serde(default)]
    env: Option<MrEnv>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MrHashes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sha512: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MrEnv {
    client: String,
    server: String,
}

/// Extract the `overrides/` tree of a pack into an instance's game dir.
fn extract_overrides(pack: &Path, game_dir: &Path) -> Result<()> {
    let f = std::fs::File::open(pack)?;
    let mut zip =
        zip::ZipArchive::new(f).map_err(|e| AppError::Other(format!("bad zip: {e}")))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| AppError::Other(format!("bad zip entry: {e}")))?;
        let name = entry.name().to_string();
        let Some(rel) = name
            .strip_prefix("overrides/")
            .or_else(|| name.strip_prefix("client-overrides/"))
        else {
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

/// Reject absolute paths and `..` traversal in archive entry names.
fn safe_rel(path: &str) -> Option<PathBuf> {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        return None;
    }
    for comp in p.components() {
        if matches!(comp, std::path::Component::ParentDir | std::path::Component::Prefix(_)) {
            return None;
        }
    }
    Some(p)
}
