//! Menu branding — drops a small Cactus resource pack into an instance and
//! enables it, so the in-game title screen shows the Cactus wordmark (in the
//! vanilla logo slot) plus Cactus splash texts.
//!
//! One mechanism for every instance, vanilla or modded: modded Minecraft still
//! loads vanilla resource packs, so no mod (FancyMenu etc.) is needed. The pack
//! is regenerated on each launch when branding is enabled and removed when it's
//! disabled, so the `menuBranding` setting is the single control.

use std::io::Write;
use std::path::Path;

use crate::error::Result;

const PACK_FILE: &str = "cactus.zip";
/// The wordmark texture, pre-baked into the vanilla two-tile title layout
/// (two 155×44 tiles at (0,0) and (0,45) of a 256×256 image).
const TITLE_PNG: &[u8] = include_bytes!("../../branding/title.png");

const SPLASHES: &str = "\
Powered by Cactus Launcher!
Stay prickly!
Open source and proud!
100% more cactus!
Now with extra spikes!
Water it once a week!
Desert vibes only!
Free and open-source!
Ouch! Careful, it's sharp!
Blocky and beautiful!
Mine on!
Crafted with love!
Photosynthesizing...
Drought-resistant!
Green and gold!
Get prickly with it!";

/// Ensure branding matches the desired state for this instance's game dir.
/// Non-fatal: callers should log and continue on error rather than block launch.
pub fn apply(game_dir: &Path, mc_version: &str, enabled: bool) -> Result<()> {
    // Clean up the old FancyMenu-based overlay from earlier builds, if present.
    remove_legacy_overlay(game_dir);

    let pack_path = game_dir.join("resourcepacks").join(PACK_FILE);
    if !enabled {
        let _ = std::fs::remove_file(&pack_path);
        set_enabled(game_dir, mc_version, false)?;
        return Ok(());
    }
    build_pack(&pack_path, mc_version)?;
    set_enabled(game_dir, mc_version, true)?;
    Ok(())
}

fn build_pack(pack_path: &Path, mc_version: &str) -> Result<()> {
    if let Some(parent) = pack_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(pack_path)?);

    // `pack_format` is the primary (used by pre-1.20.2). `supported_formats` must
    // live INSIDE the `pack` object — 1.20.2+ reads it there and accepts the pack
    // when the game's format falls in range, so the wide range keeps us
    // compatible with any current or future version.
    let mcmeta = format!(
        "{{\n  \"pack\": {{\n    \"pack_format\": {},\n    \"description\": \"Cactus Launcher branding\",\n    \"supported_formats\": {{ \"min_inclusive\": 1, \"max_inclusive\": 9999 }}\n  }}\n}}\n",
        pack_format_for(mc_version)
    );
    zip.start_file("pack.mcmeta", opts)?;
    zip.write_all(mcmeta.as_bytes())?;

    zip.start_file("assets/minecraft/texts/splashes.txt", opts)?;
    zip.write_all(SPLASHES.as_bytes())?;

    zip.start_file("assets/minecraft/textures/gui/title/minecraft.png", opts)?;
    zip.write_all(TITLE_PNG)?;

    zip.finish()?;
    Ok(())
}

