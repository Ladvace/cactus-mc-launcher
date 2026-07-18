//! Adaptive "Tune-up" — inspect the host machine and an instance's loader +
//! Minecraft version, then recommend a performance mod set, heap size, and JVM
//! flags tailored to the detected hardware.
//!
//! Unlike a fixed "FPS boost" bundle, the recommendation *adapts*: the heap
//! scales to the machine's RAM (with OS headroom), Aikar's G1GC flags kick in
//! only for larger heaps, and every mod is surfaced with a plain-language reason
//! so the user can see (and toggle) exactly what will be applied. It also works
//! on any existing instance rather than forcing a separate client.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::content;
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::ModLoader;
use crate::modrinth;
use crate::sources::Source;

/// Detected host machine specs, surfaced to the UI so the user can see what the
/// recommendation is based on.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSpecs {
    pub total_ram_mb: u64,
    pub cpu_cores: u32,
    pub os: String,
    pub arch: String,
}

/// A single recommended mod, resolved to a concrete Modrinth version.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModRec {
    pub slug: String,
    pub version_id: String,
    pub title: String,
    pub reason: String,
    /// Suggested default (all core mods are on by default; deps included).
    pub recommended: bool,
    /// True if a version of this project is already installed in the instance.
    pub installed: bool,
}

/// The full recommendation returned by [`recommend`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TuneupPlan {
    pub specs: HostSpecs,
    pub mc_version: String,
    pub loader: String,
    pub max_mem_mb: u32,
    pub min_mem_mb: u32,
    pub jvm_args: String,
    pub mods: Vec<ModRec>,
    /// Slugs that have no build for this loader/version (shown as skipped).
    pub unavailable: Vec<String>,
}

/// What the user chose to apply, sent back from the UI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuneupSelection {
    /// Modrinth version ids the user kept ticked, each with its display title.
    pub mods: Vec<SelectedMod>,
    pub apply_memory: bool,
    pub apply_flags: bool,
    pub max_mem_mb: u32,
    pub min_mem_mb: u32,
    pub jvm_args: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedMod {
    pub version_id: String,
    pub title: String,
}

/// Detect the host machine's RAM, core count, OS and architecture.
pub fn detect_specs() -> HostSpecs {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    HostSpecs {
        // sysinfo reports bytes.
        total_ram_mb: sys.total_memory() / 1024 / 1024,
        cpu_cores: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// Aikar's well-known G1GC flags — worthwhile once the heap is large enough that
/// GC pauses matter. Applied only for heaps >= 6 GB (below that they add little).
const AIKAR_FLAGS: &str = "-XX:+UseG1GC -XX:+ParallelRefProcEnabled \
-XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC \
-XX:+AlwaysPreTouch -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 \
-XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 \
-XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 \
-XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 \
-XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1";

/// Conservative, headroom-aware heap: ~half of RAM, floor 2 GB, cap 8 GB
/// (vanilla + performance mods rarely benefit beyond that), rounded to 512 MB.
/// Returns `(min_mb, max_mb, jvm_args)`.
fn recommend_heap(total_ram_mb: u64) -> (u32, u32, String) {
    let half = (total_ram_mb / 2) as u32;
    let max = (half.clamp(2048, 8192) / 512) * 512;
    let min = (max / 2).max(1024);
    let jvm_args = if max >= 6144 {
        AIKAR_FLAGS.to_string()
    } else {
        // Small heaps: just pick G1 explicitly; skip the tuning knobs.
        "-XX:+UseG1GC".to_string()
    };
    (min, max, jvm_args)
}

/// The Modrinth loader string for a mod loader (Vanilla has no mods).
fn loader_str(loader: ModLoader) -> Option<&'static str> {
    match loader {
        ModLoader::Vanilla => None,
        ModLoader::Fabric => Some("fabric"),
        ModLoader::Quilt => Some("quilt"),
        ModLoader::Forge => Some("forge"),
        ModLoader::NeoForge => Some("neoforge"),
    }
}

/// Extra mods for the "visuals" mode — shader support on top of the core set.
/// (Sodium/Embeddium from the core set are the required rendering base.)
fn visuals_mods(loader: ModLoader) -> Vec<(&'static str, &'static str)> {
    match loader {
        ModLoader::Vanilla => vec![],
        ModLoader::Fabric | ModLoader::Quilt => {
            vec![("iris", "Shader support (runs on top of Sodium)")]
        }
        ModLoader::Forge | ModLoader::NeoForge => {
            vec![("oculus", "Shader support for Forge/NeoForge (Iris port)")]
        }
    }
}

/// Curated core performance mods per loader family, as `(slug, reason)`. These
/// are all open-source, safe (no visual change, no anti-cheat concerns), and
/// resolved against the instance's exact loader + Minecraft version at runtime.
fn core_mods(loader: ModLoader) -> Vec<(&'static str, &'static str)> {
    match loader {
        ModLoader::Vanilla => vec![],
        ModLoader::Fabric | ModLoader::Quilt => vec![
            // Fabric API is a hard dependency of most Fabric mods; Quilt loads it
            // via its Fabric-compat layer, so include it either way.
            ("fabric-api", "Required library most Fabric mods depend on"),
            ("sodium", "Modern rendering engine — the biggest FPS win"),
            ("lithium", "Optimizes game logic/physics; helps CPU-bound worlds"),
            ("ferrite-core", "Cuts memory use by de-duplicating textures"),
            ("entityculling", "Skips rendering entities you can't see"),
            ("immediatelyfast", "Speeds up text/HUD/font rendering"),
            ("dynamic-fps", "Drops FPS when the window is unfocused to save power"),
        ],
        ModLoader::Forge | ModLoader::NeoForge => vec![
            ("embeddium", "Sodium's rendering engine, ported to Forge/NeoForge"),
            ("ferrite-core", "Cuts memory use by de-duplicating textures"),
            ("entityculling", "Skips rendering entities you can't see"),
            ("dynamic-fps", "Drops FPS when the window is unfocused to save power"),
        ],
    }
}

/// Build a tailored tune-up recommendation for an instance. `mode` is
/// `"performance"` (core mods only) or `"visuals"` (core + shader support).
pub async fn recommend(app: &AppHandle, instance_id: &str, mode: &str) -> Result<TuneupPlan> {
    let instance = app
        .state::<InstanceStore>()
        .get(instance_id)
        .ok_or_else(|| AppError::InstanceNotFound(instance_id.to_string()))?;

    let specs = detect_specs();
    let (min_mem_mb, max_mem_mb, jvm_args) = recommend_heap(specs.total_ram_mb);

    let mut mods = Vec::new();
    let mut unavailable = Vec::new();

    // Projects already installed in this instance, so we don't recommend adding
    // them again (matched by project, not exact version).
    let installed_ids: std::collections::HashSet<String> = content::list(app, instance_id)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.project_id)
        .collect();

    if let Some(loader) = loader_str(instance.loader) {
        let mut wanted = core_mods(instance.loader);
        if mode == "visuals" {
            wanted.extend(visuals_mods(instance.loader));
        }
        for (slug, reason) in wanted {
            match modrinth::get_versions(slug, Some(loader), Some(&instance.mc_version)).await {
                Ok(versions) => match versions.into_iter().next() {
                    // Modrinth returns newest-first; take the latest compatible.
                    Some(version) => {
                        let installed = installed_ids.contains(&version.project_id);
                        mods.push(ModRec {
                            slug: slug.to_string(),
                            version_id: version.id,
                            title: friendly_title(slug),
                            reason: reason.to_string(),
                            recommended: !installed,
                            installed,
                        });
                    }
                    None => unavailable.push(friendly_title(slug)),
                },
                Err(_) => unavailable.push(friendly_title(slug)),
            }
        }
    }

    Ok(TuneupPlan {
        specs,
        mc_version: instance.mc_version,
        loader: loader_str(instance.loader).unwrap_or("vanilla").to_string(),
        max_mem_mb,
        min_mem_mb,
        jvm_args,
        mods,
        unavailable,
    })
}

