//! Dedicated-server launch pipeline. Much smaller than the client path: a
//! server needs Java, a server jar (per loader), an accepted EULA, and a run
//! directory — no assets, natives, LWJGL, accounts or window arguments.

use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::{download, java, process};
use crate::error::{AppError, Result};
use crate::instance::{Instance, ModLoader};
use crate::minecraft::version::JavaVersion;
use crate::minecraft::{self, version};
use crate::paths;
use crate::settings::Settings;

/// Prepare and start a dedicated server for `instance`.
pub async fn launch(app: AppHandle, instance: Instance, settings: Settings) -> Result<()> {
    let id = instance.id.clone();
    let result = prepare_and_spawn(&app, &instance, &settings).await;
    if let Err(e) = &result {
        eprintln!("[server] error for instance {id}: {e}");
        super::emit_status(&app, &id, "error", Some(e.to_string()));
    }
    result
}

async fn prepare_and_spawn(app: &AppHandle, instance: &Instance, settings: &Settings) -> Result<()> {
    let id = &instance.id;
    super::emit_status(app, id, "preparing", Some("Resolving version…".into()));

    let client = reqwest::Client::builder()
        .user_agent(concat!("cactus-launcher/", env!("CARGO_PKG_VERSION")))
        .build()?;

    // Version metadata (for the Java requirement and the vanilla server jar).
    let manifest = minecraft::fetch_versions().await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == instance.mc_version)
        .ok_or_else(|| {
            AppError::Other(format!("Minecraft version '{}' not found", instance.mc_version))
        })?;
    let detail = version::fetch_detail(app, &entry.id, &entry.url).await?;

    // Java (servers don't use LWJGL, so no Rosetta special-casing).
    super::emit_status(app, id, "preparing", Some("Preparing Java runtime…".into()));
    let java_version = detail.java_version.clone().unwrap_or(JavaVersion {
        component: "jre-legacy".into(),
        major_version: 8,
    });
    let java = {
        let app_cb = app.clone();
        let id_cb = id.clone();
        java::ensure_java(
            app,
            &client,
            &java_version,
            settings.java_path.as_deref(),
            false,
            move |cur, total| super::emit_progress(&app_cb, &id_cb, "java", cur, total),
        )
        .await?
    };

    // The server runs out of the instance's game directory, so mods installed
    // via the Content tab (which target `<game>/mods`) are picked up.
    let run_dir = paths::instance_game_dir(app, id)?;
    write_eula(&run_dir)?;

    // Build the loader-specific launch args (everything after the JVM args).
    super::emit_status(app, id, "downloading", Some("Preparing server files…".into()));
    let launch_args: Vec<String> = match instance.loader {
        ModLoader::Vanilla => prepare_vanilla(&client, &detail, &run_dir).await?,
        ModLoader::Fabric | ModLoader::Quilt => {
            super::emit_status(
                app,
                id,
                "preparing",
                Some("Installing server (first run can take a minute)…".into()),
            );
            prepare_fabric_like(&client, &java, instance, &run_dir).await?
        }
        ModLoader::Forge | ModLoader::NeoForge => {
            super::emit_status(
                app,
                id,
                "preparing",
                Some("Installing server (first run can take a minute)…".into()),
            );
            crate::loader::forge::install_server(
                &client,
                &java,
                instance.loader,
                &instance.mc_version,
                instance.loader_version.as_deref(),
                &run_dir,
            )
            .await?
        }
    };

    // JVM/memory args go first, then the loader-specific args.
    let mut args = memory_args(settings, instance.server_memory_mb);
    args.extend(launch_args);

    super::emit_status(app, id, "launching", Some("Starting server…".into()));
    process::spawn_server(app.clone(), java, args, run_dir, id.clone())?;
    Ok(())
}

/// `-Xms/-Xmx` plus any user-configured extra JVM args. A per-server memory
/// override (`server_mem`) wins over the global setting; when set, `-Xms` is
/// pinned to the same value (common practice for servers).
fn memory_args(settings: &Settings, server_mem: Option<u32>) -> Vec<String> {
    let (min, max) = match server_mem {
        Some(m) => (m.max(512), m.max(512)),
        None => (settings.min_memory_mb.max(512), settings.max_memory_mb.max(1024)),
    };
    let mut args = vec![format!("-Xms{min}M"), format!("-Xmx{max}M")];
    args.extend(settings.jvm_args.split_whitespace().map(String::from));
    args
}

