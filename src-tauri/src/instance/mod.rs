pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported mod loaders. `Vanilla` means no loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

/// A single game installation the user can configure and launch.
///
/// Persisted as `instance.json` inside the instance's own folder. Game files
/// (mods, saves, config, the versioned client) live alongside it later.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    /// Optional icon: a filename relative to the instance folder, or a data URI.
    pub icon: Option<String>,
    pub mc_version: String,
    pub loader: ModLoader,
    /// Loader version (e.g. a Fabric loader build). `None` for vanilla or "latest".
    pub loader_version: Option<String>,
    /// Optional grouping label shown in the library (e.g. "Modpacks").
    pub group: Option<String>,
    pub created: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub total_playtime_seconds: u64,
}

impl Instance {
    pub fn new(
        name: String,
        mc_version: String,
        loader: ModLoader,
        loader_version: Option<String>,
        icon: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            icon,
            mc_version,
            loader,
            loader_version,
            group: None,
            created: Utc::now(),
            last_played: None,
            total_playtime_seconds: 0,
        }
    }
}

/// Payload sent from the frontend to create a new instance.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstance {
    pub name: String,
    pub mc_version: String,
    #[serde(default)]
    pub loader: ModLoader,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// Patch payload for editing an existing instance. Only `Some` fields are applied.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstance {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub group: Option<String>,
    pub mc_version: Option<String>,
    pub loader: Option<ModLoader>,
    pub loader_version: Option<String>,
}
