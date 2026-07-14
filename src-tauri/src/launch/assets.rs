use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use tauri::AppHandle;

use super::download::{download_one, DownloadTask};
use crate::error::Result;
use crate::minecraft::version::AssetIndexRef;
use crate::paths;

const RESOURCES_BASE: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
    /// Very old versions store assets "virtually" (by their real path).
    #[serde(default)]
    r#virtual: bool,
    #[serde(default)]
    map_to_resources: bool,
}

/// What the resolved assets need at launch time.
pub struct ResolvedAssets {
    pub downloads: Vec<DownloadTask>,
    /// The `--assetsDir` to pass. For legacy/virtual layouts this points at the
    /// unpacked tree rather than the object store.
    pub assets_dir: PathBuf,
    /// Post-download copies for virtual/legacy layouts: (object path, dest).
    copies: Vec<(PathBuf, PathBuf)>,
}

/// Fetch and parse the asset index, producing download tasks for every object.
pub async fn resolve(
    app: &AppHandle,
    client: &reqwest::Client,
    index_ref: &AssetIndexRef,
    game_dir: &std::path::Path,
) -> Result<ResolvedAssets> {
    let assets_root = paths::assets_dir(app)?;
    let objects_dir = assets_root.join("objects");
    let indexes_dir = assets_root.join("indexes");
    std::fs::create_dir_all(&indexes_dir)?;

    // Cache the index file itself.
    let index_path = indexes_dir.join(format!("{}.json", index_ref.id));
    let index_task = DownloadTask {
        url: index_ref.url.clone(),
        dest: index_path.clone(),
        sha1: Some(index_ref.sha1.clone()),
        executable: false,
    };
    download_one(client, &index_task).await?;

    let index: AssetIndex = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;

    let legacy = index.r#virtual || index.map_to_resources;
    let virtual_root = assets_root.join("virtual").join(&index_ref.id);

    let mut downloads = Vec::new();
    let mut copies = Vec::new();

    for (path, obj) in &index.objects {
        let sub = format!("{}/{}", &obj.hash[0..2], obj.hash);
        let dest = objects_dir.join(&obj.hash[0..2]).join(&obj.hash);
        downloads.push(DownloadTask {
            url: format!("{RESOURCES_BASE}/{sub}"),
            dest: dest.clone(),
            sha1: Some(obj.hash.clone()),
            executable: false,
        });

        if legacy {
            let target = if index.map_to_resources {
                game_dir.join("resources").join(path)
            } else {
                virtual_root.join(path)
            };
            copies.push((dest, target));
        }
        let _ = obj.size;
    }

    let assets_dir = if legacy && index.r#virtual {
        virtual_root
    } else if legacy && index.map_to_resources {
        game_dir.join("resources")
    } else {
        assets_root
    };

    Ok(ResolvedAssets {
        downloads,
        assets_dir,
        copies,
    })
}

impl ResolvedAssets {
    /// Copy objects into their virtual/legacy locations (no-op for modern versions).
    pub fn materialize_virtual(&self) -> Result<()> {
        for (src, dst) in &self.copies {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if !dst.exists() {
                std::fs::copy(src, dst)?;
            }
        }
        Ok(())
    }
}