/// Apply the user's selected tune-up: install the chosen mods and, if opted in,
/// set the instance's heap and JVM flags. Returns the number of mods installed.
pub async fn apply(app: &AppHandle, instance_id: &str, selection: TuneupSelection) -> Result<usize> {
    let mut installed = 0;
    for chosen in &selection.mods {
        // Already-present mods are overwritten harmlessly by the deduped cache.
        content::install(
            app,
            instance_id,
            Source::Modrinth,
            &chosen.version_id,
            "mod",
            &chosen.title,
            None,
        )
        .await?;
        installed += 1;
    }

    if selection.apply_memory || selection.apply_flags {
        let store = app.state::<InstanceStore>();
        let mut instance = store
            .get(instance_id)
            .ok_or_else(|| AppError::InstanceNotFound(instance_id.to_string()))?;
        if selection.apply_memory {
            instance.max_memory_mb = Some(selection.max_mem_mb);
            instance.min_memory_mb = Some(selection.min_mem_mb);
        }
        if selection.apply_flags {
            instance.jvm_args = Some(selection.jvm_args.clone());
        }
        store.save(app, &instance)?;
    }

    Ok(installed)
}

/// Display name for a curated slug (avoids an extra project fetch per mod).
fn friendly_title(slug: &str) -> String {
    match slug {
        "fabric-api" => "Fabric API",
        "sodium" => "Sodium",
        "lithium" => "Lithium",
        "ferrite-core" => "FerriteCore",
        "entityculling" => "Entity Culling",
        "immediatelyfast" => "ImmediatelyFast",
        "dynamic-fps" => "Dynamic FPS",
        "embeddium" => "Embeddium",
        "iris" => "Iris Shaders",
        "oculus" => "Oculus (Shaders)",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_is_conservative_and_capped() {
        // 8 GB machine → ~4 GB heap, light GC.
        let (min, max, args) = recommend_heap(8192);
        assert_eq!(max, 4096);
        assert_eq!(min, 2048);
        assert_eq!(args, "-XX:+UseG1GC");

        // 32 GB machine → capped at 8 GB with Aikar's flags.
        let (min, max, args) = recommend_heap(32768);
        assert_eq!(max, 8192);
        assert_eq!(min, 4096);
        assert!(args.contains("G1GC") && args.contains("MaxGCPauseMillis"));

        // Tiny machine → floored at 2 GB.
        let (_, max, _) = recommend_heap(3000);
        assert_eq!(max, 2048);
    }

    #[test]
    fn vanilla_has_no_mods() {
        assert!(core_mods(ModLoader::Vanilla).is_empty());
        assert!(loader_str(ModLoader::Vanilla).is_none());
    }

    #[test]
    fn fabric_includes_fabric_api_first() {
        let mods = core_mods(ModLoader::Fabric);
        assert_eq!(mods[0].0, "fabric-api");
        assert!(mods.iter().any(|(slug, _)| *slug == "sodium"));
    }
}
