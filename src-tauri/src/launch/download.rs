use std::path::{Path, PathBuf};

use futures::stream::{self, StreamExt};
use sha1::{Digest, Sha1};

use crate::error::{AppError, Result};

/// One file to fetch: destination path, URL, and optional expected SHA-1.
#[derive(Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    /// Mark the file executable after writing (Unix).
    pub executable: bool,
}

/// Compute the hex SHA-1 of a file, or `None` if it can't be read.
fn file_sha1(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Some(hex::encode(hasher.finalize()))
}

/// True if the file exists and matches the expected hash (or exists, when no
/// hash is given). Used to skip already-downloaded files.
pub fn is_valid(path: &Path, expected_sha1: Option<&str>) -> bool {
    if !path.exists() {
        return false;
    }
    match expected_sha1 {
        Some(want) => file_sha1(path).map(|got| got.eq_ignore_ascii_case(want)).unwrap_or(false),
        None => true,
    }
}

/// Download a single file, verifying its hash and setting the exec bit if asked.
pub async fn download_one(client: &reqwest::Client, task: &DownloadTask) -> Result<()> {
    if is_valid(&task.dest, task.sha1.as_deref()) {
        return Ok(());
    }
    if let Some(parent) = task.dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bytes = client
        .get(&task.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    if let Some(want) = &task.sha1 {
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(want) {
            return Err(AppError::Other(format!(
                "checksum mismatch for {}: expected {want}, got {got}",
                task.url
            )));
        }
    }

    std::fs::write(&task.dest, &bytes)?;

    #[cfg(unix)]
    if task.executable {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&task.dest, perms)?;
    }

    Ok(())
}

/// Download many files concurrently, invoking `on_progress(done, total)` after
/// each completes. Fails fast on the first error.
pub async fn download_all<F>(
    client: &reqwest::Client,
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
            async move { download_one(&client, &task).await }
        })
        .buffer_unordered(concurrency);

    while let Some(result) = stream.next().await {
        result?;
        done += 1;
        on_progress(done, total);
    }

    Ok(())
}
