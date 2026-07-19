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
            match std::fs::read_to_string(&meta).and_then(|text| {
                serde_json::from_str::<Instance>(&text).map_err(std::io::Error::other)
            }) {
                Ok(instance) => {
                    map.insert(instance.id.clone(), instance);
                }
                Err(error) => eprintln!("skipping unreadable instance {}: {error}", meta.display()),
            }
        }
        *self.cache.lock().unwrap() = map;
        Ok(())
    }

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

    /// On the first save of a new instance, pin its game directory from the global
    /// instances-folder setting so a later change never moves existing data.
    pub fn save(&self, app: &AppHandle, instance: &Instance) -> Result<()> {
        let mut instance = instance.clone();
        let is_new = !self.cache.lock().unwrap().contains_key(&instance.id);
        if is_new && instance.game_dir.is_none() {
            instance.game_dir = paths::new_instance_game_dir(app, &instance.id);
        }

        let dir = paths::instance_dir(app, &instance.id)?;
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(&instance)?;
        std::fs::write(dir.join("instance.json"), json)?;
        self.cache
            .lock()
            .unwrap()
            .insert(instance.id.clone(), instance);
        Ok(())
    }

    pub fn delete(&self, app: &AppHandle, id: &str) -> Result<()> {
        let removed = self.cache.lock().unwrap().remove(id);
        let Some(instance) = removed else {
            return Err(AppError::InstanceNotFound(id.to_string()));
        };

        if let Some(game_dir) = instance.game_dir.as_deref().filter(|path| !path.trim().is_empty()) {
            let game_dir = std::path::Path::new(game_dir);
            if game_dir.exists() {
                let _ = std::fs::remove_dir_all(game_dir);
            }
        }

        let dir = paths::instance_dir(app, id)?;
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}
