//! Server operator (`ops.json`) and whitelist (`whitelist.json`) management.
//!
//! When a server is running the UI sends console commands instead; these
//! functions edit the JSON files directly for a stopped server, resolving the
//! player's UUID (from Mojang for online-mode, or the offline hash otherwise).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::{AppError, Result};
use crate::launch::args::offline_uuid;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpEntry {
    pub uuid: String,
    pub name: String,
    #[serde(default = "default_level")]
    pub level: u8,
    #[serde(default)]
    pub bypasses_player_limit: bool,
}

fn default_level() -> u8 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerEntry {
    pub uuid: String,
    pub name: String,
}

fn ops_file(app: &AppHandle, id: &str) -> Result<PathBuf> {
    Ok(paths::instance_game_dir(app, id)?.join("ops.json"))
}
fn whitelist_file(app: &AppHandle, id: &str) -> Result<PathBuf> {
    Ok(paths::instance_game_dir(app, id)?.join("whitelist.json"))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Vec<T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize>(path: &PathBuf, items: &[T]) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(items)?)?;
    Ok(())
}

pub fn read_ops(app: &AppHandle, id: &str) -> Result<Vec<OpEntry>> {
    Ok(read_json(&ops_file(app, id)?))
}

pub fn read_whitelist(app: &AppHandle, id: &str) -> Result<Vec<PlayerEntry>> {
    Ok(read_json(&whitelist_file(app, id)?))
}

/// Whether this server verifies accounts against Mojang (`online-mode`,
/// default true).
fn is_online_mode(app: &AppHandle, id: &str) -> bool {
    let props = paths::instance_game_dir(app, id)
        .ok()
        .map(|dir| dir.join("server.properties"));
    let text = props.and_then(|path| std::fs::read_to_string(path).ok());
    let Some(text) = text else { return true };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("online-mode=") {
            return value.trim() != "false";
        }
    }
    true
}

async fn resolve_player(app: &AppHandle, id: &str, name: &str) -> Result<(String, String)> {
    if is_online_mode(app, id) {
        #[derive(Deserialize)]
        struct Profile {
            id: String,
            name: String,
        }
        let url = format!("https://api.mojang.com/users/profiles/minecraft/{name}");
        let resp = crate::http::client()?.get(&url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Other(format!("No Minecraft account named '{name}'.")));
        }
        let profile: Profile = resp.error_for_status()?.json().await?;
        Ok((dash_uuid(&profile.id), profile.name))
    } else {
        Ok((offline_uuid(name), name.to_string()))
    }
}

/// Insert dashes into a 32-char undashed UUID (8-4-4-4-12).
fn dash_uuid(uuid: &str) -> String {
    if uuid.len() != 32 {
        return uuid.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &uuid[0..8],
        &uuid[8..12],
        &uuid[12..16],
        &uuid[16..20],
        &uuid[20..32]
    )
}

pub async fn add_op(app: &AppHandle, id: &str, name: &str, level: u8) -> Result<()> {
    let (uuid, canonical) = resolve_player(app, id, name).await?;
    let path = ops_file(app, id)?;
    let mut ops: Vec<OpEntry> = read_json(&path);
    ops.retain(|op| !op.name.eq_ignore_ascii_case(&canonical) && op.uuid != uuid);
    ops.push(OpEntry {
        uuid,
        name: canonical,
        level: level.clamp(1, 4),
        bypasses_player_limit: false,
    });
    write_json(&path, &ops)
}

pub fn remove_op(app: &AppHandle, id: &str, name: &str) -> Result<()> {
    let path = ops_file(app, id)?;
    let mut ops: Vec<OpEntry> = read_json(&path);
    ops.retain(|op| !op.name.eq_ignore_ascii_case(name));
    write_json(&path, &ops)
}

pub async fn add_whitelist(app: &AppHandle, id: &str, name: &str) -> Result<()> {
    let (uuid, canonical) = resolve_player(app, id, name).await?;
    let path = whitelist_file(app, id)?;
    let mut list: Vec<PlayerEntry> = read_json(&path);
    list.retain(|player| !player.name.eq_ignore_ascii_case(&canonical) && player.uuid != uuid);
    list.push(PlayerEntry { uuid, name: canonical });
    write_json(&path, &list)
}

pub fn remove_whitelist(app: &AppHandle, id: &str, name: &str) -> Result<()> {
    let path = whitelist_file(app, id)?;
    let mut list: Vec<PlayerEntry> = read_json(&path);
    list.retain(|player| !player.name.eq_ignore_ascii_case(name));
    write_json(&path, &list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_uuid_inserts_hyphens() {
        assert_eq!(
            dash_uuid("0123456789abcdef0123456789abcdef"),
            "01234567-89ab-cdef-0123-456789abcdef"
        );
        assert_eq!(dash_uuid("short"), "short");
    }
}
