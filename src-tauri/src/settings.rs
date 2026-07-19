use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::Result;
use crate::paths;

/// Default parallel downloads — a good balance for most connections.
fn default_concurrency() -> u32 {
    16
}

/// Clamp a user-set download concurrency into a safe range.
pub fn clamp_concurrency(n: u32) -> usize {
    n.clamp(1, 64) as usize
}

/// Global launcher settings, persisted to `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "dark" | "light" | "system"
    pub theme: String,
    /// Explicit Java executable path applied to any version (legacy fallback).
    /// `None` = auto-detect / managed runtime. Prefer `java_paths` per major.
    pub java_path: Option<String>,
    /// Per-major-version Java executables (e.g. 8/17/21 -> path), chosen to match
    /// the version's required Java. Empty = use the managed runtime. Overrides
    /// `java_path` for the matching major.
    #[serde(default)]
    pub java_paths: HashMap<u32, String>,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
    /// How many files to download at once across the launcher (launch pipeline,
    /// content, modpacks). Higher can be faster on quick connections.
    #[serde(default = "default_concurrency")]
    pub max_concurrent_downloads: u32,
    /// Extra JVM arguments appended at launch.
    pub jvm_args: String,
    pub game_width: u32,
    pub game_height: u32,
    /// Username used for offline/dev launches (until Microsoft auth lands).
    pub offline_username: String,
    /// App background. Empty = default; otherwise `color:#rrggbb`,
    /// `pattern:<name>`, or `image:<data-uri>`.
    pub background: String,
    /// Play subtle UI click sounds on buttons.
    pub ui_sounds: bool,
    /// User-supplied Giphy API key that enables the animated-sticker picker.
    /// Empty = stickers disabled (the emoji picker still works).
    pub giphy_api_key: String,
    /// Where the dock sits: "bottom" | "top" | "left" | "right".
    pub dock_position: String,
    /// Placed-sprite decoration theme id ("" = none).
    pub decor_theme: String,
    /// macOS-style magnify-on-hover for the dock.
    pub dock_magnify: bool,
    /// Default parent folder for new instances' game data. Empty = the app's
    /// own instances folder. Only affects instances created after it's set.
    #[serde(default)]
    pub instances_dir: String,
    /// ngrok authtoken used to share a server over the internet. A per-instance
    /// token overrides this. Empty = not configured.
    #[serde(default)]
    pub ngrok_authtoken: String,
    /// Show the "Latest news" section on the Home screen.
    #[serde(default = "default_true")]
    pub show_news: bool,
    /// News layout: `true` = one story per page, `false` = a lead + two-up.
    #[serde(default)]
    pub news_single: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            java_path: None,
            java_paths: HashMap::new(),
            max_memory_mb: 4096,
            min_memory_mb: 1024,
            max_concurrent_downloads: default_concurrency(),
            jvm_args: String::new(),
            game_width: 854,
            game_height: 480,
            offline_username: "Player".into(),
            background: String::new(),
            ui_sounds: true,
            giphy_api_key: String::new(),
            dock_position: "bottom".into(),
            decor_theme: String::new(),
            dock_magnify: true,
            instances_dir: String::new(),
            ngrok_authtoken: String::new(),
            show_news: true,
            news_single: false,
        }
    }
}

/// Thread-safe settings holder that writes through to disk on save.
#[derive(Default)]
pub struct SettingsStore {
    inner: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(&self, app: &AppHandle) -> Result<()> {
        let file = paths::settings_file(app)?;
        if file.exists() {
            let data = std::fs::read_to_string(&file)?;
            if let Ok(settings) = serde_json::from_str::<Settings>(&data) {
                *self.inner.lock().unwrap() = settings;
            }
        }
        Ok(())
    }

    pub fn get(&self) -> Settings {
        self.inner.lock().unwrap().clone()
    }

    pub fn save(&self, app: &AppHandle, settings: Settings) -> Result<()> {
        let file = paths::settings_file(app)?;
        std::fs::write(&file, serde_json::to_string_pretty(&settings)?)?;
        *self.inner.lock().unwrap() = settings;
        Ok(())
    }
}
