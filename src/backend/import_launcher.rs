use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use crate::host::{Host};

use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::instance::{Instance, InstanceKind, ModLoader};
use crate::paths;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLauncher {
    pub id: String,
    pub name: String,
    pub instance_count: usize,
    pub can_delete: bool,
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalInstance {
    pub launcher: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub game_dir: String,
    pub source_dir: Option<String>,
    pub icon: Option<String>,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn app_support(name: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        match home_dir() {
            Some(home) => vec![home.join("Library/Application Support").join(name)],
            None => Vec::new(),
        }
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(|appdata| vec![PathBuf::from(appdata).join(name)])
            .unwrap_or_default()
    }
    #[cfg(target_os = "linux")]
    {
        match home_dir() {
            Some(home) => vec![home.join(".local/share").join(name)],
            None => Vec::new(),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = name;
        Vec::new()
    }
}

fn candidate_roots(launcher: &str) -> Vec<PathBuf> {
    let home = home_dir();
    let mut roots: Vec<PathBuf> = match launcher {
        "prism" => {
            let mut dirs = app_support("PrismLauncher");
            if let Some(home) = &home {
                dirs.push(home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"));
            }
            dirs
        }
        "polymc" => {
            let mut dirs = app_support("PolyMC");
            if let Some(home) = &home {
                dirs.push(home.join(".var/app/org.polymc.PolyMC/data/PolyMC"));
            }
            dirs
        }
        "multimc" => {
            let mut dirs = app_support("MultiMC");
            if let Some(home) = &home {
                dirs.push(home.join(".local/share/multimc"));
                dirs.push(home.join("MultiMC"));
            }
            dirs
        }
        "curseforge" => {
            let mut dirs = Vec::new();
            if let Some(home) = &home {
                dirs.push(home.join("curseforge/minecraft/Instances"));
                dirs.push(home.join("Documents/curseforge/minecraft/Instances"));
                dirs.push(home.join("OneDrive/Documents/curseforge/minecraft/Instances"));
            }
            dirs
        }
        "modrinth" => {
            let mut dirs = app_support("ModrinthApp");
            dirs.extend(app_support("com.modrinth.theseus"));
            if let Some(home) = &home {
                dirs.push(home.join(".var/app/com.modrinth.ModrinthApp/data/ModrinthApp"));
            }
            dirs
        }
        "vanilla" => {
            let mut dirs = Vec::new();
            #[cfg(target_os = "macos")]
            if let Some(home) = &home {
                dirs.push(home.join("Library/Application Support/minecraft"));
            }
            #[cfg(target_os = "windows")]
            if let Some(appdata) = std::env::var_os("APPDATA") {
                dirs.push(PathBuf::from(appdata).join(".minecraft"));
            }
            #[cfg(target_os = "linux")]
            if let Some(home) = &home {
                dirs.push(home.join(".minecraft"));
            }
            dirs
        }
        _ => Vec::new(),
    };
    roots.retain(|path| path.is_dir());
    roots
}

const LAUNCHERS: &[(&str, &str, bool)] = &[
    ("prism", "Prism Launcher", true),
    ("multimc", "MultiMC", true),
    ("polymc", "PolyMC", true),
    ("curseforge", "CurseForge", true),
    ("modrinth", "Modrinth App", true),
    ("vanilla", "Minecraft Launcher", false),
];

pub fn detect() -> Vec<DetectedLauncher> {
    let mut found = Vec::new();
    for (id, name, can_delete) in LAUNCHERS {
        for root in candidate_roots(id) {
            let instances = parse_launcher(id, &root);
            if instances.is_empty() {
                continue;
            }
            found.push(DetectedLauncher {
                id: (*id).to_string(),
                name: (*name).to_string(),
                instance_count: instances.len(),
                can_delete: *can_delete,
                icon: launcher_icon(id),
            });
            break;
        }
    }
    found
}

fn launcher_can_delete(id: &str) -> bool {
    LAUNCHERS
        .iter()
        .find(|(lid, _, _)| *lid == id)
        .map(|(_, _, can_delete)| *can_delete)
        .unwrap_or(false)
}

pub fn list_instances(launcher: &str) -> Vec<ExternalInstance> {
    for root in candidate_roots(launcher) {
        let instances = parse_launcher(launcher, &root);
        if !instances.is_empty() {
            return instances;
        }
    }
    Vec::new()
}

fn parse_launcher(launcher: &str, root: &Path) -> Vec<ExternalInstance> {
    match launcher {
        "prism" | "multimc" | "polymc" => parse_multimc(launcher, root),
        "curseforge" => parse_curseforge(root),
        "modrinth" => parse_modrinth(root),
        "vanilla" => parse_vanilla(root),
        _ => Vec::new(),
    }
}

fn looks_like_mc_version(token: &str) -> bool {
    let mut parts = token.split('.');
    let major = parts.next().unwrap_or("");
    !major.is_empty()
        && major.chars().all(|c| c.is_ascii_digit())
        && token.split('.').all(|p| p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty())
        && token.contains('.')
}

fn parse_multimc(launcher: &str, root: &Path) -> Vec<ExternalInstance> {
    let instances_dir = root.join("instances");
    let Ok(entries) = std::fs::read_dir(&instances_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let cfg = dir.join("instance.cfg");
        if !cfg.exists() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().into_owned();
        let cfg_text = std::fs::read_to_string(&cfg).unwrap_or_default();
        let name = ini_get(&cfg_text, "name").unwrap_or(folder);

        let (mc_version, loader, loader_version) = read_mmc_pack(&dir.join("mmc-pack.json"));
        if mc_version.is_empty() {
            continue;
        }

        let game_dir = [".minecraft", "minecraft"]
            .iter()
            .map(|sub| dir.join(sub))
            .find(|path| path.is_dir())
            .unwrap_or_else(|| dir.join(".minecraft"));

        let icon = ini_get(&cfg_text, "iconKey").and_then(|key| multimc_icon(root, &key));

        out.push(ExternalInstance {
            launcher: launcher.to_string(),
            name,
            mc_version,
            loader: loader.id().to_string(),
            loader_version,
            game_dir: game_dir.to_string_lossy().into_owned(),
            source_dir: Some(dir.to_string_lossy().into_owned()),
            icon,
        });
    }
    out
}

fn ini_get(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                let value = v.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn multimc_icon(root: &Path, key: &str) -> Option<String> {
    if key.is_empty() || key == "default" {
        return None;
    }
    let icons = root.join("icons");
    for ext in ["png", "jpg", "jpeg", "gif", "webp"] {
        let candidate = icons.join(format!("{key}.{ext}"));
        if candidate.is_file() {
            return image_file_data_uri(&candidate);
        }
    }
    None
}

fn read_mmc_pack(path: &Path) -> (String, ModLoader, Option<String>) {
    #[derive(Deserialize)]
    struct Component {
        uid: String,
        #[serde(default)]
        version: Option<String>,
    }
    #[derive(Deserialize)]
    struct Pack {
        #[serde(default)]
        components: Vec<Component>,
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return (String::new(), ModLoader::Vanilla, None);
    };
    let Ok(pack) = serde_json::from_str::<Pack>(&text) else {
        return (String::new(), ModLoader::Vanilla, None);
    };

    let mut mc_version = String::new();
    let mut loader = ModLoader::Vanilla;
    let mut loader_version = None;
    for component in pack.components {
        match component.uid.as_str() {
            "net.minecraft" => mc_version = component.version.unwrap_or_default(),
            "net.fabricmc.fabric-loader" => {
                loader = ModLoader::Fabric;
                loader_version = component.version;
            }
            "org.quiltmc.quilt-loader" => {
                loader = ModLoader::Quilt;
                loader_version = component.version;
            }
            "net.minecraftforge" => {
                loader = ModLoader::Forge;
                loader_version = component.version;
            }
            "net.neoforged" => {
                loader = ModLoader::NeoForge;
                loader_version = component.version;
            }
            _ => {}
        }
    }
    (mc_version, loader, loader_version)
}

fn parse_curseforge(instances_dir: &Path) -> Vec<ExternalInstance> {
    let Ok(entries) = std::fs::read_dir(instances_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        let manifest = dir.join("minecraftinstance.json");
        if !manifest.exists() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };

        let folder = entry.file_name().to_string_lossy().into_owned();
        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&folder)
            .to_string();
        let mc_version = json
            .get("gameVersion")
            .and_then(|v| v.as_str())
            .or_else(|| {
                json.get("baseModLoader")
                    .and_then(|m| m.get("minecraftVersion"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or_default()
            .to_string();
        if mc_version.is_empty() {
            continue;
        }

        let (loader, loader_version) = json
            .get("baseModLoader")
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .and_then(|full| full.split_once('-'))
            .map(|(kind, version)| (ModLoader::from_id(&kind.to_lowercase()), Some(version.to_string())))
            .unwrap_or((ModLoader::Vanilla, None));

        let icon = json
            .get("installedModpack")
            .and_then(|modpack| modpack.get("thumbnailUrl"))
            .and_then(|value| value.as_str())
            .filter(|url| url.starts_with("https://"))
            .map(str::to_string);

        out.push(ExternalInstance {
            launcher: "curseforge".to_string(),
            name,
            mc_version,
            loader: loader.id().to_string(),
            loader_version,
            game_dir: dir.to_string_lossy().into_owned(),
            source_dir: Some(dir.to_string_lossy().into_owned()),
            icon,
        });
    }
    out
}

fn parse_modrinth(root: &Path) -> Vec<ExternalInstance> {
    let db = root.join("app.db");
    if !db.exists() {
        return Vec::new();
    }
    read_modrinth_db(root, &db).unwrap_or_default()
}

fn read_modrinth_db(root: &Path, db: &Path) -> Result<Vec<ExternalInstance>> {
    let conn = open_sqlite_ro(db)
        .map_err(|error| AppError::Other(format!("modrinth db: {error}")))?;
    let mut stmt = conn
        .prepare(
            "SELECT i.path, i.name, i.icon_path, s.game_version, s.loader, s.loader_version \
             FROM instances i \
             LEFT JOIN instance_content_sets s ON s.id = i.applied_content_set_id",
        )
        .map_err(|error| AppError::Other(format!("modrinth db: {error}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|error| AppError::Other(format!("modrinth db: {error}")))?;

    let mut out = Vec::new();
    for row in rows.flatten() {
        let (path, name, icon_path, game_version, mod_loader, loader_version) = row;
        let Some(mc_version) = game_version.filter(|value| !value.is_empty()) else {
            continue;
        };
        if paths::safe_relative_path(&path).is_none() {
            continue;
        }
        let game_dir = root.join("profiles").join(&path);
        if !game_dir.is_dir() {
            continue;
        }
        let icon = icon_path
            .filter(|value| !value.is_empty())
            .and_then(|value| {
                let candidate = Path::new(&value);
                let resolved = if candidate.is_absolute() {
                    candidate.to_path_buf()
                } else {
                    game_dir.join(candidate)
                };
                image_file_data_uri(&resolved)
            });
        let loader = ModLoader::from_id(&mod_loader.as_deref().unwrap_or("vanilla").to_lowercase());
        out.push(ExternalInstance {
            launcher: "modrinth".to_string(),
            name,
            mc_version,
            loader: loader.id().to_string(),
            loader_version: loader_version.filter(|value| !value.is_empty()),
            game_dir: game_dir.to_string_lossy().into_owned(),
            source_dir: Some(game_dir.to_string_lossy().into_owned()),
            icon,
        });
    }
    Ok(out)
}

fn open_sqlite_ro(db: &Path) -> rusqlite::Result<rusqlite::Connection> {
    use rusqlite::{Connection, OpenFlags};
    Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).or_else(|_| {
        let encoded: String = db
            .to_string_lossy()
            .chars()
            .flat_map(|ch| match ch {
                '%' => "%25".chars().collect::<Vec<_>>(),
                '?' => "%3F".chars().collect(),
                '#' => "%23".chars().collect(),
                ' ' => "%20".chars().collect(),
                '\\' => vec!['/'],
                other => vec![other],
            })
            .collect();
        Connection::open_with_flags(
            format!("file:{encoded}?immutable=1"),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
    })
}

fn parse_vanilla(root: &Path) -> Vec<ExternalInstance> {
    let file = root.join("launcher_profiles.json");
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(profiles) = json.get("profiles").and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (key, profile) in profiles {
        let last_version = profile
            .get("lastVersionId")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let Some((mc_version, loader, loader_version)) = parse_vanilla_version(last_version) else {
            continue;
        };

        let name = profile
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(key)
            .to_string();

        let custom_dir = profile
            .get("gameDir")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let game_dir = custom_dir.clone().unwrap_or_else(|| root.to_path_buf());

        let icon = profile
            .get("icon")
            .and_then(|v| v.as_str())
            .filter(|value| value.starts_with("data:"))
            .map(str::to_string);

        out.push(ExternalInstance {
            launcher: "vanilla".to_string(),
            name,
            mc_version,
            loader: loader.id().to_string(),
            loader_version,
            game_dir: game_dir.to_string_lossy().into_owned(),
            source_dir: custom_dir.map(|dir| dir.to_string_lossy().into_owned()),
            icon,
        });
    }
    out
}

fn parse_vanilla_version(id: &str) -> Option<(String, ModLoader, Option<String>)> {
    if id.is_empty() || id.starts_with("latest-") {
        return None;
    }
    if let Some(rest) = id.strip_prefix("fabric-loader-") {
        let (loader_ver, mc) = rest.split_once('-')?;
        return Some((mc.to_string(), ModLoader::Fabric, Some(loader_ver.to_string())));
    }
    if let Some(rest) = id.strip_prefix("quilt-loader-") {
        let (loader_ver, mc) = rest.split_once('-')?;
        return Some((mc.to_string(), ModLoader::Quilt, Some(loader_ver.to_string())));
    }
    let lower = id.to_lowercase();
    if lower.contains("neoforge") {
        let mc = id.split('-').find(|t| looks_like_mc_version(t))?;
        return Some((mc.to_string(), ModLoader::NeoForge, None));
    }
    if lower.contains("forge") {
        let mc = id.split('-').find(|t| looks_like_mc_version(t))?;
        return Some((mc.to_string(), ModLoader::Forge, None));
    }
    Some((id.to_string(), ModLoader::Vanilla, None))
}

const SKIP_NAMES: &[&str] = &[
    "versions",
    "libraries",
    "assets",
    "logs",
    "crash-reports",
    "natives",
    "bin",
    ".cache",
    "cache",
    ".fabric",
    ".mixin.out",
    ".quilt",
    "usercache.json",
    "usernamecache.json",
    ".ds_store",
];

pub async fn import(
    app: &Host,
    external: &ExternalInstance,
    delete_source: bool,
) -> Result<Instance> {
    let external = list_instances(&external.launcher)
        .into_iter()
        .find(|candidate| {
            candidate.game_dir == external.game_dir
                && candidate.source_dir == external.source_dir
                && candidate.name == external.name
        })
        .ok_or_else(|| {
            AppError::Other("that instance is no longer available in its launcher".into())
        })?;

    let source_game = PathBuf::from(&external.game_dir);
    let meta = std::fs::symlink_metadata(&source_game).map_err(|_| {
        AppError::Other(format!("source folder no longer exists: {}", external.game_dir))
    })?;
    if !meta.is_dir() {
        return Err(AppError::Other(format!(
            "source folder is not a real directory: {}",
            external.game_dir
        )));
    }

    let store = app.state::<InstanceStore>();
    let mut instance = Instance::new(
        store.unique_name(&external.name),
        InstanceKind::Client,
        external.mc_version.clone(),
        ModLoader::from_id(&external.loader.to_lowercase()),
        external.loader_version.clone(),
        None,
    );
    instance.icon = external.icon.clone();
    store.save(app, &instance)?;

    let dest = paths::instance_game_dir(app, &instance.id)?;
    std::fs::create_dir_all(&dest)?;
    if let Err(error) = copy_filtered(&source_game, &dest) {
        let _ = store.delete(app, &instance.id);
        return Err(error);
    }

    let _ = crate::content::register_local(app, &instance.id);
    let _ = crate::content::resolve_local(app, &instance.id).await;

    if delete_source && launcher_can_delete(&external.launcher) {
        delete_source_dir(app, &external);
    }

    Ok(store.get(&instance.id).unwrap_or(instance))
}

fn copy_filtered(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if SKIP_NAMES.contains(&name.as_str()) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let dest = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dest, 0)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

const MAX_COPY_DEPTH: u32 = 64;

fn copy_tree(from: &Path, to: &Path, depth: u32) -> Result<()> {
    if depth > MAX_COPY_DEPTH {
        return Err(AppError::Other("source folder is nested too deeply".into()));
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let dest = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dest, depth + 1)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

fn delete_source_dir(app: &Host, external: &ExternalInstance) {
    let Some(source) = external.source_dir.as_deref().filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let path = PathBuf::from(source);
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return;
    };
    if !meta.is_dir() {
        return;
    }
    if path.components().count() < 3 {
        return;
    }
    match home_dir() {
        Some(home) if path.starts_with(&home) => {}
        _ => return,
    }
    if let Ok(data_dir) = paths::data_dir(app) {
        if path.starts_with(&data_dir) {
            return;
        }
    }
    let _ = std::fs::remove_dir_all(&path);
}

fn launcher_icon(id: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        macos_app_icon(id)
    }
    #[cfg(target_os = "linux")]
    {
        linux_icon(id)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = id;
        None
    }
}

fn png_data_uri(png: &[u8]) -> String {
    use base64::Engine;
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

const MAX_ICON_BYTES: u64 = 4 * 1024 * 1024;

fn image_file_data_uri(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_ICON_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        _ => "image/png",
    };
    use base64::Engine;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

fn icns_to_data_uri(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_ICON_BYTES {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;
    let family = icns::IconFamily::read(std::io::BufReader::new(file)).ok()?;
    let best = family
        .available_icons()
        .into_iter()
        .max_by_key(|icon_type| icon_type.pixel_width())?;
    let image = family.get_icon_with_type(best).ok()?;
    let mut png = Vec::new();
    image.write_png(&mut png).ok()?;
    Some(png_data_uri(&png))
}

#[cfg(target_os = "macos")]
fn macos_app_icon(id: &str) -> Option<String> {
    let app_name = match id {
        "prism" => "Prism Launcher",
        "polymc" => "PolyMC",
        "multimc" => "MultiMC",
        "curseforge" => "CurseForge",
        "modrinth" => "Modrinth App",
        "vanilla" => "Minecraft",
        _ => return None,
    };
    let mut candidates = vec![PathBuf::from("/Applications").join(format!("{app_name}.app"))];
    if let Some(home) = home_dir() {
        candidates.push(home.join("Applications").join(format!("{app_name}.app")));
    }
    let resources = candidates
        .into_iter()
        .map(|app| app.join("Contents/Resources"))
        .find(|dir| dir.is_dir())?;

    let mut icns: Vec<PathBuf> = std::fs::read_dir(&resources)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("icns"))
        })
        .collect();
    icns.sort_by_key(|path| std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0));
    icns_to_data_uri(&icns.pop()?)
}

#[cfg(target_os = "linux")]
fn linux_icon(id: &str) -> Option<String> {
    let names: &[&str] = match id {
        "prism" => &["org.prismlauncher.PrismLauncher"],
        "polymc" => &["org.polymc.PolyMC"],
        "modrinth" => &["com.modrinth.ModrinthApp", "ModrinthApp"],
        "vanilla" => &["minecraft-launcher"],
        _ => return None,
    };
    let home = home_dir();
    let mut bases: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/var/lib/flatpak/exports/share/icons"),
    ];
    if let Some(home) = &home {
        bases.push(home.join(".local/share/icons"));
        bases.push(home.join(".local/share/flatpak/exports/share/icons"));
    }
    for base in &bases {
        for name in names {
            for size in ["512x512", "256x256", "128x128", "192x192", "scalable"] {
                let path = base.join(format!("hicolor/{size}/apps/{name}.png"));
                if path.is_file() {
                    if let Ok(bytes) = std::fs::read(&path) {
                        return Some(png_data_uri(&bytes));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fabric_version_id() {
        let (mc, loader, ver) = parse_vanilla_version("fabric-loader-0.15.7-1.20.1").unwrap();
        assert_eq!(mc, "1.20.1");
        assert_eq!(loader, ModLoader::Fabric);
        assert_eq!(ver.as_deref(), Some("0.15.7"));
    }

    #[test]
    fn parses_forge_version_id() {
        let (mc, loader, _) = parse_vanilla_version("1.20.1-forge-47.1.0").unwrap();
        assert_eq!(mc, "1.20.1");
        assert_eq!(loader, ModLoader::Forge);
    }

    #[test]
    fn skips_latest_pseudo_ids() {
        assert!(parse_vanilla_version("latest-release").is_none());
        assert!(parse_vanilla_version("").is_none());
    }

    #[test]
    fn parses_plain_release() {
        let (mc, loader, _) = parse_vanilla_version("1.21").unwrap();
        assert_eq!(mc, "1.21");
        assert_eq!(loader, ModLoader::Vanilla);
    }

    #[test]
    fn reads_ini_value() {
        let cfg = "[General]\nname=My Pack\niconKey=default\n";
        assert_eq!(ini_get(cfg, "name").as_deref(), Some("My Pack"));
        assert_eq!(ini_get(cfg, "missing"), None);
    }

    #[test]
    fn mc_version_detection() {
        assert!(looks_like_mc_version("1.20.1"));
        assert!(looks_like_mc_version("1.21"));
        assert!(!looks_like_mc_version("forge"));
        assert!(!looks_like_mc_version("47"));
    }

    #[test]
    fn can_delete_matches_table() {
        assert!(launcher_can_delete("modrinth"));
        assert!(launcher_can_delete("prism"));
        assert!(!launcher_can_delete("vanilla"));
        assert!(!launcher_can_delete("unknown"));
    }
}
