//! Minecraft Server List Ping (SLP) — the same status query the multiplayer
//! server list uses. Opens a TCP connection, performs the 1.7+ handshake, and
//! reads back the server's status JSON (online/max players, MOTD, a player
//! sample). No server-side cooperation is needed.
//!
//! Protocol reference: <https://minecraft.wiki/w/Java_Edition_protocol/Server_List_Ping>.
//! SRV-record resolution is not done yet, so a server that only advertises its
//! real port via SRV must be pinged at that explicit `host:port`.

use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::error::{AppError, Result};

const DEFAULT_PORT: u16 = 25565;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The status JSON is small; cap it so a hostile server can't exhaust memory.
const MAX_RESPONSE_BYTES: i32 = 1 << 20;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: i64,
    pub max: i64,
    pub motd: String,
    pub players: Vec<String>,
    pub ping_ms: u64,
    /// The server's icon as a `data:image/png;base64,…` URI, if it broadcasts one
    /// (the same icon the vanilla multiplayer list shows). `None` if absent.
    pub favicon: Option<String>,
}

#[tauri::command]
pub async fn ping_server(address: String) -> Result<ServerStatus> {
    timeout(CONNECT_TIMEOUT, ping(&address))
        .await
        .map_err(|_| AppError::Other("server did not respond in time".into()))?
}

async fn ping(address: &str) -> Result<ServerStatus> {
    let (host, port) = split_host_port(address);
    let started = Instant::now();

    let mut stream = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| AppError::Other(format!("could not reach {host}:{port}: {e}")))?;

    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00); // handshake packet id
    write_varint(&mut handshake, -1); // protocol version (-1 = status query)
    write_string(&mut handshake, &host);
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake, 1); // next state: status
    write_framed(&mut stream, &handshake).await?;

    let mut request = Vec::new();
    write_varint(&mut request, 0x00); // status request packet id
    write_framed(&mut stream, &request).await?;

    let _packet_len = read_varint(&mut stream).await?;
    let packet_id = read_varint(&mut stream).await?;
    if packet_id != 0x00 {
        return Err(AppError::Other(format!(
            "unexpected status packet id {packet_id}"
        )));
    }
    let json_len = read_varint(&mut stream).await?;
    if !(0..=MAX_RESPONSE_BYTES).contains(&json_len) {
        return Err(AppError::Other("status response too large".into()));
    }
    let mut buf = vec![0u8; json_len as usize];
    stream.read_exact(&mut buf).await?;
    let ping_ms = started.elapsed().as_millis() as u64;

    let status: serde_json::Value = serde_json::from_slice(&buf)?;
    Ok(ServerStatus {
        online: status["players"]["online"].as_i64().unwrap_or(0),
        max: status["players"]["max"].as_i64().unwrap_or(0),
        motd: parse_motd(&status["description"]),
        players: status["players"]["sample"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        ping_ms,
        favicon: status["favicon"]
            .as_str()
            .filter(|s| s.starts_with("data:image/"))
            .map(str::to_owned),
    })
}

/// Split `host`, `host:port`, or a bare IPv4 into its host and port, defaulting
/// to 25565. IPv6 literals aren't handled (servers are addressed by hostname).
fn split_host_port(address: &str) -> (String, u16) {
    let address = address.trim();
    match address.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            (host.to_string(), port.parse().unwrap_or(DEFAULT_PORT))
        }
        _ => (address.to_string(), DEFAULT_PORT),
    }
}

/// The MOTD `description` is either a plain string or a chat component with a
/// root `text` and nested `extra` parts; flatten it to readable text.
fn parse_motd(description: &serde_json::Value) -> String {
    match description {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Object(_) => {
            let mut text = description["text"].as_str().unwrap_or("").to_string();
            if let Some(extra) = description["extra"].as_array() {
                for part in extra {
                    text.push_str(&parse_motd(part));
                }
            }
            text
        }
        _ => String::new(),
    }
}

/// Prefix a packet body with its length as a varint and send it.
async fn write_framed(stream: &mut TcpStream, body: &[u8]) -> Result<()> {
    let mut framed = Vec::with_capacity(body.len() + 5);
    write_varint(&mut framed, body.len() as i32);
    framed.extend_from_slice(body);
    stream.write_all(&framed).await?;
    Ok(())
}

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut remaining = value as u32;
    loop {
        let mut byte = (remaining & 0x7F) as u8;
        remaining >>= 7;
        if remaining != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if remaining == 0 {
            break;
        }
    }
}

fn write_string(buf: &mut Vec<u8>, value: &str) {
    write_varint(buf, value.len() as i32);
    buf.extend_from_slice(value.as_bytes());
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32> {
    let mut result: i32 = 0;
    for shift in 0..5 {
        let byte = stream.read_u8().await?;
        result |= ((byte & 0x7F) as i32) << (shift * 7);
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(AppError::Other("varint too long".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_the_port_when_absent() {
        assert_eq!(split_host_port("play.example.net"), ("play.example.net".into(), 25565));
    }

    #[test]
    fn reads_an_explicit_port() {
        assert_eq!(split_host_port("play.example.net:25570"), ("play.example.net".into(), 25570));
    }

    #[test]
    fn varint_roundtrip() {
        for value in [0i32, 1, 127, 128, 255, 25565, 2_097_151] {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            // decode
            let mut result = 0i32;
            for (shift, byte) in buf.iter().enumerate() {
                result |= ((byte & 0x7F) as i32) << (shift * 7);
                if byte & 0x80 == 0 {
                    break;
                }
            }
            assert_eq!(result, value, "varint roundtrip for {value}");
        }
    }

    #[test]
    fn flattens_a_chat_component_motd() {
        let description = serde_json::json!({
            "text": "Hello ",
            "extra": [{ "text": "world" }, { "text": "!" }]
        });
        assert_eq!(parse_motd(&description), "Hello world!");
    }
}
