use std::collections::HashMap;
use std::sync::Mutex;

use tauri::AppHandle;

use super::Instance;
use crate::error::{AppError, Result};
use crate::paths;

/// In-memory cache of all instances, backed by one `instance.json` per folder
/// on disk. All mutations write through to disk immediately.
#[derive(Default)]
pub struct InstanceStore {
    cache: Mutex<HashMap<String, Instance>>,
}

impl InstanceStore {
    /// Scan the instances directory and populate the cache. Called once at startup.
    pub fn load(&self, app: &AppHandle) -> Result<()> {
        let dir = paths::instances_dir(app)?;
        let mut map = HashMap::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let meta = entry.path().join("instance.json");
            if !meta.exists() {
                continue;
            }
            match std::fs::read_to_string(&meta).and_then(|s| {
                serde_json::from_str::<Instance>(&s).map_err(std::io::Error::other)
            }) {
                Ok(inst) => {
                    map.insert(inst.id.clone(), inst);
                }
                Err(e) => eprintln!("skipping unreadable instance {}: {e}", meta.display()),
            }
        }
        *self.cache.lock().unwrap() = map;
        Ok(())
    }

    /// All instances, sorted most-recently-played first, then by name.
    pub fn list(&self) -> Vec<Instance> {
        let mut list: Vec<Instance> = self.cache.lock().unwrap().values().cloned().collect();
        list.sort_by(|a, b| {
            b.last_played
                .cmp(&a.last_played)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        list
    }

    pub fn get(&self, id: &str) -> Option<Instance> {
        self.cache.lock().unwrap().get(id).cloned()
    }

    /// Persist an instance (create or overwrite) and update the cache.
    pub fn save(&self, app: &AppHandle, instance: &Instance) -> Result<()> {
        let dir = paths::instance_dir(app, &instance.id)?;
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(instance)?;
        std::fs::write(dir.join("instance.json"), json)?;
        self.cache
            .lock()
            .unwrap()
            .insert(instance.id.clone(), instance.clone());
        Ok(())
    }

    /// Remove an instance and its entire folder from disk.
    pub fn delete(&self, app: &AppHandle, id: &str) -> Result<()> {
        if self.cache.lock().unwrap().remove(id).is_none() {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        let dir = paths::instance_dir(app, id)?;
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}
