use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::Result;
use crate::paths;

fn default_concurrency() -> u32 {
    8
}

pub fn clamp_concurrency(n: u32) -> usize {
    n.clamp(1, 64) as usize
}

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
    #[serde(default = "default_concurrency")]
    pub max_concurrent_downloads: u32,
    pub jvm_args: String,
    pub game_width: u32,
    pub game_height: u32,
    pub offline_username: String,
    /// App background. Empty = default; otherwise `color:#rrggbb`,
    /// `pattern:<name>`, or `image:<data-uri>`.
    pub background: String,
    pub ui_sounds: bool,
    /// User-supplied Giphy API key that enables the animated-sticker picker.
    /// Empty = stickers disabled (the emoji picker still works).
    pub giphy_api_key: String,
    /// Where the dock sits: "bottom" | "top" | "left" | "right".
    pub dock_position: String,
    /// "" = none.
    pub decor_theme: String,
    pub dock_magnify: bool,
    /// Default parent folder for new instances' game data. Empty = the app's
    /// own instances folder. Only affects instances created after it's set.
    #[serde(default)]
    pub instances_dir: String,
    /// ngrok authtoken used to share a server over the internet. A per-instance
    /// token overrides this. Empty = not configured.
    #[serde(default)]
    pub ngrok_authtoken: String,
    #[serde(default = "default_true")]
    pub show_news: bool,
    /// News layout: `true` = one story per page, `false` = a lead + two-up.
    #[serde(default)]
    pub news_single: bool,
    /// How dates are displayed: "system" | "iso" | "us" | "eu".
    #[serde(default = "default_date_format")]
    pub date_format: String,
    /// UI language code (see the i18n LOCALES list, e.g. "en", "fr", "ar", "ja").
    #[serde(default = "default_language")]
    pub language: String,
    /// Accent colour preset ("" = default gold).
    #[serde(default)]
    pub accent: String,
    /// Whole-UI zoom in percent (100 = default).
    #[serde(default = "default_ui_scale")]
    pub ui_scale: u32,
    /// UI click-sound volume, 0–100.
    #[serde(default = "default_volume")]
    pub sound_volume: u32,
    /// Loader pre-selected in the new-instance dialog.
    #[serde(default = "default_loader")]
    pub default_loader: String,
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default)]
    pub readable_font: bool,
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default)]
    pub reduce_transparency: bool,
    #[serde(default)]
    pub always_show_focus: bool,
}

fn default_ui_scale() -> u32 {
    100
}

fn default_volume() -> u32 {
    100
}

fn default_loader() -> String {
    "vanilla".into()
}

fn default_date_format() -> String {
    "system".into()
}

fn default_language() -> String {
    "en".into()
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
            date_format: default_date_format(),
            language: default_language(),
            accent: String::new(),
            ui_scale: default_ui_scale(),
            sound_volume: default_volume(),
            default_loader: default_loader(),
            reduce_motion: false,
            readable_font: false,
            high_contrast: false,
            reduce_transparency: false,
            always_show_focus: false,
        }
    }
}

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
