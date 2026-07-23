pub mod forge;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::instance::ModLoader;
use crate::minecraft::version::{Argument, Arguments, Library, VersionDetail};

fn meta_base(loader: ModLoader) -> Option<&'static str> {
    match loader {
        ModLoader::Fabric => Some("https://meta.fabricmc.net/v2"),
        ModLoader::Quilt => Some("https://meta.quiltmc.org/v3"),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Debug, Deserialize)]
struct LoaderInfo {
    version: String,
    // Quilt sends an explicit `null` here, which `#[serde(default)]` alone
    // wouldn't cover — Option handles both missing and null.
    #[serde(default)]
    stable: Option<bool>,
}

/// List available loader builds for a Minecraft version (newest first).
pub async fn list_versions(loader: ModLoader, mc_version: &str) -> Result<Vec<LoaderVersion>> {
    if matches!(loader, ModLoader::Forge | ModLoader::NeoForge) {
        return forge::list_versions(loader, mc_version).await;
    }

    let base = meta_base(loader)
        .ok_or_else(|| AppError::Other(format!("{loader:?} is not supported yet")))?;
    let url = format!("{base}/versions/loader/{mc_version}");
    let entries: Vec<LoaderEntry> = crate::http::client()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(entries
        .into_iter()
        .map(|entry| LoaderVersion {
            version: entry.loader.version,
            stable: entry.loader.stable.unwrap_or(false),
        })
        .collect())
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProfileMainClass {
    Plain(String),
    Sided { client: String },
}

#[derive(Debug, Deserialize)]
struct ProfileLib {
    name: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ProfileArgs {
    #[serde(default)]
    jvm: Vec<String>,
    #[serde(default)]
    game: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileJson {
    #[serde(rename = "mainClass")]
    main_class: ProfileMainClass,
    #[serde(default)]
    libraries: Vec<ProfileLib>,
    #[serde(default)]
    arguments: Option<ProfileArgs>,
}

async fn fetch_profile(
    loader: ModLoader,
    mc_version: &str,
    loader_version: &str,
) -> Result<ProfileJson> {
    let base = meta_base(loader)
        .ok_or_else(|| AppError::Other(format!("{loader:?} is not supported yet")))?;
    let url = format!("{base}/versions/loader/{mc_version}/{loader_version}/profile/json");
    Ok(crate::http::client()?.get(url).send().await?.error_for_status()?.json().await?)
}

async fn resolve_version(
    loader: ModLoader,
    mc_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(version) = requested {
        if !version.trim().is_empty() {
            return Ok(version.trim().to_string());
        }
    }
    let versions = list_versions(loader, mc_version).await?;
    versions
        .iter()
        .find(|version| version.stable)
        .or_else(|| versions.first())
        .map(|version| version.version.clone())
        .ok_or_else(|| {
            AppError::Other(format!(
                "no {loader:?} loader builds available for Minecraft {mc_version}"
            ))
        })
}

pub async fn resolve_loader_version(
    loader: ModLoader,
    mc_version: &str,
    requested: Option<&str>,
) -> Result<String> {
    resolve_version(loader, mc_version, requested).await
}

pub async fn apply_loader(
    detail: &mut VersionDetail,
    loader: ModLoader,
    mc_version: &str,
    loader_version: Option<&str>,
) -> Result<String> {
    let resolved = resolve_version(loader, mc_version, loader_version).await?;
    let profile = fetch_profile(loader, mc_version, &resolved).await?;

    detail.main_class = match profile.main_class {
        ProfileMainClass::Plain(class_name) => class_name,
        ProfileMainClass::Sided { client } => client,
    };

    // Loader libraries go first so they take classpath precedence.
    let mut libs: Vec<Library> = profile
        .libraries
        .into_iter()
        .map(|lib| Library {
            name: lib.name,
            downloads: None,
            url: lib.url,
            rules: None,
            natives: None,
            extract: None,
        })
        .collect();
    libs.append(&mut detail.libraries);
    detail.libraries = libs;

    if let Some(extra) = profile.arguments {
        let args = detail.arguments.get_or_insert_with(Arguments::default);
        args.jvm.extend(extra.jvm.into_iter().map(Argument::Plain));
        args.game.extend(extra.game.into_iter().map(Argument::Plain));
    }

    Ok(resolved)
}
