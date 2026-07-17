use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::AppHandle;

use super::download::{download_all, DownloadTask};
use crate::error::{AppError, Result};
use crate::minecraft::version::JavaVersion;
use crate::paths;

const JAVA_RUNTIME_MANIFEST: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Components we pre-install for the "auto-setup" button, covering every
/// modern Minecraft version (Java 8, 17, and 21).
pub const COMMON_COMPONENTS: [&str; 3] =
    ["jre-legacy", "java-runtime-gamma", "java-runtime-delta"];

// platform -> component -> entries
type AllRuntimes = HashMap<String, HashMap<String, Vec<RuntimeEntry>>>;

#[derive(Debug, Deserialize)]
struct RuntimeEntry {
    manifest: ManifestRef,
    #[serde(default)]
    version: Option<RuntimeVersion>,
}

#[derive(Debug, Deserialize)]
struct RuntimeVersion {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ManifestRef {
    url: String,
}

#[derive(Debug, Deserialize)]
struct FilesManifest {
    files: HashMap<String, JavaFile>,
}

#[derive(Debug, Deserialize)]
struct JavaFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    executable: bool,
    #[serde(default)]
    downloads: Option<JavaFileDownloads>,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JavaFileDownloads {
    raw: RawDownload,
}

#[derive(Debug, Deserialize)]
struct RawDownload {
    url: String,
    sha1: String,
    #[serde(default)]
    size: u64,
}

/// The Mojang platform key for the current OS/arch. When `force_x64` is set on
/// Apple Silicon we pick the x86_64 runtime (run under Rosetta 2) so old
/// Minecraft versions whose LWJGL lacks arm64 natives can still launch.
fn platform_key(force_x64: bool) -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") if force_x64 => "mac-os",
        ("macos", "aarch64") => "mac-os-arm64",
        ("macos", _) => "mac-os",
        ("linux", "x86") => "linux-i386",
        ("linux", _) => "linux",
        ("windows", "aarch64") => "windows-arm64",
        ("windows", "x86") => "windows-x86",
        ("windows", _) => "windows-x64",
        _ => "linux",
    }
}

fn java_binary_name() -> &'static str {
    if cfg!(windows) {
        "bin/java.exe"
    } else {
        "bin/java"
    }
}

