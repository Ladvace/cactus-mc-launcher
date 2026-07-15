use std::path::Path;

use serde::Deserialize;
use tauri::AppHandle;

use super::LoaderVersion;
use crate::error::{AppError, Result};
use crate::instance::ModLoader;
use crate::minecraft::version::{Arguments, Library, VersionDetail};
use crate::paths;

const FORGE_MAVEN: &str = "https://maven.minecraftforge.net";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases";

/// The maven-metadata URL listing every build of the loader.
fn metadata_url(loader: ModLoader) -> &'static str {
    match loader {
        ModLoader::Forge => {
            "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml"
        }
        _ => "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
    }
}

/// Extract every `<version>…</version>` value from a maven-metadata document
/// (avoids pulling in a full XML parser).
fn parse_metadata_versions(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<version>") {
        let after = &rest[start + "<version>".len()..];
        if let Some(end) = after.find("</version>") {
            out.push(after[..end].trim().to_string());
            rest = &after[end + "</version>".len()..];
        } else {
            break;
        }
    }
    out
}

/// For a Minecraft version, decide whether a raw metadata version belongs to it
/// and what the user-facing loader version string is.
///
/// - Forge metadata versions look like `1.20.1-47.2.0` → we strip the `mc-` prefix.
/// - NeoForge (1.20.2+) versions look like `21.1.66` where `21.1` maps to MC
///   `1.21.1` (and `.0` minor means no trailing patch, e.g. `21.0.x` → `1.21`).
fn match_version(loader: ModLoader, mc: &str, raw: &str) -> Option<String> {
    match loader {
        // Forge keeps the "<mc>-<forge>" scheme (both "1.20.1-47.4.21" and the
        // newer "26.2-65.0.4"), so stripping "<mc>-" works for all eras.
        ModLoader::Forge => raw.strip_prefix(&format!("{mc}-")).map(|v| v.to_string()),
        // NeoForge encodes the Minecraft version in its own version string:
        // old MC "1.X.Y" drops the leading "1." ("1.21.1" -> "21.1.x"), while
        // new MC like "26.2" is used as-is ("26.2" -> "26.2.0.x").
        _ => {
            let sig = mc.strip_prefix("1.").unwrap_or(mc);
            let prefix = format!("{sig}.");
            raw.starts_with(&prefix).then(|| raw.to_string())
        }
    }
}

/// List Forge/NeoForge builds for a Minecraft version (newest first).
pub async fn list_versions(loader: ModLoader, mc: &str) -> Result<Vec<LoaderVersion>> {
    let xml = reqwest::get(metadata_url(loader))
        .await?
        .error_for_status()?
        .text()
        .await?;

    let mut versions: Vec<LoaderVersion> = parse_metadata_versions(&xml)
        .iter()
        .filter_map(|raw| match_version(loader, mc, raw))
        .map(|v| LoaderVersion {
            stable: !v.contains("-beta"),
            version: v,
        })
        .collect();

    versions.reverse(); // metadata is ascending; show newest first
    Ok(versions)
}

async fn resolve_version(loader: ModLoader, mc: &str, requested: Option<&str>) -> Result<String> {
    if let Some(v) = requested {
        if !v.trim().is_empty() {
            return Ok(v.trim().to_string());
        }
    }
    let versions = list_versions(loader, mc).await?;
    versions
        .into_iter()
        .next()
        .map(|v| v.version)
        .ok_or_else(|| {
            AppError::Other(format!("no {loader:?} builds available for Minecraft {mc}"))
        })
}

/// URL of the `-installer.jar` for a build.
fn installer_url(loader: ModLoader, mc: &str, ver: &str) -> String {
    match loader {
        ModLoader::Forge => format!(
            "{FORGE_MAVEN}/net/minecraftforge/forge/{mc}-{ver}/forge-{mc}-{ver}-installer.jar"
        ),
        _ => format!("{NEOFORGE_MAVEN}/net/neoforged/neoforge/{ver}/neoforge-{ver}-installer.jar"),
    }
}

/// The profile subset we merge into the vanilla version detail.
#[derive(Debug, Deserialize)]
struct ForgeProfile {
    #[serde(rename = "mainClass")]
    main_class: String,
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(default)]
    arguments: Option<Arguments>,
}

