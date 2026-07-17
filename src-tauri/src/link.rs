//! "Cactus Link" — client side of the zero-install relay tunnel (see
//! PLAY_TOGETHER.md). Bridges a Minecraft TCP stream over a WebSocket to the
//! LinkRoom Durable Object so friends on other networks can join a self-hosted
//! server without port-forwarding or a VPN.
//!
//! Host: keep a control WebSocket open; for each guest the relay announces, open
//! a data socket and pipe it to the local Minecraft server. Guest: listen on a
//! local port and pipe each incoming Minecraft connection out over the relay.

use std::sync::Mutex;

use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::error::{AppError, Result};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// The tunnel tasks currently running (host control loop and/or guest accept
/// loop), aborted together by `link_stop`.
#[derive(Default)]
pub struct LinkState {
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl LinkState {
    fn track(&self, handle: JoinHandle<()>) {
        self.tasks.lock().unwrap().push(handle);
    }

    fn stop(&self) {
        for handle in self.tasks.lock().unwrap().drain(..) {
            handle.abort();
        }
    }
}

#[derive(Deserialize)]
struct OpenStream {
    stream: String,
}

/// Start hosting: open the control socket and return a short code to share. Each
/// guest the relay announces gets bridged to the local server on `port`.
#[tauri::command]
pub async fn link_host(state: State<'_, LinkState>, api_base: String, port: u16) -> Result<String> {
    let code = short_code();
    let (control, _) = connect_async(link_url(&api_base, &code, "role=host"))
        .await
        .map_err(|e| AppError::Other(format!("link connect failed: {e}")))?;

    let handle = tokio::spawn(host_loop(control, api_base, code.clone(), port));
    state.track(handle);
    Ok(code)
}

/// Join a session: listen on a local port (returned) and tunnel each Minecraft
/// connection to that port out to the host. Point the game at 127.0.0.1:<port>.
#[tauri::command]
pub async fn link_join(state: State<'_, LinkState>, api_base: String, code: String) -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();

    let handle = tokio::spawn(guest_loop(listener, api_base, code));
    state.track(handle);
    Ok(port)
}

#[tauri::command]
pub fn link_stop(state: State<'_, LinkState>) {
    state.stop();
}

async fn host_loop(mut control: Socket, api_base: String, code: String, port: u16) {
    while let Some(Ok(message)) = control.next().await {
        let Message::Text(text) = message else { continue };
        let Ok(open) = serde_json::from_str::<OpenStream>(&text) else { continue };

        let url = link_url(&api_base, &code, &format!("role=host&stream={}", open.stream));
        tokio::spawn(async move {
            if let Ok((ws, _)) = connect_async(url).await {
                if let Ok(tcp) = TcpStream::connect(("127.0.0.1", port)).await {
                    bridge(ws, tcp).await;
                }
            }
        });
    }
}

async fn guest_loop(listener: TcpListener, api_base: String, code: String) {
    while let Ok((tcp, _)) = listener.accept().await {
        let stream = uuid::Uuid::new_v4().simple().to_string();
        let url = link_url(&api_base, &code, &format!("role=guest&stream={stream}"));
        tokio::spawn(async move {
            if let Ok((ws, _)) = connect_async(url).await {
                bridge(ws, tcp).await;
            }
        });
    }
}

/// Pipe bytes both ways between a relay WebSocket and a local TCP connection
/// until either side closes.
async fn bridge(ws: Socket, tcp: TcpStream) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (mut tcp_rd, mut tcp_wr) = tokio::io::split(tcp);

    let ws_to_tcp = async {
        while let Some(Ok(message)) = ws_rx.next().await {
            match message {
                Message::Binary(data) => {
                    if tcp_wr.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        let _ = tcp_wr.shutdown().await;
    };

    let tcp_to_ws = async {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            match tcp_rd.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if ws_tx.send(Message::Binary(buf[..count].to_vec().into())).await.is_err() {
                        break;
                    }
                }
            }
        }
        let _ = ws_tx.send(Message::Close(None)).await;
    };

    tokio::select! {
        _ = ws_to_tcp => {}
        _ = tcp_to_ws => {}
    }
}

/// Turn the boards HTTP base into the ws(s) URL for a link room.
fn link_url(api_base: &str, code: &str, query: &str) -> String {
    let ws = api_base
        .trim_end_matches('/')
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);
    format!("{ws}/v1/link/{code}?{query}")
}

fn short_code() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..6].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_wss_room_url() {
        assert_eq!(
            link_url("https://api.example.com/", "abc123", "role=host"),
            "wss://api.example.com/v1/link/abc123?role=host"
        );
    }

    #[test]
    fn short_code_is_six_chars() {
        assert_eq!(short_code().len(), 6);
    }
}