/// Find the `java` executable within an installed runtime directory.
fn locate_java(install_dir: &Path) -> Option<PathBuf> {
    let suffix = java_binary_name();
    for candidate in [
        install_dir.join(suffix),
        install_dir.join("jre.bundle/Contents/Home").join(suffix),
    ] {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// The Java major version a component provides. Known components are mapped
/// directly (the `1.8.0`-style legacy name would otherwise misparse); unknown
/// ones fall back to parsing the runtime's `version.name`.
fn component_major(component: &str, entry: &RuntimeEntry) -> Option<u32> {
    let known = match component {
        "jre-legacy" => Some(8),
        "java-runtime-alpha" => Some(16),
        "java-runtime-beta" | "java-runtime-gamma" | "java-runtime-gamma-snapshot" => Some(17),
        "java-runtime-delta" => Some(21),
        "java-runtime-epsilon" => Some(22),
        _ => None,
    };
    if known.is_some() {
        return known;
    }
    // Fallback: parse "17.0.8" -> 17, or "1.8.0_202" -> 8.
    let name = &entry.version.as_ref()?.name;
    let mut parts = name.split('.');
    let first = parts.next()?;
    if first == "1" {
        parts.next()?.parse().ok()
    } else {
        first.parse().ok()
    }
}

/// Rank used to break ties when several components provide the same major.
fn preference(component: &str) -> u8 {
    match component {
        "java-runtime-gamma" => 0,
        "java-runtime-delta" => 1,
        "java-runtime-beta" => 2,
        "java-runtime-epsilon" => 3,
        "jre-legacy" => 4,
        _ => 5,
    }
}

/// Choose the best available component for the current platform: the exact one
/// if present, otherwise the closest available runtime whose major version is at
/// least the requested one (falling back to the newest available). Returns
/// `(component_name, files_manifest_url)`.
fn pick_component(
    all: &AllRuntimes,
    required_component: &str,
    required_major: u32,
    force_x64: bool,
) -> Option<(String, String)> {
    let map = all.get(platform_key(force_x64))?;

    // Exact match, if that component actually has a build for this platform.
    if let Some(entries) = map.get(required_component) {
        if let Some(entry) = entries.first() {
            return Some((required_component.to_string(), entry.manifest.url.clone()));
        }
    }

    // Otherwise gather every non-empty candidate with a known major version.
    let mut candidates: Vec<(String, String, u32)> = map
        .iter()
        .filter(|(component, _)| component.as_str() != "minecraft-java-exe")
        .filter_map(|(component, entries)| {
            let entry = entries.first()?;
            let major = component_major(component, entry)?;
            Some((component.clone(), entry.manifest.url.clone(), major))
        })
        .collect();

    // Smallest sufficient major first; ties broken by component preference.
    candidates.sort_by_key(|(comp, _, major)| (*major, preference(comp)));

    candidates
        .iter()
        .find(|(_, _, major)| *major >= required_major)
        .or_else(|| candidates.last())
        .map(|(comp, url, _)| (comp.clone(), url.clone()))
}

async fn fetch_all(client: &reqwest::Client) -> Result<AllRuntimes> {
    Ok(client
        .get(JAVA_RUNTIME_MANIFEST)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Ensure a usable Java executable, returning its path.
///
/// Priority: an explicit configured path, then a managed runtime matching (or
/// compatible with) the version's required component, downloaded on first use.
pub async fn ensure_java<F>(
    app: &AppHandle,
    client: &reqwest::Client,
    required: &JavaVersion,
    configured: Option<&str>,
    force_x64: bool,
    on_progress: F,
) -> Result<PathBuf>
where
    F: FnMut(usize, usize),
{
    if let Some(path) = configured {
        let path = path.trim();
        if !path.is_empty() {
            let candidate = PathBuf::from(path);
            if candidate.exists() {
                return Ok(candidate);
            }
            return Err(AppError::Other(format!(
                "configured Java path does not exist: {path}"
            )));
        }
    }

    let all = fetch_all(client).await?;
    let (component, manifest_url) =
        pick_component(&all, &required.component, required.major_version, force_x64).ok_or_else(
            || {
                AppError::Other(format!(
                    "no managed Java available for {}. Set a Java path in Settings.",
                    platform_key(force_x64)
                ))
            },
        )?;

    let install_dir = paths::java_dir(app)?.join(&component).join(platform_key(force_x64));
    if let Some(java) = locate_java(&install_dir) {
        return Ok(java);
    }

    download_runtime(client, &manifest_url, &install_dir, on_progress).await?;

    locate_java(&install_dir).ok_or_else(|| {
        AppError::Other(format!(
            "downloaded Java runtime '{component}' but could not find its java executable"
        ))
    })
}

/// Pre-install the common runtimes for the auto-setup button. `on_progress`
/// receives (component_label, current, total) as each runtime downloads.
/// Returns the human-readable labels of the runtimes that are now installed.
pub async fn setup_common<F>(
    app: &AppHandle,
    client: &reqwest::Client,
    mut on_progress: F,
) -> Result<Vec<String>>
where
    F: FnMut(&str, usize, usize),
{
    let all = fetch_all(client).await?;
    let map = all
        .get(platform_key(false))
        .ok_or_else(|| AppError::Other(format!("no Java runtimes for {}", platform_key(false))))?;

    let mut installed = Vec::new();

    for component in COMMON_COMPONENTS {
        // Skip components with no build for this platform (arch fallback covers them).
        let Some(entry) = map.get(component).and_then(|entries| entries.first()) else {
            continue;
        };
        let major = component_major(component, entry).unwrap_or(0);
        let label = format!("Java {major}");

        let install_dir = paths::java_dir(app)?.join(component).join(platform_key(false));
        if locate_java(&install_dir).is_some() {
            installed.push(label);
            continue;
        }

        let label_for_cb = label.clone();
        download_runtime(client, &entry.manifest.url, &install_dir, |cur, total| {
            on_progress(&label_for_cb, cur, total);
        })
        .await?;
        installed.push(label);
    }

    Ok(installed)
}

async fn download_runtime<F>(
    client: &reqwest::Client,
    manifest_url: &str,
    install_dir: &Path,
    on_progress: F,
) -> Result<()>
where
    F: FnMut(usize, usize),
{
    let manifest: FilesManifest = client
        .get(manifest_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut tasks = Vec::new();
    let mut links: Vec<(PathBuf, String)> = Vec::new();

    for (rel, file) in &manifest.files {
        let dest = install_dir.join(rel);
        match file.kind.as_str() {
            "directory" => {
                std::fs::create_dir_all(&dest)?;
            }
            "file" => {
                if let Some(download) = &file.downloads {
                    tasks.push(DownloadTask {
                        url: download.raw.url.clone(),
                        dest,
                        sha1: Some(download.raw.sha1.clone()),
                        executable: file.executable,
                    });
                    let _ = download.raw.size;
                }
            }
            "link" => {
                if let Some(target) = &file.target {
                    links.push((dest, target.clone()));
                }
            }
            _ => {}
        }
    }

    download_all(client, tasks, 8, on_progress).await?;

    for (link, target) in links {
        if link.exists() {
            continue;
        }
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link)?;
        #[cfg(windows)]
        {
            let _ = &target;
            let _ = &link;
        }
    }

    Ok(())
}