/// Ensure the loader is installed (running the official installer once) and
/// merge its profile into `detail`. Returns the resolved loader version.
pub async fn apply(
    detail: &mut VersionDetail,
    app: &AppHandle,
    client: &reqwest::Client,
    java: &Path,
    loader: ModLoader,
    mc: &str,
    requested: Option<&str>,
) -> Result<String> {
    let ver = resolve_version(loader, mc, requested).await?;
    let profile = ensure_installed(app, client, java, loader, mc, &ver).await?;

    detail.main_class = profile.main_class;

    let mut libs = profile.libraries;
    libs.append(&mut detail.libraries);
    detail.libraries = libs;

    if let Some(extra) = profile.arguments {
        let args = detail.arguments.get_or_insert_with(Arguments::default);
        args.jvm.extend(extra.jvm);
        args.game.extend(extra.game);
    }

    Ok(ver)
}

/// Cached profile id (also the installer's `versions/<id>` folder name).
fn profile_id(loader: ModLoader, mc: &str, ver: &str) -> String {
    match loader {
        ModLoader::Forge => format!("{mc}-forge-{ver}"),
        _ => format!("neoforge-{ver}"),
    }
}

async fn ensure_installed(
    app: &AppHandle,
    client: &reqwest::Client,
    java: &Path,
    loader: ModLoader,
    mc: &str,
    ver: &str,
) -> Result<ForgeProfile> {
    let id = profile_id(loader, mc, ver);
    let cache = paths::version_dir(app, &id)?.join(format!("{id}.json"));

    // Reuse a previous install.
    if cache.exists() {
        if let Ok(text) = std::fs::read_to_string(&cache) {
            if let Ok(profile) = serde_json::from_str::<ForgeProfile>(&text) {
                return Ok(profile);
            }
        }
    }

    // Download the installer.
    let tmp = paths::meta_dir(app)?.join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let installer = tmp.join(format!("{id}-installer.jar"));
    let bytes = client
        .get(installer_url(loader, mc, ver))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    std::fs::write(&installer, &bytes)?;

    // A `.minecraft`-shaped root the installer writes into.
    let root = paths::meta_dir(app)?.join("forge-install");
    std::fs::create_dir_all(root.join("versions"))?;
    std::fs::create_dir_all(root.join("libraries"))?;
    let profiles_file = root.join("launcher_profiles.json");
    if !profiles_file.exists() {
        std::fs::write(&profiles_file, "{\"profiles\":{}}")?;
    }

    // Run the official installer headlessly (it performs the processor steps).
    let output = tokio::process::Command::new(java)
        .arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(&root)
        .current_dir(&root)
        .output()
        .await
        .map_err(|e| AppError::Other(format!("failed to run {loader:?} installer: {e}")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail: String = err.chars().rev().take(600).collect::<String>().chars().rev().collect();
        return Err(AppError::Other(format!(
            "{loader:?} installer failed: {tail}"
        )));
    }

    // Locate the generated version JSON.
    let (found_id, text) = find_version_json(&root, &id, ver)?;

    // Make the installed libraries available to our shared library tree.
    copy_dir(&root.join("libraries"), &paths::libraries_dir(app)?)?;

    // Cache under the id we look up by.
    std::fs::write(&cache, &text)?;
    let _ = found_id;
    let _ = std::fs::remove_file(&installer);

    serde_json::from_str::<ForgeProfile>(&text)
        .map_err(|e| AppError::Other(format!("could not parse {loader:?} profile: {e}")))
}

/// Find the version JSON the installer produced under `root/versions`.
fn find_version_json(root: &Path, expected_id: &str, ver: &str) -> Result<(String, String)> {
    let versions = root.join("versions");

    // Preferred: the exact folder we expect.
    let direct = versions.join(expected_id).join(format!("{expected_id}.json"));
    if direct.exists() {
        return Ok((expected_id.to_string(), std::fs::read_to_string(direct)?));
    }

    // Fallback: any version folder whose name contains the loader version.
    if let Ok(entries) = std::fs::read_dir(&versions) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(ver) {
                let json = entry.path().join(format!("{name}.json"));
                if json.exists() {
                    return Ok((name, std::fs::read_to_string(json)?));
                }
            }
        }
    }

    Err(AppError::Other(
        "the installer did not produce a version profile".into(),
    ))
}

/// Recursively copy files from `src` into `dst`, skipping ones that already exist.
fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir(&path, &target)?;
        } else if !target.exists() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
