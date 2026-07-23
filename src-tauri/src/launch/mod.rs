pub mod args;
pub mod assets;
pub mod branding;
pub mod download;
pub mod java;
pub mod libraries;
pub mod process;
pub mod rules;
pub mod server;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::instance::Instance;
use crate::minecraft::{self, version};
use crate::minecraft::version::JavaVersion;
use crate::settings::Settings;
use crate::paths;

/// Tracks running game processes so they can be stopped. `starting` additionally
/// holds instances that are preparing to launch (downloading, before the process
/// spawns) so a second launch click during that window is rejected.
#[derive(Default)]
pub struct LaunchState {
    running: Mutex<HashMap<String, oneshot::Sender<()>>>,
    starting: Mutex<HashSet<String>>,
}

impl LaunchState {
    pub fn register(&self, id: String, kill: oneshot::Sender<()>) {
        self.running.lock().unwrap().insert(id, kill);
    }
    pub fn unregister(&self, id: &str) {
        self.running.lock().unwrap().remove(id);
    }
    pub fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains_key(id)
    }
    /// Mark an instance as starting. Returns false if it is already starting or
    /// running, letting the caller reject a duplicate launch before the (slow)
    /// prepare phase begins. Pair every `true` with `finish_start`.
    pub fn try_begin_start(&self, id: &str) -> bool {
        if self.is_running(id) {
            return false;
        }
        self.starting.lock().unwrap().insert(id.to_string())
    }
    pub fn finish_start(&self, id: &str) {
        self.starting.lock().unwrap().remove(id);
    }
    pub fn kill(&self, id: &str) {
        if let Some(tx) = self.running.lock().unwrap().remove(id) {
            let _ = tx.send(());
        }
    }
}

pub enum ServerMsg {
    Line(String),
    Stop,
}

#[derive(Default)]
pub struct ServerState {
    running: Mutex<HashMap<String, mpsc::UnboundedSender<ServerMsg>>>,
}

impl ServerState {
    pub fn register(&self, id: String, tx: mpsc::UnboundedSender<ServerMsg>) {
        self.running.lock().unwrap().insert(id, tx);
    }
    pub fn unregister(&self, id: &str) {
        self.running.lock().unwrap().remove(id);
    }
    pub fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains_key(id)
    }
    pub fn send(&self, id: &str, line: String) {
        if let Some(tx) = self.running.lock().unwrap().get(id) {
            let _ = tx.send(ServerMsg::Line(line));
        }
    }
    pub fn stop(&self, id: &str) -> bool {
        if let Some(tx) = self.running.lock().unwrap().get(id) {
            let _ = tx.send(ServerMsg::Stop);
            true
        } else {
            false
        }
    }
}

pub const EVENT_STATUS: &str = "launch-status";
pub const EVENT_PROGRESS: &str = "launch-progress";
pub const EVENT_LOG: &str = "launch-log";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEvent {
    pub instance_id: String,
    /// preparing | downloading | launching | running | exited | error
    pub state: String,
    pub message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub instance_id: String,
    pub stage: String,
    pub current: usize,
    pub total: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub instance_id: String,
    pub line: String,
}

fn emit_status(app: &AppHandle, id: &str, state: &str, message: Option<String>) {
    let _ = app.emit(
        EVENT_STATUS,
        StatusEvent {
            instance_id: id.to_string(),
            state: state.to_string(),
            message,
        },
    );
}

fn emit_progress(app: &AppHandle, id: &str, stage: &str, current: usize, total: usize) {
    let _ = app.emit(
        EVENT_PROGRESS,
        ProgressEvent {
            instance_id: id.to_string(),
            stage: stage.to_string(),
            current,
            total,
        },
    );
}

pub async fn launch(app: AppHandle, instance: Instance, settings: Settings) -> Result<()> {
    let id = instance.id.clone();
    let result = prepare_and_spawn(&app, &instance, &settings).await;
    if let Err(error) = &result {
        eprintln!("[launch] error for instance {id}: {error}");
        emit_status(&app, &id, "error", Some(error.to_string()));
    }
    result
}

