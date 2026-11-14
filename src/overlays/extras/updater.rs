//! `UpdatePrompt.svelte` and `src/lib/stores/updater.svelte.ts`. A packaged build (a `.app`, an
//! AppImage, an installed Windows copy) checks the signed `latest.json` the releases publish and
//! installs in place; a dev run only reports the newest release and opens its page.

use rune::{Color, Cx, El, Transition, col, row, text};
use serde::Deserialize;

use crate::app::App;
use crate::core::i18n;
use crate::icons;
use crate::ui::{self, Btn};

const ENDPOINT: &str = "https://github.com/Ladvace/cactus-mc-launcher/releases/latest/download/latest.json";
const RELEASES_API: &str = "https://api.github.com/repos/Ladvace/cactus-mc-launcher/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/Ladvace/cactus-mc-launcher/releases/latest";
/// The minisign key update archives are signed with (the one the Tauri builds trusted).
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEI4MUFDODlDNDU3Njk3NDcKUldSSGwzWkZuTWdhdUlNU1hZaGc2Rjk0dTA2QVVjZTVUOXZ0QkpmMElyLzEwQyt5YXFjSnk5UEYK";

#[derive(Clone, Debug)]
pub struct Update {
    pub version: String,
    pub url: String,
    /// The verified package to install, when this build can update itself.
    package: Option<cargo_packager_updater::Update>,
}

#[derive(Default)]
pub struct State {
    /// Some while the prompt is up ("available").
    pub update: Option<Update>,
    pub checking: bool,
    /// The label while an update downloads and installs.
    installing: Option<String>,
    /// Flatpak builds update through the software centre.
    managed: Option<bool>,
}

/// The silent check on launch. Headless runs (snapshots, scripts) skip it so they stay offline
/// and deterministic.
pub fn on_start(app: &mut App) {
    let headless = std::env::args().any(|arg| matches!(arg.as_str(), "--snap" | "--tree" | "--script" | "--trace" | "--video" | "--out"));
    if !headless {
        check(app, false);
    }
}

/// `updater.check(manual)`: a manual check reports "up to date" and failures as toasts.
pub fn check(app: &mut App, manual: bool) {
    if app.extras.updater.checking {
        return;
    }
    let managed = *app.extras.updater.managed.get_or_insert_with(crate::commands::is_flatpak);
    if managed {
        if manual {
            app.toasts.success("Updates are managed by your software centre (Flathub).");
        }
        return;
    }
    app.extras.updater.checking = true;
    let done = move |app: &mut App, result: Result<Update, String>| {
        app.extras.updater.checking = false;
        match result {
            Ok(update) if newer(&update.version, env!("CARGO_PKG_VERSION")) => app.extras.updater.update = Some(update),
            Ok(_) => {
                if manual {
                    app.toasts.success("You're on the latest version.");
                }
            }
            Err(e) => {
                if manual {
                    app.toasts.error(format!("Update check failed: {e}"));
                }
            }
        }
    };
    if packaged() {
        app.fx.blocking(|_| packaged_check(), done);
    } else {
        app.fx.spawn(latest(), done);
    }
}

/// Running from what an installer put down, so replacing it in place is safe. Elsewhere (a
/// `cargo run` binary) the updater would treat the build folder as the app and replace it.
fn packaged() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    if cfg!(target_os = "macos") {
        exe.to_string_lossy().contains(".app/Contents/MacOS/")
    } else if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        !exe.components().any(|component| component.as_os_str() == "target")
    }
}

/// The signed manifest, checked by the updater; `version` past ours, or this version again.
fn packaged_check() -> Result<Update, String> {
    use cargo_packager_updater::{Config, UpdaterBuilder, semver::Version, url::Url};
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    let config = Config { endpoints: vec![Url::parse(ENDPOINT).map_err(|e| e.to_string())?], pubkey: PUBKEY.into(), windows: None };
    let found = UpdaterBuilder::new(current, config).build().and_then(|updater| updater.check()).map_err(|e| e.to_string())?;
    Ok(match found {
        Some(package) => Update { version: package.version.trim_start_matches('v').to_string(), url: RELEASES_PAGE.into(), package: Some(package) },
        None => Update { version: env!("CARGO_PKG_VERSION").into(), url: RELEASES_PAGE.into(), package: None },
    })
}

/// The newest release: the updater manifest first, then the GitHub API.
async fn latest() -> Result<Update, String> {
    #[derive(Deserialize)]
    struct Manifest {
        version: String,
    }
    #[derive(Deserialize)]
    struct GhRelease {
        tag_name: String,
        html_url: String,
    }
    let client = crate::http::client().map_err(|e| e.to_string())?;
    let manifest = async {
        let response = client.get(ENDPOINT).send().await.ok()?.error_for_status().ok()?;
        response.json::<Manifest>().await.ok()
    }
    .await;
    if let Some(manifest) = manifest {
        return Ok(Update { version: manifest.version.trim_start_matches('v').to_string(), url: RELEASES_PAGE.into(), package: None });
    }
    let response = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let gh: GhRelease = response.json().await.map_err(|e| e.to_string())?;
    Ok(Update { version: gh.tag_name.trim_start_matches('v').to_string(), url: gh.html_url, package: None })
}

