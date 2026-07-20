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

pub async fn list(app: &AppHandle) -> Result<FriendsList> {
    let client = crate::http::client()?;
    let account = crate::auth::active_valid_account(app, &client)
        .await?
        .ok_or_else(|| {
            AppError::Other("Sign in with your Microsoft account to see friends.".into())
        })?;

    let resp = client
        .get(FRIENDS_URL)
        .bearer_auth(&account.mc_access_token)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "couldn't load friends ({})",
            resp.status()
        )));
    }

    let data: RawFriends = resp.json().await?;
    Ok(FriendsList {
        friends: map(data.friends),
        incoming: map(data.incoming_requests),
        outgoing: map(data.outgoing_requests),
        empty: data.empty,
    })
}
