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

/// Whether an instance is a normal game client or a dedicated server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstanceKind {
    #[default]
    Client,
    Server,
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
    /// Client (playable game) or a dedicated server. Defaults to client so that
    /// instances created before this field existed still deserialize.
    #[serde(default)]
    pub kind: InstanceKind,
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
    /// When true, the icon is shown full-bleed (cover) behind the tile instead
    /// of as a small centered thumbnail.
    #[serde(default)]
    pub cover_image: bool,
    /// Max heap (MB) for a dedicated server. `None` falls back to the global
    /// memory setting. Superseded by `max_memory_mb`; kept for older instances.
    #[serde(default)]
    pub server_memory_mb: Option<u32>,

    // --- Per-instance overrides (each `None` = use the global setting) ---
    #[serde(default)]
    pub max_memory_mb: Option<u32>,
    #[serde(default)]
    pub min_memory_mb: Option<u32>,
    /// Extra JVM args for this instance (replaces the global ones when set).
    #[serde(default)]
    pub jvm_args: Option<String>,
    /// Explicit Java executable path for this instance.
    #[serde(default)]
    pub java_path: Option<String>,
    /// Game window size (client only).
    #[serde(default)]
    pub game_width: Option<u32>,
    #[serde(default)]
    pub game_height: Option<u32>,
}

impl Instance {
    pub fn new(
        name: String,
        kind: InstanceKind,
        mc_version: String,
        loader: ModLoader,
        loader_version: Option<String>,
        icon: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            kind,
            icon,
            mc_version,
            loader,
            loader_version,
            group: None,
            created: Utc::now(),
            last_played: None,
            total_playtime_seconds: 0,
            cover_image: false,
            server_memory_mb: None,
            max_memory_mb: None,
            min_memory_mb: None,
            jvm_args: None,
            java_path: None,
            game_width: None,
            game_height: None,
        }
    }
}

/// Payload sent from the frontend to create a new instance.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstance {
    pub name: String,
    #[serde(default)]
    pub kind: InstanceKind,
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
    pub cover_image: Option<bool>,
    /// Max heap (MB) for a server; 0 clears the override (use the global setting).
    pub server_memory_mb: Option<u32>,
    // Per-instance overrides. For the numeric ones, 0 clears the override; for
    // the string ones, an empty string clears it.
    pub max_memory_mb: Option<u32>,
    pub min_memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub java_path: Option<String>,
    pub game_width: Option<u32>,
    pub game_height: Option<u32>,
}
