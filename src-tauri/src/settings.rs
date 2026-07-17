use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::Result;
use crate::paths;

/// Global launcher settings, persisted to `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "dark" | "light" | "system"
    pub theme: String,
    /// Explicit Java executable path. `None` = auto-detect / managed runtime.
    pub java_path: Option<String>,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            java_path: None,
            max_memory_mb: 4096,
            min_memory_mb: 1024,
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
