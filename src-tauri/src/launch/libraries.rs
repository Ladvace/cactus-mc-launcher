use std::collections::HashMap;
use std::path::PathBuf;

use tauri::AppHandle;

use super::download::DownloadTask;
use super::rules::{os_name, rules_allow};
use crate::error::Result;
use crate::minecraft::version::{Artifact, Library};
use crate::paths;

const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2/";

/// Convert a Maven coordinate (`group:artifact:version[:classifier][@ext]`) to a
/// relative repository path.
fn maven_to_path(name: &str) -> Option<String> {
    // Split off an optional "@extension" suffix (e.g. "@zip").
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e),
        None => (name, "jar"),
    };
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let file = match parts.get(3) {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!("{group}/{artifact}/{version}/{file}"))
}

fn join_url(base: &str, rel: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{rel}")
    } else {
        format!("{base}/{rel}")
    }
}

/// Queue a download for a vanilla-style artifact and return its local path.
fn push_artifact(
    downloads: &mut Vec<DownloadTask>,
    base: &std::path::Path,
    art: &Artifact,
) -> Option<PathBuf> {
    let rel = art.path.clone()?;
    let dest = base.join(&rel);
    downloads.push(DownloadTask {
        url: art.url.clone(),
        dest: dest.clone(),
        sha1: Some(art.sha1.clone()),
        executable: false,
    });
    Some(dest)
}

/// Result of resolving a version's libraries for the current OS/arch.
pub struct ResolvedLibraries {
    /// Jars to place on the classpath.
    pub classpath: Vec<PathBuf>,
    /// Everything to download (classpath jars + native jars).
    pub downloads: Vec<DownloadTask>,
    /// Native jars to extract, with their optional exclude prefixes.
    pub natives: Vec<(PathBuf, Option<Vec<String>>)>,
}

/// Resolve all applicable libraries into download tasks, a classpath, and a
/// list of native jars to extract.
///
/// Two native schemes exist:
/// - **Old (≤1.18):** a `natives` OS→classifier map with an `extract` directive.
///   These classifier jars are extracted into the per-instance natives dir.
/// - **New (1.19+):** `natives-<os>-<arch>` jars are plain artifacts gated by OS
///   rules (arch is encoded in the file name). They go on the classpath, where
///   LWJGL 3.3 loads the arch-appropriate binary at runtime — no extraction.
pub fn resolve(app: &AppHandle, libraries: &[Library]) -> Result<ResolvedLibraries> {
    let base = paths::libraries_dir(app)?;
    let no_features: HashMap<String, bool> = HashMap::new();

    let mut classpath = Vec::new();
    let mut downloads = Vec::new();
    let mut natives = Vec::new();

    for lib in libraries {
        if let Some(rules) = &lib.rules {
            if !rules_allow(rules, &no_features) {
                continue;
            }
        }

        let Some(dl) = &lib.downloads else {
            // Maven-style library (Fabric/Quilt/Forge): resolve from name + url.
            if let Some(rel) = maven_to_path(&lib.name) {
                let base_url = lib.url.as_deref().unwrap_or(MAVEN_CENTRAL);
                let dest = base.join(&rel);
                downloads.push(DownloadTask {
                    url: join_url(base_url, &rel),
                    dest: dest.clone(),
                    sha1: None,
                    executable: false,
                });
                classpath.push(dest);
            }
            continue;
        };

        // Old-scheme natives: `natives` map -> classifier artifact, to extract.
        if let (Some(natives_map), Some(classifiers)) = (&lib.natives, &dl.classifiers) {
            if let Some(key) = natives_map.get(os_name()) {
                let key = key.replace("${arch}", "64");
                if let Some(art) = classifiers.get(&key) {
                    if let Some(path) = push_artifact(&mut downloads, &base, art) {
                        natives.push((path, lib.extract.as_ref().and_then(|e| e.exclude.clone())));
                    }
                }
            }
        }

        // Main artifact always goes on the classpath (including new-scheme
        // natives jars — the OS rules above already filtered them).
        if let Some(art) = &dl.artifact {
            if let Some(path) = push_artifact(&mut downloads, &base, art) {
                classpath.push(path);
            }
        }
    }

    Ok(ResolvedLibraries {
        classpath,
        downloads,
        natives,
    })
}

/// Extract native binaries from a jar into `dest_dir`, skipping directories,
/// `META-INF`, class files, and any excluded prefixes.
pub fn extract_natives(
    jar: &PathBuf,
    dest_dir: &PathBuf,
    exclude: &Option<Vec<String>>,
) -> Result<()> {
    let file = std::fs::File::open(jar)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        crate::error::AppError::Other(format!("failed to open native jar {}: {e}", jar.display()))
    })?;

    std::fs::create_dir_all(dest_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| crate::error::AppError::Other(format!("bad zip entry: {e}")))?;
        let name = entry.name().to_string();

        if entry.is_dir() || name.ends_with('/') {
            continue;
        }
        if name.starts_with("META-INF/") || name.ends_with(".class") || name.ends_with(".git") {
            continue;
        }
        if let Some(excludes) = exclude {
            if excludes.iter().any(|p| name.starts_with(p)) {
                continue;
            }
        }

        // Flatten to the base filename to keep natives directly in dest_dir.
        let file_name = match std::path::Path::new(&name).file_name() {
            Some(n) => n.to_owned(),
            None => continue,
        };
        let out_path = dest_dir.join(file_name);
        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }

    Ok(())
}
