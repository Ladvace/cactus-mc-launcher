//! Minecraft friends list via Mojang's API (unstable, Java 26.2+). Called
//! client-side with the account's own Minecraft token, so the token never
//! leaves the launcher and it isn't affected by the backend's IP block.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::{AppError, Result};

const FRIENDS_URL: &str = "https://api.minecraftservices.com/friends";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsList {
    pub friends: Vec<Friend>,
    pub incoming: Vec<Friend>,
    pub outgoing: Vec<Friend>,
    pub empty: bool,
}

#[derive(Deserialize)]
struct RawFriend {
    #[serde(default, rename = "profileId")]
    profile_id: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFriends {
    #[serde(default)]
    friends: Vec<RawFriend>,
    #[serde(default)]
    incoming_requests: Vec<RawFriend>,
    #[serde(default)]
    outgoing_requests: Vec<RawFriend>,
    #[serde(default)]
    empty: bool,
}

fn map(list: Vec<RawFriend>) -> Vec<Friend> {
    list.into_iter()
        .map(|f| Friend {
            profile_id: f.profile_id,
            name: f.name,
        })
        .collect()
}

fn to_list(data: RawFriends) -> FriendsList {
    FriendsList {
        friends: map(data.friends),
        incoming: map(data.incoming_requests),
        outgoing: map(data.outgoing_requests),
        empty: data.empty,
    }
}

async fn token(app: &AppHandle, client: &reqwest::Client) -> Result<String> {
    crate::auth::active_valid_account(app, client)
        .await?
        .map(|account| account.mc_access_token)
        .ok_or_else(|| AppError::Other("Sign in with your Microsoft account first.".into()))
}

pub async fn list(app: &AppHandle) -> Result<FriendsList> {
    let client = crate::http::client()?;
    let token = token(app, &client).await?;
    let resp = client
        .get(FRIENDS_URL)
        .bearer_auth(&token)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "couldn't load friends ({})",
            resp.status()
        )));
    }
    Ok(to_list(resp.json().await?))
}

/// Add/accept (`add = true`) or remove/decline/cancel (`add = false`) a friend,
/// by `name` or `profile_id`. Returns the updated list. Accepting an incoming
/// request is the same as adding by that profile id.
pub async fn update(
    app: &AppHandle,
    name: Option<String>,
    profile_id: Option<String>,
    add: bool,
) -> Result<FriendsList> {
    let client = crate::http::client()?;
    let token = token(app, &client).await?;

    let mut body = serde_json::Map::new();
    if let Some(name) = name.filter(|s| !s.trim().is_empty()) {
        body.insert("name".into(), serde_json::json!(name.trim()));
    }
    if let Some(id) = profile_id.filter(|s| !s.trim().is_empty()) {
        body.insert("profileId".into(), serde_json::json!(id));
    }
    if body.is_empty() {
        return Err(AppError::Other("Enter a player name.".into()));
    }
    body.insert(
        "updateType".into(),
        serde_json::json!(if add { "ADD" } else { "REMOVE" }),
    );

    let resp = client
        .put(FRIENDS_URL)
        .bearer_auth(&token)
        .json(&body)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("friend update failed ({status}): {text}")));
    }
    Ok(to_list(resp.json().await?))
}
