//! Sign in to the boards service using the player's existing Minecraft account,
//! via Mojang's `hasJoined` handshake — the same mechanism servers use. The MC
//! access token never leaves the launcher: we prove ownership to Mojang, and the
//! backend confirms it with Mojang and mints its own session token.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::auth::AccountStore;
use crate::error::{AppError, Result};

const MOJANG_JOIN: &str = "https://sessionserver.mojang.com/session/minecraft/join";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSession {
    pub token: String,
    pub uuid: String,
    pub name: String,
}

pub async fn login(app: &AppHandle, api_base: &str) -> Result<BoardSession> {
    let account = app
        .state::<AccountStore>()
        .active_account()
        .ok_or_else(|| AppError::Other("Sign in with your Microsoft account first.".into()))?;

    let base = api_base.trim_end_matches('/');
    let client = crate::http::client()?;

    // 1. Get a one-shot serverId challenge from the backend.
    let challenge: serde_json::Value = client
        .post(format!("{base}/v1/auth/challenge"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let server_id = challenge
        .get("serverId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| AppError::Other("bad challenge response".into()))?
        .to_string();

    // 2. Prove account ownership to Mojang (our access token stays local).
    let join = client
        .post(MOJANG_JOIN)
        .json(&serde_json::json!({
            "accessToken": account.mc_access_token,
            "selectedProfile": account.uuid,
            "serverId": server_id,
        }))
        .send()
        .await?;
    if !join.status().is_success() {
        return Err(AppError::Other(format!(
            "Minecraft session verification failed ({}). Try signing out and back in.",
            join.status()
        )));
    }

    // 3. Backend confirms with Mojang (hasJoined) and issues a session token.
    let resp = client
        .post(format!("{base}/v1/auth/verify"))
        .json(&serde_json::json!({ "username": account.username, "serverId": server_id }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("sign-in failed ({status}): {text}")));
    }

    let response_json: serde_json::Value = resp.json().await?;
    let get = |key: &str| {
        response_json
            .get(key)
            .and_then(|field| field.as_str())
            .unwrap_or_default()
            .to_string()
    };
    Ok(BoardSession {
        token: get("token"),
        uuid: get("uuid"),
        name: get("name"),
    })
}
