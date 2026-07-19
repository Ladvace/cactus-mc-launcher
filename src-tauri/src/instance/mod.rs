pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstanceKind {
    #[default]
    Client,
    Server,
}

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
    pub loader_version: Option<String>,
    pub group: Option<String>,
    pub created: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub total_playtime_seconds: u64,
    #[serde(default)]
    pub cover_image: bool,
    /// Max heap (MB) for a dedicated server. `None` falls back to the global
    /// memory setting. Superseded by `max_memory_mb`; kept for older instances.
    #[serde(default)]
    pub server_memory_mb: Option<u32>,

    #[serde(default)]
    pub max_memory_mb: Option<u32>,
    #[serde(default)]
    pub min_memory_mb: Option<u32>,
    #[serde(default)]
    pub jvm_args: Option<String>,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub game_width: Option<u32>,
    #[serde(default)]
    pub game_height: Option<u32>,
    /// Absolute path to this instance's game directory (mods, saves, worlds).
    /// `None` = the default location under the instances folder. Set to move an
    /// instance's data to another drive/folder without moving its record.
    #[serde(default)]
    pub game_dir: Option<String>,
    #[serde(default)]
    pub ngrok_authtoken: Option<String>,
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
            game_dir: None,
            ngrok_authtoken: None,
        }
    }
}

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
    /// Per-instance ngrok authtoken; empty string clears it (use the global one).
    pub ngrok_authtoken: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_kind_defaults_to_client() {
        assert_eq!(InstanceKind::default(), InstanceKind::Client);
    }

    #[test]
    fn old_instance_json_deserializes_with_field_defaults() {
        let json = r#"{
            "id":"abc","name":"Old","icon":null,"mcVersion":"1.20.1",
            "loader":"fabric","loaderVersion":null,"group":null,
            "created":"2024-01-01T00:00:00Z","lastPlayed":null,"totalPlaytimeSeconds":0
        }"#;
        let inst: Instance = serde_json::from_str(json).unwrap();
        assert_eq!(inst.kind, InstanceKind::Client);
        assert_eq!(inst.loader, ModLoader::Fabric);
        assert!(!inst.cover_image);
        assert!(inst.server_memory_mb.is_none());
        assert!(inst.max_memory_mb.is_none());
        assert!(inst.java_path.is_none());
    }
}