async fn prepare_and_spawn(app: &AppHandle, instance: &Instance, settings: &Settings) -> Result<()> {
    let id = &instance.id;
    emit_status(app, id, "preparing", Some("Resolving version…".into()));

    let client = crate::http::client()?;
    let dl_concurrency = crate::settings::clamp_concurrency(settings.max_concurrent_downloads);

    let manifest = minecraft::fetch_versions().await?;
    let entry = manifest
        .versions
        .iter()
        .find(|version| version.id == instance.mc_version)
        .ok_or_else(|| {
            AppError::Other(format!("Minecraft version '{}' not found", instance.mc_version))
        })?;

    let mut detail = version::fetch_detail(app, &entry.id, &entry.url, &entry.sha1).await?;

    // --- Ensure Java early (Forge/NeoForge need it to run their installer) ---
    emit_status(app, id, "preparing", Some("Preparing Java runtime…".into()));
    let java_version = detail.java_version.clone().unwrap_or(JavaVersion {
        component: "jre-legacy".into(),
        major_version: 8,
    });
    // Apple Silicon: versions with LWJGL < 3.3.1 have no arm64 natives, so run
    // them with an x86_64 (Rosetta) Java to match the x86_64 native libraries.
    let force_x64 = macos_needs_rosetta(&detail);
    let java_path = java::resolve_path(instance, settings, java_version.major_version);
    let java = {
        let app_cb = app.clone();
        let id_cb = id.clone();
        java::ensure_java(
            app,
            &client,
            &java_version,
            java_path,
            force_x64,
            move |cur, total| emit_progress(&app_cb, &id_cb, "java", cur, total),
        )
        .await?
    };

    use crate::instance::ModLoader;
    match instance.loader {
        ModLoader::Vanilla => {}
        ModLoader::Fabric | ModLoader::Quilt => {
            emit_status(app, id, "preparing", Some("Resolving mod loader…".into()));
            crate::loader::apply_loader(
                &mut detail,
                instance.loader,
                &instance.mc_version,
                instance.loader_version.as_deref(),
            )
            .await?;
        }
        ModLoader::Forge | ModLoader::NeoForge => {
            emit_status(
                app,
                id,
                "preparing",
                Some("Installing Forge/NeoForge (first run can take a minute)…".into()),
            );
            crate::loader::forge::apply(
                &mut detail,
                app,
                &client,
                &java,
                instance.loader,
                &instance.mc_version,
                instance.loader_version.as_deref(),
            )
            .await?;
        }
    }

    let version_jar = paths::version_dir(app, &detail.id)?.join(format!("{}.jar", detail.id));
    let natives_dir = paths::natives_dir(app, id)?;
    let game_dir = paths::instance_game_dir(app, id)?;

    let libs = libraries::resolve(app, &detail.libraries)?;
    let assets = assets::resolve(app, &client, &detail.asset_index, &game_dir).await?;

    emit_status(app, id, "downloading", Some("Downloading game files…".into()));
    let mut file_tasks = libs.downloads.clone();
    file_tasks.push(download::DownloadTask {
        url: detail.downloads.client.url.clone(),
        dest: version_jar.clone(),
        sha1: Some(detail.downloads.client.sha1.clone()),
        executable: false,
    });
    {
        let app = app.clone();
        let id = id.clone();
        download::download_all(&client, file_tasks, dl_concurrency, move |cur, total| {
            emit_progress(&app, &id, "libraries", cur, total);
        })
        .await?;
    }

    emit_status(app, id, "downloading", Some("Downloading assets…".into()));
    {
        let app = app.clone();
        let id = id.clone();
        download::download_all(&client, assets.downloads.clone(), dl_concurrency, move |cur, total| {
            emit_progress(&app, &id, "assets", cur, total);
        })
        .await?;
    }
    assets.materialize_virtual()?;

    for (jar, exclude) in &libs.natives {
        libraries::extract_natives(jar, &natives_dir, exclude)?;
    }

    // Menu branding (Cactus wordmark + splashes) — one resource pack for every
    // instance, vanilla or modded. Non-fatal: never block launch.
    if let Err(error) = branding::apply(&game_dir, &instance.mc_version, settings.menu_branding) {
        eprintln!("[branding] skipped for instance {id}: {error}");
    }

    let (player_name, uuid, access_token) =
        match crate::auth::active_valid_account(app, &client).await? {
            Some(acc) => (acc.username, acc.uuid, acc.mc_access_token),
            None => {
                let name = if settings.offline_username.trim().is_empty() {
                    "Player".to_string()
                } else {
                    settings.offline_username.trim().to_string()
                };
                let uuid = args::offline_uuid(&name);
                (name, uuid, "0".to_string())
            }
        };

    let mut classpath = libs.classpath.clone();
    classpath.push(version_jar);

    let jvm_args_src = instance
        .jvm_args
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(settings.jvm_args.as_str());
    let ctx = args::LaunchContext {
        classpath,
        natives_dir,
        game_dir: game_dir.clone(),
        assets_dir: assets.assets_dir.clone(),
        library_directory: paths::libraries_dir(app)?,
        assets_index: detail.assets.clone(),
        uuid,
        player_name,
        access_token,
        user_type: "msa".into(),
        width: instance.game_width.unwrap_or(settings.game_width),
        height: instance.game_height.unwrap_or(settings.game_height),
        min_mem: instance.min_memory_mb.unwrap_or(settings.min_memory_mb),
        max_mem: instance.max_memory_mb.unwrap_or(settings.max_memory_mb),
        extra_jvm: jvm_args_src.split_whitespace().map(String::from).collect(),
    };
    let command_args = args::build(&detail, &ctx);

    emit_status(app, id, "launching", Some("Starting Minecraft…".into()));
    process::spawn_and_monitor(app.clone(), java, command_args, game_dir, id.clone())?;

    Ok(())
}

