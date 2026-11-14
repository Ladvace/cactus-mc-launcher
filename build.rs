//! AZURE_CLIENT_ID and CACTUS_API_BASE for the backend's `option_env!`, from the environment or a
//! gitignored `.env` here (see `.env.example`). Unset leaves those features off.

use std::path::Path;

const ENV_KEYS: &[&str] = &["AZURE_CLIENT_ID", "CACTUS_API_BASE"];
const ENV_FILE: &str = ".env";

fn main() {
    println!("cargo:rerun-if-changed={ENV_FILE}");
    for key in ENV_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        if let Some(val) = resolve_key(key).filter(|v| !v.is_empty()) {
            println!("cargo:rustc-env={key}={val}");
        }
    }
}

/// A real environment variable takes precedence over the `.env` file.
fn resolve_key(key: &str) -> Option<String> {
    if let Some(val) = std::env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
        return Some(val);
    }
    let contents = std::fs::read_to_string(Path::new(ENV_FILE)).ok()?;
    contents.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').trim_matches('\'').to_string())
    })
}
