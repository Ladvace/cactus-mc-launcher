use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;

use super::{LaunchState, LogEvent, ServerMsg, ServerState, EVENT_LOG};
use crate::error::{AppError, Result};
use crate::instance::store::InstanceStore;
use crate::paths;

/// Collapse the (very long) classpath argument for readable launch logging.
fn concise_args(args: &[String]) -> String {
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if (args[i] == "-cp" || args[i] == "-classpath") && i + 1 < args.len() {
            let count = args[i + 1].split(sep).count();
            out.push("-cp".to_string());
            out.push(format!("<{count} jars>"));
            i += 2;
        } else {
            out.push(args[i].clone());
            i += 1;
        }
    }
    out.join(" ")
}

/// Human-readable exit description, including the signal name on Unix.
fn describe_exit(status: &std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("Exited with code {code}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            let name = match sig {
                4 => "SIGILL",
                5 => "SIGTRAP",
                6 => "SIGABRT",
                7 | 10 => "SIGBUS",
                9 => "SIGKILL",
                11 => "SIGSEGV",
                _ => "signal",
            };
            return format!("Crashed ({name}, signal {sig})");
        }
    }
    "Exited".to_string()
}

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
    eprintln!(
        "[launch] {} {}\n[launch] cwd: {}",
        java.display(),
        concise_args(&args),
        game_dir.display()
    );

    // Capture the child's stderr to a file so native crashes (which bypass our
    // line-buffered reader) are preserved for diagnosis.
    let stderr_path = paths::instance_dir(&app, &instance_id)?.join("launch-stderr.log");
    let stderr_file = std::fs::File::create(&stderr_path)?;

    let mut cmd = tokio::process::Command::new(&java);
    cmd.args(&args)
        .current_dir(&game_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file));

    // Strip inherited `DYLD_*` overrides. In dev, `cargo run` sets
    // DYLD_FALLBACK_LIBRARY_PATH to Rust build dirs; inheriting it in the game
    // process breaks macOS OpenGL loading (GL dispatch LOAD_ERROR -> SIGABRT).
    for (key, _) in std::env::vars() {
        if key.starts_with("DYLD_") {
            cmd.env_remove(key);
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| {
            let hint = if cfg!(target_os = "macos") {
                " (older Minecraft versions need Rosetta 2 on Apple Silicon — install it with: softwareupdate --install-rosetta --agree-to-license)"
            } else {
                ""
            };
            AppError::Other(format!("failed to start Java process: {e}{hint}"))
        })?;

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

    // (stderr goes to launch-stderr.log — see Stdio::from above.)

    // Register a kill channel so `stop_instance` can terminate the game.
    let (kill_tx, kill_rx) = oneshot::channel::<()>();
    app.state::<LaunchState>().register(instance_id.clone(), kill_tx);

    let started = Instant::now();
    tokio::spawn(async move {
        let exit_message = tokio::select! {
            status = child.wait() => match status {
                Ok(s) => describe_exit(&s),
                Err(e) => format!("Process error: {e}"),
            },
            _ = kill_rx => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                "Stopped".to_string()
            }
        };

        eprintln!("[launch] instance {instance_id} {exit_message}");

        // Surface captured stderr (incl. native crashes) in the in-app Logs tab.
        if let Ok(err) = std::fs::read_to_string(&stderr_path) {
            let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
            if !lines.is_empty() {
                // Keep the last chunk to avoid flooding the UI on huge dumps.
                let start = lines.len().saturating_sub(200);
                for line in &lines[start..] {
                    emit_log(&app, &instance_id, line.to_string());
                }
            }
        }

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

/// Spawn a dedicated server and monitor it. Unlike the game client, the server
/// keeps stdin open so console commands can be piped in, and "stop" is issued
/// gracefully before falling back to a hard kill.
pub fn spawn_server(
    app: AppHandle,
    java: PathBuf,
    args: Vec<String>,
    run_dir: PathBuf,
    instance_id: String,
) -> Result<()> {
    eprintln!(
        "[server] {} {}\n[server] cwd: {}",
        java.display(),
        concise_args(&args),
        run_dir.display()
    );

    let mut cmd = tokio::process::Command::new(&java);
    cmd.args(&args)
        .current_dir(&run_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, _) in std::env::vars() {
        if key.starts_with("DYLD_") {
            cmd.env_remove(key);
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Other(format!("failed to start server process: {e}")))?;

    emit_status(&app, &instance_id, "running", None);

    // Stream stdout and stderr both into the console log.
    for pipe in [child.stdout.take().map(Pipe::Out), child.stderr.take().map(Pipe::Err)]
        .into_iter()
        .flatten()
    {
        let app = app.clone();
        let id = instance_id.clone();
        tokio::spawn(async move {
            match pipe {
                Pipe::Out(o) => {
                    let mut lines = BufReader::new(o).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        emit_log(&app, &id, line);
                    }
                }
                Pipe::Err(e) => {
                    let mut lines = BufReader::new(e).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        emit_log(&app, &id, line);
                    }
                }
            }
        });
    }

    let mut stdin = child.stdin.take();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ServerMsg>();
    app.state::<ServerState>().register(instance_id.clone(), tx);

    let started = Instant::now();
    tokio::spawn(async move {
        let exit_message = loop {
            tokio::select! {
                status = child.wait() => {
                    break match status {
                        Ok(s) => describe_exit(&s),
                        Err(e) => format!("Process error: {e}"),
                    };
                }
                msg = rx.recv() => {
                    match msg {
                        Some(ServerMsg::Line(line)) => {
                            if let Some(si) = stdin.as_mut() {
                                let _ = si.write_all(format!("{line}\n").as_bytes()).await;
                                let _ = si.flush().await;
                            }
                        }
                        Some(ServerMsg::Stop) => {
                            if let Some(si) = stdin.as_mut() {
                                let _ = si.write_all(b"stop\n").await;
                                let _ = si.flush().await;
                            }
                            // Let the server shut itself down; child.wait() above
                            // resolves once it exits.
                        }
                        None => {
                            let _ = child.start_kill();
                            let _ = child.wait().await;
                            break "Stopped".to_string();
                        }
                    }
                }
            }
        };

        eprintln!("[server] instance {instance_id} {exit_message}");

        let elapsed = started.elapsed().as_secs();
        let store = app.state::<InstanceStore>();
        if let Some(mut inst) = store.get(&instance_id) {
            inst.total_playtime_seconds += elapsed;
            inst.last_played = Some(Utc::now());
            let _ = store.save(&app, &inst);
        }

        app.state::<ServerState>().unregister(&instance_id);
        emit_status(&app, &instance_id, "exited", Some(exit_message));
    });

    Ok(())
}

enum Pipe {
    Out(tokio::process::ChildStdout),
    Err(tokio::process::ChildStderr),
}
