use std::path::Path;

/// Secrets exposed to the crate via `option_env!`, read from the environment
/// or a gitignored `.env` file. Leave unset to keep the feature disabled.
const ENV_KEYS: &[&str] = &["AZURE_CLIENT_ID", "CURSEFORGE_API_KEY", "GIPHY_API_KEY"];

fn main() {
    load_env_vars();
    tauri_build::build()
}

fn load_env_vars() {
    println!("cargo:rerun-if-changed=.env");
    for key in ENV_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
    }
    for key in ENV_KEYS {
        if let Some(val) = resolve_key(key) {
            if !val.is_empty() {
                println!("cargo:rustc-env={key}={val}");
            }
        }
    }
}

/// A real environment variable takes precedence over the `.env` file.
fn resolve_key(key: &str) -> Option<String> {
    if let Ok(val) = std::env::var(key) {
        let val = val.trim();
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }
    let contents = std::fs::read_to_string(Path::new(".env")).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    None
}
