//! `src/lib/boardApi.ts`: HTTP to the Cactus board server, and the types it returns
//! (`Board`, `BoardCard`, `PresencePlayer`, … in types.ts).

use reqwest::{Method, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::snapshot::ImportResult;

/// Max size of a downloaded board snapshot before import.
const MAX_SNAPSHOT_BYTES: usize = 64_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub token: String,
    pub uuid: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardCard {
    pub handle: String,
    pub display_name: String,
    pub kind: String,
    pub owner_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardMessage {
    pub id: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardInstance {
    pub id: String,
    pub name: String,
    pub mc_version: Option<String>,
    pub mod_loader: Option<String>,
    pub changelog: Option<String>,
    pub created_at: String,
}

/// `Board` and `OwnedBoard` (the latter has no instances).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub handle: String,
    pub display_name: String,
    pub kind: String,
    pub owner_name: String,
    pub description: Option<String>,
    pub stream_url: Option<String>,
    pub server_address: Option<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub instances: Vec<BoardInstance>,
    #[serde(default)]
    pub messages: Vec<BoardMessage>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresencePlayer {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub status: String,
    pub server_address: Option<String>,
    pub mc_version: Option<String>,
    pub loader: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerSample {
    pub online: Option<f64>,
}

pub fn base() -> String {
    crate::commands::backend_base().trim_end_matches('/').to_string()
}

/// `boardApi.configured()`: a backend base was compiled in.
pub fn configured() -> bool {
    !base().is_empty()
}

/// `encodeURIComponent`.
pub fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn client() -> Result<reqwest::Client, String> {
    crate::http::client().map_err(|e| e.to_string())
}

/// `ensureOk`: the body's `error`, or "request failed (status)".
async fn ensure_ok(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    Err(body.get("error").and_then(Value::as_str).map_or_else(|| format!("request failed ({status})"), str::to_string))
}

async fn json_of<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    let empty = response.status().as_u16() == 204;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if empty || bytes.is_empty() {
        return serde_json::from_value(Value::Null).map_err(|e| e.to_string());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub async fn get<T: DeserializeOwned>(path: String) -> Result<T, String> {
    let response = client()?.get(format!("{}{path}", base())).send().await.map_err(|e| e.to_string())?;
    json_of(ensure_ok(response).await?).await
}

async fn send(method: &Method, path: &str, token: &str, body: &Option<Value>) -> Result<Response, String> {
    let mut request = client()?.request(method.clone(), format!("{}{path}", base())).bearer_auth(token);
    if let Some(body) = body {
        request = request.json(body);
    }
    request.send().await.map_err(|e| e.to_string())
}

/// What an authed call returns: its result, and the session minted if the old one had expired
/// (the store adopts it, as `onBoardSessionRefresh` does).
pub type Authed<T> = (Result<T, String>, Option<Session>);

/// `authed`: on a rejection that looks like an expired session, re-mint one with `board_login`
/// (no user interaction) and retry once.
pub async fn authed<T: DeserializeOwned>(host: crate::host::Host, token: String, method: Method, path: String, body: Option<Value>) -> Authed<T> {
    let mut fresh = None;
    let mut response = match send(&method, &path, &token, &body).await {
        Ok(response) => response,
        Err(e) => return (Err(e), None),
    };
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let bytes = response.bytes().await.unwrap_or_default();
        let message = serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|json| json.get("error").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default();
        let lower = message.to_lowercase();
        let expired = status == 401
            || status == 403
            || lower.contains("expired")
            || lower.find("invalid").is_some_and(|i| lower[i..].contains("session"));
        let original = if message.is_empty() { format!("request failed ({status})") } else { message };
        if !expired {
            return (Err(original), None);
        }
        let retried = match login(host).await {
            Ok(session) => {
                let result = send(&method, &path, &session.token, &body).await;
                fresh = Some(session);
                result
            }
            Err(_) => return (Err(original), None),
        };
        response = match retried {
            Ok(response) => response,
            Err(e) => return (Err(e), fresh),
        };
    }
    let out = match ensure_ok(response).await {
        Ok(response) => json_of(response).await,
        Err(e) => Err(e),
    };
    (out, fresh)
}

/// `board_login`, as a session the stores keep.
pub async fn login(host: crate::host::Host) -> Result<Session, String> {
    let session = crate::commands::board_login(host).await.map_err(|e| e.to_string())?;
    Ok(Session { token: session.token, uuid: session.uuid, name: session.name })
}

pub async fn search(query: String) -> Result<Vec<BoardCard>, String> {
    #[derive(Deserialize)]
    struct Reply {
        results: Vec<BoardCard>,
    }
    get::<Reply>(format!("/v1/boards?q={}", url_encode(&query))).await.map(|reply| reply.results)
}

pub async fn board(handle: String) -> Result<Board, String> {
    get(format!("/v1/boards/{}", url_encode(&handle))).await
}

pub async fn resolve_code(code: String) -> Result<String, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Reply {
        snapshot_id: String,
    }
    get::<Reply>(format!("/v1/codes/{}", url_encode(&code.trim().to_lowercase()))).await.map(|reply| reply.snapshot_id)
}

/// `importSnapshot`: download the blob (bounded: it's a remote board's) and import it.
pub async fn import_snapshot(host: crate::host::Host, id: String) -> Result<ImportResult, String> {
    let response = client()?
        .get(format!("{}/v1/snapshots/{}/blob", base(), url_encode(&id)))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("couldn't download snapshot ({})", response.status().as_u16()));
    }
    if response.content_length().is_some_and(|n| n as usize > MAX_SNAPSHOT_BYTES) {
        return Err("this snapshot is too large to import".into());
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err("this snapshot is too large to import".into());
    }
    crate::commands::import_setup(host, bytes.to_vec()).await.map_err(|e| e.to_string())
}

pub async fn my_boards(host: crate::host::Host, token: String) -> Authed<Vec<Board>> {
    #[derive(Deserialize)]
    struct Reply {
        boards: Vec<Board>,
    }
    let (result, fresh) = authed::<Reply>(host, token, Method::GET, "/v1/boards/me".into(), None).await;
    (result.map(|reply| reply.boards), fresh)
}

pub struct NewBoard {
    pub handle: String,
    pub display_name: String,
    pub kind: String,
    pub description: Option<String>,
    pub stream_url: Option<String>,
    pub server_address: Option<String>,
}

pub async fn create_board(host: crate::host::Host, token: String, board: NewBoard) -> Authed<Value> {
    let mut body = json!({ "handle": board.handle, "displayName": board.display_name, "kind": board.kind });
    for (key, value) in [("description", board.description), ("streamUrl", board.stream_url), ("serverAddress", board.server_address)] {
        if let Some(value) = value {
            body[key] = Value::String(value);
        }
    }
    authed(host, token, Method::POST, "/v1/boards".into(), Some(body)).await
}

pub async fn update_board(host: crate::host::Host, token: String, handle: String, patch: Value) -> Authed<()> {
    authed(host, token, Method::PATCH, format!("/v1/boards/{}", url_encode(&handle)), Some(patch)).await
}

pub async fn delete_board(host: crate::host::Host, token: String, handle: String) -> Authed<()> {
    authed(host, token, Method::DELETE, format!("/v1/boards/{}", url_encode(&handle)), None).await
}

pub async fn post_message(host: crate::host::Host, token: String, handle: String, body: String) -> Authed<()> {
    authed(host, token, Method::POST, format!("/v1/boards/{}/messages", url_encode(&handle)), Some(json!({ "body": body }))).await
}

pub async fn delete_message(host: crate::host::Host, token: String, handle: String, id: String) -> Authed<()> {
    authed(host, token, Method::DELETE, format!("/v1/boards/{}/messages/{}", url_encode(&handle), url_encode(&id)), None).await
}

pub async fn list_presence(host: crate::host::Host, token: String) -> Authed<Vec<PresencePlayer>> {
    #[derive(Deserialize)]
    struct Reply {
        players: Vec<PresencePlayer>,
    }
    let (result, fresh) = authed::<Reply>(host, token, Method::GET, "/v1/presence".into(), None).await;
    (result.map(|reply| reply.players), fresh)
}

pub async fn set_presence(host: crate::host::Host, token: String, body: Value) -> Authed<()> {
    authed(host, token, Method::PUT, "/v1/presence".into(), Some(body)).await
}

pub async fn clear_presence(host: crate::host::Host, token: String) -> Authed<()> {
    authed(host, token, Method::DELETE, "/v1/presence".into(), None).await
}

pub async fn publish(
    host: crate::host::Host,
    instance_id: String,
    format: String,
    token: String,
    board_handle: Option<String>,
    changelog: String,
) -> Result<String, String> {
    let changelog = Some(changelog.trim().to_string()).filter(|changelog| !changelog.is_empty());
    crate::commands::publish_setup(host, instance_id, format, token, board_handle, None, changelog).await.map_err(|e| e.to_string())
}

pub async fn server_history(address: String, hours: u32) -> Result<Vec<ServerSample>, String> {
    #[derive(Deserialize)]
    struct Reply {
        samples: Vec<ServerSample>,
    }
    get::<Reply>(format!("/v1/servers/history?address={}&hours={hours}", url_encode(&address))).await.map(|reply| reply.samples)
}