/// Semver-ish: numeric dotted parts, a pre-release (`-beta`) sorting before its release.
fn newer(candidate: &str, current: &str) -> bool {
    fn parts(version: &str) -> (Vec<u64>, bool) {
        let (core, pre) = version.split_once('-').map_or((version, false), |(core, _)| (core, true));
        (core.split('.').map(|part| part.parse().unwrap_or(0)).collect(), pre)
    }
    let (a, a_pre) = parts(candidate);
    let (b, b_pre) = parts(current);
    let n = a.len().max(b.len());
    for i in 0..n {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    b_pre && !a_pre
}

/// "Install & restart": download, verify and install the package, then start the new version.
/// Without a package (a dev run, the manifest unreachable) the release page opens instead.
fn install(app: &mut App) {
    if app.extras.updater.installing.is_some() {
        return;
    }
    let Some(update) = app.extras.updater.update.clone() else { return };
    let Some(package) = update.package else {
        app.extras.updater.update = None;
        if let Err(e) = open::that(&update.url) {
            app.toasts.error(format!("Update failed: {e}"));
        }
        return;
    };
    // A game's output pipes into this process; restarting under it would break them.
    if app.launch.values().any(|launch| launch.is_busy() || launch.is_running()) {
        app.toasts.info("Close your running games before updating.");
        return;
    }
    app.extras.updater.installing = Some(i18n::t("update.downloading"));
    let fx = app.fx.clone();
    app.fx.blocking(
        move |_| {
            let bytes = package.download()?;
            fx.post(|app| app.extras.updater.installing = Some(i18n::t("update.installing")));
            // Windows: this runs the installer, which restarts the app, and exits.
            package.install(bytes)?;
            Ok::<_, cargo_packager_updater::Error>(package.extract_path.clone())
        },
        |app, result| match result {
            Ok(path) => relaunch(&path),
            Err(e) => {
                app.extras.updater.installing = None;
                app.toasts.error(format!("Update failed: {e}"));
            }
        },
    );
}

/// Start the freshly installed copy and leave.
fn relaunch(path: &std::path::Path) {
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg("-n").arg(path).spawn()
    } else {
        std::process::Command::new(path).spawn()
    };
    if started.is_ok() {
        std::process::exit(0);
    }
}

pub fn view(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) {
    let Some(update) = &app.extras.updater.update else { return };
    let theme = app.theme();
    let head = row()
        .center_y()
        .gap(9.6)
        .child(ui::colored_icon(&icons::DOWNLOAD, 16.0, theme.text))
        .child(
            col()
                .child(text(i18n::t("update.available")).font(theme.sans(14.0, 600.0)).color(theme.text))
                .child(text(format!("v{}", update.version)).font(theme.sans(12.5, 400.0)).color(theme.text_muted)),
        );
    let actions = row()
        .justify_end()
        .gap(8.0)
        .child(
            Btn::new(i18n::t("update.later"))
                .key("update-later")
                .ghost()
                .sm()
                .disabled(app.extras.updater.installing.is_some())
                .on_press(|app| app.extras.updater.update = None)
                .build(&theme),
        )
        .child(match &app.extras.updater.installing {
            Some(label) => Btn::new(label.clone()).key("update-install").primary().sm().disabled(true).build(&theme),
            None => Btn::new(i18n::t("update.installRestart")).key("update-install").primary().sm().on_press(install).build(&theme),
        });
    layer.push(
        col()
            .key("update-prompt")
            .role("status")
            .pin(None, None, Some(18.0), Some(18.0))
            .w(320.0)
            .gap(9.6)
            .px(16.0)
            .py(13.6)
            .fill(theme.bg_card)
            .border(1.0, theme.border)
            .radius(4.0)
            .shadow(12.0, 34.0, Color::BLACK.alpha(0.4))
            .transition(Transition::offset(0.0, 12.0))
            .child(head)
            .child(actions),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn compares_versions() {
        assert!(super::newer("1.12.0", "1.11.0"));
        assert!(super::newer("1.11.1", "1.11.0"));
        assert!(!super::newer("1.11.0", "1.11.0"));
        assert!(!super::newer("1.10.9", "1.11.0"));
        assert!(super::newer("1.11.0", "1.11.0-beta"));
    }

    /// The release manifest carries Tauri's keys (`darwin-aarch64`, for installs still on the
    /// Tauri build) next to ours (`macos-aarch64`). Every entry needs a `format`, or the updater
    /// rejects the whole manifest; Tauri ignores the field.
    #[test]
    fn reads_the_shared_manifest() {
        let manifest = r#"{
            "version": "1.12.0",
            "notes": "",
            "pub_date": "2026-11-20T10:00:00Z",
            "platforms": {
                "darwin-aarch64": { "signature": "c2ln", "url": "https://example.com/Cactus.Launcher_aarch64.app.tar.gz", "format": "app" },
                "macos-aarch64": { "signature": "c2ln", "url": "https://example.com/Cactus.Launcher_aarch64.app.tar.gz", "format": "app" },
                "windows-x86_64": { "signature": "c2ln", "url": "https://example.com/setup.exe", "format": "nsis" },
                "linux-x86_64": { "signature": "c2ln", "url": "https://example.com/x.AppImage", "format": "appimage" }
            }
        }"#;
        let release: cargo_packager_updater::RemoteRelease = serde_json::from_str(manifest).unwrap();
        assert_eq!(release.version.to_string(), "1.12.0");
        assert!(release.download_url("macos-aarch64").unwrap().as_str().ends_with(".app.tar.gz"));
        assert!(release.format("windows-x86_64").is_ok());
        assert!(release.download_url("linux-x86_64").is_ok());
    }
}
