//! Content-addressed store for downloaded content (mods, resource packs,
//! shaders). Files are kept once under `meta/content-cache/<xx>/<sha1>` and
//! **hard-linked** into each instance's game folder, so ten modpacks that share
//! the same shader or library keep a single copy on disk instead of ten.

use std::path::{Path, PathBuf};

use futures::stream::{self, StreamExt};
use sha1::{Digest, Sha1};
use tauri::AppHandle;

use crate::error::{AppError, Result};
use crate::launch::download::DownloadTask;
use crate::paths;

pub fn blob_path(app: &AppHandle, sha1: &str) -> Result<PathBuf> {
    let sha1_lower = sha1.to_lowercase();
    let shard = if sha1_lower.len() >= 2 { &sha1_lower[..2] } else { "00" };
    Ok(paths::content_cache_dir(app)?.join(shard).join(&sha1_lower))
}

pub async fn fetch(
    client: &reqwest::Client,
    app: &AppHandle,
    url: &str,
    sha1: Option<&str>,
) -> Result<PathBuf> {
    if let Some(hash) = sha1 {
        let path = blob_path(app, hash)?;
        if path.exists() {
            return Ok(path);
        }
    }

    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let got = hex::encode(hasher.finalize());
    if let Some(want) = sha1 {
        if !got.eq_ignore_ascii_case(want) {
            return Err(AppError::Other(format!(
                "checksum mismatch for {url}: expected {want}, got {got}"
            )));
        }
    }

    let blob = blob_path(app, &got)?;
    if !blob.exists() {
        if let Some(parent) = blob.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Write to a temp name then rename so concurrent installs never see a
        // half-written blob.
        let tmp = blob.with_extension("part");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &blob)?;
    }
    Ok(blob)
}

pub fn link_into(blob: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if dest.exists() {
        std::fs::remove_file(dest)?;
    }
    match std::fs::hard_link(blob, dest) {
        Ok(()) => Ok(()),
        // Different volume, or the OS/FS doesn't support links — copy instead.
        Err(_) => {
            std::fs::copy(blob, dest)?;
            Ok(())
        }
    }
}

pub async fn install_one(
    client: &reqwest::Client,
    app: &AppHandle,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
) -> Result<()> {
    let blob = fetch(client, app, url, sha1).await?;
    link_into(&blob, dest)
}

pub async fn install_all<F>(
    client: &reqwest::Client,
    app: &AppHandle,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(usize, usize),
{
    let total = tasks.len();
    let mut done = 0;
    on_progress(done, total);

    let mut stream = stream::iter(tasks)
        .map(|task| {
            let client = client.clone();
            let app = app.clone();
            async move {
                install_one(&client, &app, &task.url, &task.dest, task.sha1.as_deref()).await
            }
        })
        .buffer_unordered(concurrency);

    while let Some(result) = stream.next().await {
        result?;
        done += 1;
        on_progress(done, total);
    }
    Ok(())
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Unique blobs held in the cache.
    pub files: u64,
    /// Physical bytes the cache occupies (one copy per unique file).
    pub bytes: u64,
    /// Bytes that would be used if every instance kept its own copy.
    pub linked_bytes: u64,
    /// `linked_bytes - bytes`: disk saved by sharing.
    pub saved_bytes: u64,
}

pub fn stats(app: &AppHandle) -> Result<CacheStats> {
    use std::collections::HashMap;

    // Map inode -> size for every cached blob (one entry per unique file).
    let mut blob_size: HashMap<u64, u64> = HashMap::new();
    let cache = paths::content_cache_dir(app)?;
    collect_inodes(&cache, &mut blob_size);

    let mut cache_stats = CacheStats {
        files: blob_size.len() as u64,
        bytes: blob_size.values().sum(),
        ..Default::default()
    };

    // Sum the apparent size of every linked file across instances: this is what
    // per-instance copies would have cost.
    let instances = paths::instances_dir(app)?;
    if let Ok(entries) = std::fs::read_dir(&instances) {
        for entry in entries.flatten() {
            let game = entry.path().join("minecraft");
            for sub in ["mods", "resourcepacks", "shaderpacks", "datapacks"] {
                let mut sizes: HashMap<u64, u64> = HashMap::new();
                collect_inodes(&game.join(sub), &mut sizes);
                cache_stats.linked_bytes += sizes.values().sum::<u64>();
            }
        }
    }

    cache_stats.saved_bytes = cache_stats.linked_bytes.saturating_sub(cache_stats.bytes);
    Ok(cache_stats)
}

/// Record each file's (inode -> size) under `dir`, deduplicating shared inodes.
fn collect_inodes(dir: &Path, out: &mut std::collections::HashMap<u64, u64>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_inodes(&path, out);
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            out.insert(inode_of(&meta), meta.len());
        }
    }
}

#[cfg(unix)]
fn inode_of(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.ino()
}

#[cfg(not(unix))]
fn inode_of(meta: &std::fs::Metadata) -> u64 {
    // No stable inode on non-Unix here; fall back to size (approximate dedup).
    meta.len()
}