/// Vanilla: download the official server jar and run it directly.
async fn prepare_vanilla(
    client: &reqwest::Client,
    detail: &version::VersionDetail,
    run_dir: &Path,
) -> Result<Vec<String>> {
    let server = detail.downloads.server.as_ref().ok_or_else(|| {
        AppError::Other(format!(
            "Minecraft {} has no downloadable server jar.",
            detail.id
        ))
    })?;
    let jar = run_dir.join("server.jar");
    download::download_all(
        client,
        vec![download::DownloadTask {
            url: server.url.clone(),
            dest: jar.clone(),
            sha1: Some(server.sha1.clone()),
            executable: false,
        }],
        1,
        |_, _| {},
    )
    .await?;
    Ok(vec!["-jar".into(), path_str(&jar), "nogui".into()])
}

/// Fabric/Quilt: run the official installer to set up a server (downloads the
/// vanilla server + builds the launch jar). More robust than the meta
/// `/server/jar` shortcut, which 404s for some game/loader combinations.
async fn prepare_fabric_like(
    client: &reqwest::Client,
    java: &Path,
    instance: &Instance,
    run_dir: &Path,
) -> Result<Vec<String>> {
    let launch_name = match instance.loader {
        ModLoader::Quilt => "quilt-server-launch.jar",
        _ => "fabric-server-launch.jar",
    };
    let launch_jar = run_dir.join(launch_name);

    // Reuse a previous install.
    if launch_jar.exists() {
        return Ok(vec!["-jar".into(), path_str(&launch_jar), "nogui".into()]);
    }

    let loader_version = crate::loader::resolve_loader_version(
        instance.loader,
        &instance.mc_version,
        instance.loader_version.as_deref(),
    )
    .await?;

    // Fetch the latest installer download URL from the loader's meta API.
    let installer_meta = match instance.loader {
        ModLoader::Quilt => "https://meta.quiltmc.org/v3/versions/installer",
        _ => "https://meta.fabricmc.net/v2/versions/installer",
    };
    #[derive(serde::Deserialize)]
    struct InstallerEntry {
        url: String,
    }
    let installers: Vec<InstallerEntry> = client
        .get(installer_meta)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let installer_url = installers
        .first()
        .map(|i| i.url.clone())
        .ok_or_else(|| AppError::Other("no installer available".into()))?;

    let installer = run_dir.join(".loader-installer.jar");
    let bytes = client
        .get(&installer_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    std::fs::write(&installer, &bytes)?;

    // Installer CLIs differ between Fabric and Quilt.
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-jar").arg(&installer).current_dir(run_dir);
    match instance.loader {
        ModLoader::Quilt => {
            cmd.arg("install")
                .arg("server")
                .arg(&instance.mc_version)
                .arg(&loader_version)
                .arg(format!("--install-dir={}", run_dir.display()))
                .arg("--download-server");
        }
        _ => {
            cmd.arg("server")
                .arg("-mcversion")
                .arg(&instance.mc_version)
                .arg("-loader")
                .arg(&loader_version)
                .arg("-dir")
                .arg(run_dir)
                .arg("-downloadMinecraft");
        }
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::Other(format!("failed to run {:?} installer: {e}", instance.loader)))?;
    let _ = std::fs::remove_file(&installer);

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail: String = err.chars().rev().take(800).collect::<String>().chars().rev().collect();
        return Err(AppError::Other(format!(
            "{:?} server installer failed: {tail}",
            instance.loader
        )));
    }

    if !launch_jar.exists() {
        return Err(AppError::Other(format!(
            "the {:?} installer did not produce {launch_name}",
            instance.loader
        )));
    }
    Ok(vec!["-jar".into(), path_str(&launch_jar), "nogui".into()])
}

/// Write `eula.txt=true`. Acceptance is gathered in the UI at creation time.
fn write_eula(run_dir: &Path) -> Result<()> {
    let eula = run_dir.join("eula.txt");
    std::fs::write(
        &eula,
        "# Accepted via Cactus Launcher (https://www.minecraft.net/eula)\neula=true\n",
    )?;
    Ok(())
}

fn path_str(p: &PathBuf) -> String {
    p.to_string_lossy().to_string()
}