/// Remove the FancyMenu overlay a previous version of the launcher installed:
/// our layout/asset plus the FancyMenu mod and its FancyMenu-only dependencies
/// (which add the unwanted top menu bar and suppress the vanilla title logo).
/// Fabric API is deliberately left alone — other mods commonly depend on it.
fn remove_legacy_overlay(game_dir: &Path) {
    let _ = std::fs::remove_file(game_dir.join("config/fancymenu/customization/cactus_branding.txt"));
    let _ = std::fs::remove_file(game_dir.join("config/fancymenu/assets/cactus_logo.png"));

    let mods = game_dir.join("mods");
    if let Ok(entries) = std::fs::read_dir(&mods) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !name.ends_with(".jar") {
                continue;
            }
            // Match the library name only at a version/word boundary, so a user's
            // unrelated mod (e.g. "melodycraft.jar") isn't caught by "melody".
            let is_ours = ["fancymenu", "konkrete", "melody"].iter().any(|lib| {
                name.strip_prefix(lib)
                    .is_some_and(|rest| rest.chars().next().map_or(true, |c| !c.is_ascii_alphanumeric()))
            });
            if is_ours {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// The `resourcePacks` entry name. Minecraft 1.13+ prefixes on-disk packs with
/// `file/`; older versions list the bare file name.
fn pack_entry(mc_version: &str) -> String {
    let (minor, _) = parse_version(mc_version);
    if minor >= 13 {
        format!("file/{PACK_FILE}")
    } else {
        PACK_FILE.to_string()
    }
}

/// Add or remove our pack from `options.txt`'s `resourcePacks` list, preserving
/// every other option. Creates the file (with just the one line) if absent —
/// Minecraft fills in the remaining defaults on first run.
fn set_enabled(game_dir: &Path, mc_version: &str, enabled: bool) -> Result<()> {
    let entry = pack_entry(mc_version);
    let options = game_dir.join("options.txt");

    let existing = std::fs::read_to_string(&options).unwrap_or_default();
    if existing.is_empty() && !enabled {
        return Ok(());
    }

    let mut lines: Vec<String> = existing.lines().map(String::from).collect();
    let idx = lines.iter().position(|l| l.starts_with("resourcePacks:"));

    // Parse the existing list. If the line is present but unparseable, bail
    // rather than overwrite it — never clobber the user's real pack list.
    let mut packs: Vec<String> = match idx {
        Some(i) => match lines[i]
            .split_once(':')
            .and_then(|(_, value)| serde_json::from_str::<Vec<String>>(value.trim()).ok())
        {
            Some(parsed) => parsed,
            None => return Ok(()),
        },
        None => Vec::new(),
    };

    packs.retain(|p| p != &entry);
    if enabled {
        packs.push(entry);
    }

    let new_line = format!("resourcePacks:{}", serde_json::to_string(&packs)?);
    match idx {
        Some(i) => lines[i] = new_line,
        None => lines.push(new_line),
    }

    if let Some(parent) = options.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(&options, out)?;
    Ok(())
}

/// Parse `(minor, patch)` from a release id like `1.20.4` → `(20, 4)` or
/// `1.21` → `(21, 0)`. Non-release ids (snapshots, `inf-`, pre-releases) can't
/// be parsed and yield a large minor so they map to the newest pack format.
fn parse_version(mc_version: &str) -> (u32, u32) {
    let mut parts = mc_version.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok());
    if major != Some(1) {
        return (u32::MAX, 0); // snapshot / unknown → treat as newest
    }
    let minor = parts.next().and_then(|s| s.parse::<u32>().ok());
    let Some(minor) = minor else {
        return (u32::MAX, 0);
    };
    // Patch may carry a suffix (e.g. "2-pre1"); take the leading digits.
    let patch = parts
        .next()
        .map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    (minor, patch)
}

/// Best-effort resource-pack `pack_format` for a Minecraft version. On 1.20.2+
/// `supported_formats` (wide range) is what actually gates acceptance, so this
/// only matters for older versions; unknowns fall back to the newest known.
fn pack_format_for(mc_version: &str) -> u32 {
    let (minor, patch) = parse_version(mc_version);
    match (minor, patch) {
        (0..=8, _) => 1,
        (9..=10, _) => 2,
        (11..=12, _) => 3,
        (13..=14, _) => 4,
        (15, _) | (16, 0..=1) => 5,
        (16, _) => 6,
        (17, _) => 7,
        (18, _) => 8,
        (19, 0..=2) => 9,
        (19, 3) => 12,
        (19, _) => 13,
        (20, 0..=1) => 15,
        (20, 2) => 18,
        (20, 3..=4) => 22,
        (20, _) => 32,
        (21, 0..=1) => 34,
        (21, 2..=3) => 42,
        (21, 4) => 46,
        (21, _) => 55,
        _ => 55,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("1.20.4"), (20, 4));
        assert_eq!(parse_version("1.21"), (21, 0));
        assert_eq!(parse_version("1.7.10"), (7, 10));
        assert_eq!(parse_version("1.20.2-pre1"), (20, 2));
        assert_eq!(parse_version("24w14a").0, u32::MAX);
    }

    #[test]
    fn pack_formats() {
        assert_eq!(pack_format_for("1.12.2"), 3);
        assert_eq!(pack_format_for("1.20.1"), 15);
        assert_eq!(pack_format_for("1.21.4"), 46);
        assert_eq!(pack_format_for("1.8.9"), 1);
    }

    #[test]
    fn entry_name_prefix() {
        assert_eq!(pack_entry("1.12.2"), "cactus.zip");
        assert_eq!(pack_entry("1.20.1"), "file/cactus.zip");
    }

    #[test]
    fn options_merge_preserves_and_toggles() {
        let dir = std::env::temp_dir().join(format!("cactus-brand-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let options = dir.join("options.txt");
        std::fs::write(&options, "fov:0.5\nresourcePacks:[\"vanilla\"]\nlang:en_us\n").unwrap();

        set_enabled(&dir, "1.20.1", true).unwrap();
        let text = std::fs::read_to_string(&options).unwrap();
        assert!(text.contains("fov:0.5"));
        assert!(text.contains("lang:en_us"));
        assert!(text.contains("file/cactus.zip"));
        assert!(text.contains("vanilla"));

        // Toggling off removes only our entry.
        set_enabled(&dir, "1.20.1", false).unwrap();
        let text = std::fs::read_to_string(&options).unwrap();
        assert!(!text.contains("cactus.zip"));
        assert!(text.contains("vanilla"));
        assert!(text.contains("fov:0.5"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
