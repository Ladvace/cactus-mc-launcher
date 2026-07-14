use std::path::Path;

fn main() {
    load_env_var();
    tauri_build::build()
}

/// Expose `AZURE_CLIENT_ID` to the crate via `env!`/`option_env!`.
///
/// Precedence: a real environment variable wins; otherwise it's read from a
/// gitignored `.env` file next to this build script. Leave it unset to keep
/// Microsoft login disabled (offline mode still works).
fn load_env_var() {
    const KEY: &str = "AZURE_CLIENT_ID";

    // Rebuild if the env var or the .env file changes.
    println!("cargo:rerun-if-env-changed={KEY}");
    println!("cargo:rerun-if-changed=.env");

    // 1) Real environment variable takes priority.
    if let Ok(val) = std::env::var(KEY) {
        if !val.trim().is_empty() {
            println!("cargo:rustc-env={KEY}={}", val.trim());
            return;
        }
    }

    // 2) Fall back to a `.env` file (simple KEY=VALUE lines, `#` comments).
    let env_path = Path::new(".env");
    if let Ok(contents) = std::fs::read_to_string(env_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == KEY {
                    let v = v.trim().trim_matches('"').trim_matches('\'');
                    if !v.is_empty() {
                        println!("cargo:rustc-env={KEY}={v}");
                    }
                    return;
                }
            }
        }
    }
}
