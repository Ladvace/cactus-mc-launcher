//! Bring-your-own-key server tunnel via ngrok. The host pastes their own ngrok
//! authtoken; we open a TCP edge (embedded ngrok SDK — nothing extra to install)
//! that forwards to the local Minecraft server, and hand back the public address
//! for friends to Direct Connect to. No relay of ours, no port-forwarding, and
//! nothing for guests to install.

use std::sync::Mutex;

use futures::StreamExt;
use ngrok::prelude::*;
use tauri::State;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use crate::error::{AppError, Result};

struct Running {
    session: ngrok::Session,
    forwarder: JoinHandle<()>,
    address: String,
}

#[derive(Default)]
pub struct TunnelState {
    running: Mutex<Option<Running>>,
}

impl TunnelState {
    fn stop(&self) {
        if let Some(running) = self.running.lock().unwrap().take() {
            running.forwarder.abort();
            drop(running.session);
        }
    }

    fn address(&self) -> Option<String> {
        self.running.lock().unwrap().as_ref().map(|r| r.address.clone())
    }
}

#[tauri::command]
pub async fn tunnel_start(
    state: State<'_, TunnelState>,
    authtoken: String,
    port: u16,
) -> Result<String> {
    state.stop();

    let session = ngrok::Session::builder()
        .authtoken(authtoken)
        .connect()
        .await
        .map_err(|e| AppError::Other(format!("ngrok connect failed: {e}")))?;

    let mut tunnel = session
        .tcp_endpoint()
        .listen()
        .await
        .map_err(|e| AppError::Other(format!("ngrok tunnel failed: {e}")))?;

    // ngrok reports the edge as e.g. "tcp://0.tcp.ngrok.io:12345"; players enter
    // the bare host:port in Minecraft's Direct Connect.
    let address = tunnel
        .url()
        .strip_prefix("tcp://")
        .unwrap_or(tunnel.url())
        .to_string();

    let forwarder = tokio::spawn(async move {
        while let Some(Ok(mut conn)) = tunnel.next().await {
            tokio::spawn(async move {
                if let Ok(mut local) = TcpStream::connect(("127.0.0.1", port)).await {
                    let _ = copy_bidirectional(&mut conn, &mut local).await;
                }
            });
        }
    });

    *state.running.lock().unwrap() = Some(Running {
        session,
        forwarder,
        address: address.clone(),
    });
    Ok(address)
}

#[tauri::command]
pub fn tunnel_stop(state: State<'_, TunnelState>) {
    state.stop();
}

#[tauri::command]
pub fn tunnel_status(state: State<'_, TunnelState>) -> Option<String> {
    state.address()
}
