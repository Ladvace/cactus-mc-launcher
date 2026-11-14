//! What the Svelte app kept in `localStorage` (`readJson`/`writeJson` in `src/lib/storage.ts`),
//! under the same keys, in `ui-prefs.json` beside `settings.json`. A webview's localStorage
//! can't be read from here, so these start empty on first run.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Default)]
pub struct Prefs {
    path: Option<PathBuf>,
    values: BTreeMap<String, Value>,
}

impl Prefs {
    pub fn load(dir: Option<PathBuf>) -> Prefs {
        let path = dir.map(|dir| dir.join("ui-prefs.json"));
        let values = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Prefs { path, values }
    }

    /// The prefs in the current data folder, so a moved data folder takes them along.
    /// Before that was the rule they always lived in the default folder: when the data
    /// folder has none, adopt those; the next write saves them in the data folder.
    pub fn load_for(host: &crate::host::Host) -> Prefs {
        let data_dir = crate::paths::data_dir(host).ok();
        let mut prefs = Prefs::load(data_dir.clone());
        let default_dir = crate::paths::default_data_dir(host).ok();
        if prefs.values.is_empty() && default_dir != data_dir {
            prefs.values = Prefs::load(default_dir).values;
        }
        prefs
    }

    /// Forget every value and delete the file (`localStorage.clear()`).
    pub fn clear(&mut self) {
        self.values.clear();
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }

    pub fn read<T: DeserializeOwned>(&self, key: &str, fallback: T) -> T {
        self.values.get(key).and_then(|value| serde_json::from_value(value.clone()).ok()).unwrap_or(fallback)
    }

    pub fn write(&mut self, key: &str, value: impl Serialize) {
        if let Ok(json) = serde_json::to_value(value) {
            self.values.insert(key.to_string(), json);
            self.flush();
        }
    }

    pub fn remove(&mut self, key: &str) {
        if self.values.remove(key).is_some() {
            self.flush();
        }
    }

    fn flush(&self) {
        let Some(path) = &self.path else { return };
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.values) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(tmp, path);
            }
        }
    }
}
