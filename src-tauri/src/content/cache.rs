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

const MAX_ATTEMPTS: u32 = 4;

/// Download `url` into the content cache, streaming to a temp file (never
/// buffering the whole thing in memory) and, on a dropped connection, resuming
/// from the partial via an HTTP `Range` request instead of starting over.
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

    // Unique temp per fetch: this fetch's own retries still resume from it, but
    // two concurrent installs of the same file never fight over one `.part`
    // (which would interleave into a checksum failure).
    let cache_dir = paths::content_cache_dir(app)?;
    let tmp = match sha1 {
        Some(hash) => {
            let blob = blob_path(app, hash)?;
            if let Some(parent) = blob.parent() {
                std::fs::create_dir_all(parent)?;
            }
            blob.with_extension(format!("part-{}", uuid::Uuid::new_v4()))
        }
        None => {
            std::fs::create_dir_all(&cache_dir)?;
            cache_dir.join(format!(".dl-{}.part", uuid::Uuid::new_v4()))
        }
    };

    let mut last_err: Option<AppError> = None;
    for attempt in 0..MAX_ATTEMPTS {
        let resume_from = if sha1.is_some() {
            std::fs::metadata(&tmp).map(|meta| meta.len()).unwrap_or(0)
        } else {
            0
        };

        match stream_to_file(client, url, &tmp, resume_from).await {
            Ok(()) => {
                let got = file_sha1(&tmp).ok_or_else(|| {
                    AppError::Other(format!("could not read downloaded file for {url}"))
                })?;
                if let Some(want) = sha1 {
                    if !got.eq_ignore_ascii_case(want) {
                        // The (possibly resumed) file is corrupt — discard and
                        // restart cleanly.
                        let _ = std::fs::remove_file(&tmp);
                        last_err = Some(AppError::Other(format!(
                            "checksum mismatch for {url}: expected {want}, got {got}"
                        )));
                        continue;
                    }
                }
                let blob = blob_path(app, &got)?;
                if blob.exists() {
                    let _ = std::fs::remove_file(&tmp);
                } else {
                    if let Some(parent) = blob.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    if std::fs::rename(&tmp, &blob).is_err() {
                        // Rare (same-dir rename); on a transient lock, stage via a
                        // fresh temp and rename so a concurrent reader never sees a
                        // half-written blob at the final path.
                        let staging = blob.with_extension(format!("part-{}", uuid::Uuid::new_v4()));
                        std::fs::copy(&tmp, &staging)?;
                        std::fs::rename(&staging, &blob)?;
                        let _ = std::fs::remove_file(&tmp);
                    }
                }
                return Ok(blob);
            }
            Err(error) => {
                // Keep the partial for the next attempt to resume from.
                last_err = Some(error);
                if attempt + 1 < MAX_ATTEMPTS {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        300 * (attempt as u64 + 1),
                    ))
                    .await;
                }
            }
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Err(last_err.unwrap_or_else(|| AppError::Other(format!("download failed: {url}"))))
}

/// Stream the response body to `tmp`, appending when the server honours a
/// `Range` resume (206) and overwriting when it doesn't (200) or the range is
/// invalid (416).
async fn stream_to_file(
    client: &reqwest::Client,
    url: &str,
    tmp: &Path,
    resume_from: u64,
) -> Result<()> {
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut request = client.get(url);
    if resume_from > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }
    let response = request.send().await?;

    if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // Our partial is stale/complete-but-wrong; drop it and start over.
        let _ = std::fs::remove_file(tmp);
        return Err(AppError::Other("resume range rejected, restarting".into()));
    }
    let append = resume_from > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let response = response.error_for_status()?;

    let mut opts = tokio::fs::OpenOptions::new();
    opts.create(true).write(true);
    if append {
        opts.append(true);
    } else {
        opts.truncate(true);
    }
    let mut file = opts.open(tmp).await?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    Ok(())
}

/// SHA-1 of a file, read incrementally so large downloads aren't buffered.
fn file_sha1(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(hex::encode(hasher.finalize()))
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
    /// `linked_bytes` minus the real disk those files occupy (hard-linked files
    /// share one inode, counted once): the disk saved by sharing.
    pub saved_bytes: u64,
}

pub fn stats(app: &AppHandle) -> Result<CacheStats> {
    use std::collections::HashMap;
    use tauri::Manager;

    // Cache footprint on disk (unique blobs; may include versions no longer
    // linked into any instance).
    let mut blob_size: HashMap<u64, u64> = HashMap::new();
    collect_inodes(&paths::content_cache_dir(app)?, &mut blob_size);

    // Walk each instance's content folders. `apparent` is what per-instance
    // copies would cost (each instance counted separately); `physical` is the
    // real disk they occupy — hard links across instances share an inode and are
    // counted once. Saved = apparent - physical.
    let mut apparent: u64 = 0;
    let mut physical: HashMap<u64, u64> = HashMap::new();
    let store = app.state::<crate::instance::store::InstanceStore>();
    for instance in store.list() {
        let Ok(game) = paths::instance_game_dir(app, &instance.id) else {
            continue;
        };
        for sub in ["mods", "resourcepacks", "shaderpacks", "datapacks"] {
            let mut per: HashMap<u64, u64> = HashMap::new();
            collect_inodes(&game.join(sub), &mut per);
            apparent += per.values().sum::<u64>();
            physical.extend(per);
        }
    }

    Ok(CacheStats {
        files: blob_size.len() as u64,
        bytes: blob_size.values().sum(),
        linked_bytes: apparent,
        saved_bytes: apparent.saturating_sub(physical.values().sum::<u64>()),
    })
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

// No stable per-file id on non-Unix std; fall back to size (approximate dedup).
#[cfg(not(unix))]
fn inode_of(meta: &std::fs::Metadata) -> u64 {
    meta.len()
}
