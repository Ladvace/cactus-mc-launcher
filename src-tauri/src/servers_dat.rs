//! Append to the vanilla `servers.dat` (uncompressed Java NBT) so a server added
//! from the launcher shows up in Minecraft's in-game multiplayer list. Uses the
//! dynamic NBT `Value` so existing entries and their fields are preserved.

use std::collections::HashMap;
use std::path::Path;

use fastnbt::Value;

use crate::error::{AppError, Result};

fn nbt_err(e: impl std::fmt::Display) -> AppError {
    AppError::Other(format!("servers.dat: {e}"))
}

pub fn add_server(path: &Path, name: &str, address: &str) -> Result<()> {
    let mut root: Value = if path.exists() {
        fastnbt::from_bytes(&std::fs::read(path)?).map_err(nbt_err)?
    } else {
        Value::Compound(HashMap::new())
    };

    let Value::Compound(map) = &mut root else {
        return Err(AppError::Other("servers.dat: unexpected root tag".into()));
    };
    let servers = map
        .entry("servers".to_string())
        .or_insert_with(|| Value::List(Vec::new()));
    let Value::List(list) = servers else {
        return Err(AppError::Other("servers.dat: 'servers' is not a list".into()));
    };

    let existing = list.iter_mut().find_map(|item| match item {
        Value::Compound(entry)
            if matches!(entry.get("ip"), Some(Value::String(ip)) if ip == address) =>
        {
            Some(entry)
        }
        _ => None,
    });
    if let Some(entry) = existing {
        entry.insert("name".into(), Value::String(name.to_string()));
    } else {
        let mut entry = HashMap::new();
        entry.insert("name".to_string(), Value::String(name.to_string()));
        entry.insert("ip".to_string(), Value::String(address.to_string()));
        list.push(Value::Compound(entry));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, fastnbt::to_bytes(&root).map_err(nbt_err)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_then_updates_a_server() {
        let dir = std::env::temp_dir().join(format!("cactus-sd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("servers.dat");
        let _ = std::fs::remove_file(&path);

        add_server(&path, "Hypixel", "mc.hypixel.net").unwrap();
        add_server(&path, "Wynn", "play.wynncraft.com").unwrap();
        add_server(&path, "Hypixel Network", "mc.hypixel.net").unwrap();

        let root: Value = fastnbt::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
        let Value::Compound(map) = root else { panic!("root") };
        let Value::List(list) = &map["servers"] else { panic!("list") };
        assert_eq!(list.len(), 2);
        let names: Vec<&str> = list
            .iter()
            .filter_map(|v| match v {
                Value::Compound(e) => match e.get("name") {
                    Some(Value::String(s)) => Some(s.as_str()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert!(names.contains(&"Hypixel Network"));
        assert!(names.contains(&"Wynn"));
        let _ = std::fs::remove_file(&path);
    }
}
