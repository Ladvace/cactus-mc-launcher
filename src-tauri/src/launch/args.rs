use std::collections::HashMap;
use std::path::PathBuf;

use md5::{Digest, Md5};

use super::rules::rules_allow;
use crate::minecraft::version::{ArgValue, Argument, VersionDetail};

const LAUNCHER_NAME: &str = "cactus-launcher";
const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Everything needed to render a version's launch command for one instance.
pub struct LaunchContext {
    pub classpath: Vec<PathBuf>,
    pub natives_dir: PathBuf,
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    /// Shared libraries root (Forge's module-path args reference this).
    pub library_directory: PathBuf,
    pub assets_index: String,
    pub player_name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub width: u32,
    pub height: u32,
    pub min_mem: u32,
    pub max_mem: u32,
    pub extra_jvm: Vec<String>,
}

/// Offline (cracked) player UUID, matching Java's `UUID.nameUUIDFromBytes`
/// over `OfflinePlayer:<name>` (a name-based v3 UUID).
pub fn offline_uuid(name: &str) -> String {
    let mut hash = Md5::digest(format!("OfflinePlayer:{name}").as_bytes());
    hash[6] = (hash[6] & 0x0f) | 0x30; // version 3
    hash[8] = (hash[8] & 0x3f) | 0x80; // RFC 4122 variant
    let hex_str = hex::encode(hash);
    format!(
        "{}-{}-{}-{}-{}",
        &hex_str[0..8],
        &hex_str[8..12],
        &hex_str[12..16],
        &hex_str[16..20],
        &hex_str[20..32]
    )
}

fn classpath_string(paths: &[PathBuf]) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(sep)
}

/// Build the full argument vector: JVM args, then main class, then game args.
pub fn build(detail: &VersionDetail, ctx: &LaunchContext) -> Vec<String> {
    let subs = substitutions(detail, ctx);

    // Feature flags gate certain arguments (e.g. custom resolution).
    let mut features = HashMap::new();
    features.insert("has_custom_resolution".to_string(), true);
    features.insert("is_demo_user".to_string(), false);

    let mut out = Vec::new();

    // Memory + user JVM args always come first.
    out.push(format!("-Xmx{}M", ctx.max_mem));
    out.push(format!("-Xms{}M", ctx.min_mem));
    out.extend(ctx.extra_jvm.iter().filter(|s| !s.is_empty()).cloned());

    match &detail.arguments {
        // Modern (1.13+): structured jvm/game argument lists.
        Some(arguments) => {
            out.extend(collect(&arguments.jvm, &features, &subs));
            out.push(detail.main_class.clone());
            out.extend(collect(&arguments.game, &features, &subs));
        }
        // Legacy (pre-1.13): default JVM args + a single arguments string.
        None => {
            out.push(format!(
                "-Djava.library.path={}",
                ctx.natives_dir.to_string_lossy()
            ));
            out.push("-cp".to_string());
            out.push(classpath_string(&ctx.classpath));
            out.push(detail.main_class.clone());
            if let Some(legacy) = &detail.minecraft_arguments {
                for token in legacy.split_whitespace() {
                    out.push(apply(token, &subs));
                }
            }
        }
    }

    out
}

/// Collect the applicable entries from a modern argument list, substituting
/// placeholders and honoring OS/feature rules.
fn collect(
    args: &[Argument],
    features: &HashMap<String, bool>,
    subs: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Argument::Plain(value) => out.push(apply(value, subs)),
            Argument::Conditional { rules, value } => {
                if !rules_allow(rules, features) {
                    continue;
                }
                match value {
                    ArgValue::One(value) => out.push(apply(value, subs)),
                    ArgValue::Many(list) => {
                        for value in list {
                            out.push(apply(value, subs));
                        }
                    }
                }
            }
        }
    }
    out
}

/// Replace every `${token}` in `input` with its substitution (unknown tokens
/// are left untouched).
fn apply(input: &str, subs: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, val) in subs {
        let needle = format!("${{{key}}}");
        if result.contains(&needle) {
            result = result.replace(&needle, val);
        }
    }
    result
}

fn substitutions(detail: &VersionDetail, ctx: &LaunchContext) -> HashMap<String, String> {
    let mut subs = HashMap::new();
    let path_str = |path: &PathBuf| path.to_string_lossy().to_string();

    subs.insert("natives_directory".into(), path_str(&ctx.natives_dir));
    subs.insert("launcher_name".into(), LAUNCHER_NAME.into());
    subs.insert("launcher_version".into(), LAUNCHER_VERSION.into());
    subs.insert("classpath".into(), classpath_string(&ctx.classpath));
    subs.insert("library_directory".into(), path_str(&ctx.library_directory));
    subs.insert(
        "classpath_separator".into(),
        if cfg!(windows) { ";" } else { ":" }.into(),
    );

    subs.insert("auth_player_name".into(), ctx.player_name.clone());
    subs.insert("version_name".into(), detail.id.clone());
    subs.insert("game_directory".into(), path_str(&ctx.game_dir));
    subs.insert("assets_root".into(), path_str(&ctx.assets_dir));
    subs.insert("game_assets".into(), path_str(&ctx.assets_dir)); // legacy alias
    subs.insert("assets_index_name".into(), ctx.assets_index.clone());
    subs.insert("auth_uuid".into(), ctx.uuid.clone());
    subs.insert("auth_access_token".into(), ctx.access_token.clone());
    subs.insert("auth_session".into(), ctx.access_token.clone()); // legacy alias
    subs.insert("auth_xuid".into(), String::new());
    subs.insert("clientid".into(), String::new());
    subs.insert("user_type".into(), ctx.user_type.clone());
    subs.insert("user_properties".into(), "{}".into());
    subs.insert("version_type".into(), detail.kind.clone());
    subs.insert("resolution_width".into(), ctx.width.to_string());
    subs.insert("resolution_height".into(), ctx.height.to_string());

    subs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_stable_and_v3() {
        let a = offline_uuid("Player");
        assert_eq!(a, offline_uuid("Player"), "same name → same uuid");
        assert_ne!(a, offline_uuid("Steve"));
        assert_eq!(a.len(), 36); // dashed
        assert_eq!(a.as_bytes()[14], b'3'); // version 3 nibble
    }
}