/// On Apple Silicon, decide whether this version must run under x86_64/Rosetta.
/// True when the version's LWJGL has no arm64 macOS natives.
fn macos_needs_rosetta(detail: &version::VersionDetail) -> bool {
    if !(cfg!(target_os = "macos") && std::env::consts::ARCH == "aarch64") {
        return false;
    }
    detail
        .libraries
        .iter()
        .any(|lib| lib_lacks_arm64_natives(&lib.name))
}

/// True if this library is an LWJGL build with no arm64 macOS natives. LWJGL 2
/// (Maven group `org.lwjgl.lwjgl`, used by MC ≤1.12.2) never shipped any, and
/// LWJGL 3 (group `org.lwjgl`) only gained them in 3.3.1.
fn lib_lacks_arm64_natives(name: &str) -> bool {
    if name.starts_with("org.lwjgl.lwjgl:lwjgl:") {
        return true;
    }
    if let Some(version) = name.strip_prefix("org.lwjgl:lwjgl:") {
        return lwjgl_below_331(version);
    }
    false
}

fn lwjgl_below_331(version: &str) -> bool {
    let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
    let target = [3u32, 3, 1];
    for (index, &want) in target.iter().enumerate() {
        let part = parts.get(index).copied().unwrap_or(0);
        if part != want {
            return part < want;
        }
    }
    false // equal to 3.3.1 → has arm64 natives
}

#[cfg(test)]
mod tests {
    use super::{lib_lacks_arm64_natives, lwjgl_below_331};

    #[test]
    fn lwjgl_version_threshold() {
        assert!(lwjgl_below_331("3.2.2"));
        assert!(lwjgl_below_331("3.3.0"));
        assert!(!lwjgl_below_331("3.3.1"));
        assert!(!lwjgl_below_331("3.3.2"));
        assert!(!lwjgl_below_331("3.4.0"));
    }

    #[test]
    fn arm64_native_detection() {
        // LWJGL 2 (group org.lwjgl.lwjgl) never had arm64 macOS natives.
        assert!(lib_lacks_arm64_natives(
            "org.lwjgl.lwjgl:lwjgl:2.9.4-nightly-20150209"
        ));
        // LWJGL 3 gained them in 3.3.1.
        assert!(lib_lacks_arm64_natives("org.lwjgl:lwjgl:3.2.2"));
        assert!(!lib_lacks_arm64_natives("org.lwjgl:lwjgl:3.3.1"));
        // Unrelated libraries never force Rosetta.
        assert!(!lib_lacks_arm64_natives("com.mojang:authlib:1.5.25"));
    }
}
