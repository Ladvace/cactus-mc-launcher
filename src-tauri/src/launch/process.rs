use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::oneshot;

use super::{LaunchState, LogEvent, EVENT_LOG};
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;

fn emit_log(app: &AppHandle, id: &str, line: String) {
    let _ = app.emit(
        EVENT_LOG,
        LogEvent {
            instance_id: id.to_string(),
            line,
        },
    );
}

fn emit_status(app: &AppHandle, id: &str, state: &str, message: Option<String>) {
    let _ = app.emit(
        super::EVENT_STATUS,
        super::StatusEvent {
            instance_id: id.to_string(),
            state: state.to_string(),
            message,
        },
    );
}

/// Spawn the game process and monitor it in a background task: stream stdout/
/// stderr as log events, honor a kill signal, and update playtime on exit.
pub fn spawn_and_monitor(
    app: AppHandle,
    java: PathBuf,
    args: Vec<String>,
    game_dir: PathBuf,
    instance_id: String,
) -> Result<()> {
    let mut child = tokio::process::Command::new(&java)
        .args(&args)
        .current_dir(&game_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Other(format!("failed to start Java process: {e}")))?;

    emit_status(&app, &instance_id, "running", None);

    // Stream stdout.
    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let id = instance_id.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(&app, &id, line);
            }
        });
    }

    // Stream stderr.
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        let id = instance_id.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(&app, &id, line);
            }
        });
    }

    // Register a kill channel so `stop_instance` can terminate the game.
    let (kill_tx, kill_rx) = oneshot::channel::<()>();
    app.state::<LaunchState>().register(instance_id.clone(), kill_tx);

    let started = Instant::now();
    tokio::spawn(async move {
        let exit_message = tokio::select! {
            status = child.wait() => match status {
                Ok(s) => s.code().map(|c| format!("Exited with code {c}"))
                    .unwrap_or_else(|| "Exited".to_string()),
                Err(e) => format!("Process error: {e}"),
            },
            _ = kill_rx => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                "Stopped".to_string()
            }
        };

        // Record playtime.
        let elapsed = started.elapsed().as_secs();
        let store = app.state::<InstanceStore>();
        if let Some(mut inst) = store.get(&instance_id) {
            inst.total_playtime_seconds += elapsed;
            inst.last_played = Some(Utc::now());
            let _ = store.save(&app, &inst);
        }

        app.state::<LaunchState>().unregister(&instance_id);
        emit_status(&app, &instance_id, "exited", Some(exit_message));
    });

    Ok(())
}
