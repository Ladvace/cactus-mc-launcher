use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, Result};

const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// An Xbox token plus the user hash needed to build the Minecraft identity token.
pub struct XboxAuth {
    pub token: String,
    pub user_hash: String,
}

#[derive(Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    xerr: Option<i64>,
}

fn extract(resp: XboxResponse) -> Result<XboxAuth> {
    let user_hash = resp
        .display_claims
        .xui
        .into_iter()
        .next()
        .map(|x| x.uhs)
        .ok_or_else(|| AppError::Other("Xbox response missing user hash".into()))?;
    Ok(XboxAuth {
        token: resp.token,
        user_hash,
    })
}

/// Authenticate a Microsoft access token with Xbox Live.
pub async fn xbl_authenticate(client: &reqwest::Client, ms_access_token: &str) -> Result<XboxAuth> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp = client.post(XBL_URL).json(&body).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "Xbox Live authentication failed ({status}): {}",
            body.chars().take(300).collect::<String>()
        )));
    }
    extract(resp.json().await?)
}

/// Authorize the Xbox token for Minecraft services (XSTS).
pub async fn xsts_authorize(client: &reqwest::Client, xbl_token: &str) -> Result<XboxAuth> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp = client.post(XSTS_URL).json(&body).send().await?;
    let status = resp.status();
    if !status.is_success() {
        // Map the well-known XSTS error codes to friendly messages.
        if let Ok(err) = resp.json::<XstsError>().await {
            let msg = match err.xerr {
                Some(2148916233) => {
                    "This Microsoft account has no Xbox profile. Create one at xbox.com, then try again."
                }
                Some(2148916235) => "Xbox Live is not available in your account's region.",
                Some(2148916236) | Some(2148916237) => {
                    "This account needs adult verification (South Korea)."
                }
                Some(2148916238) => {
                    "This account is a child and must be added to a Family by an adult."
                }
                _ => "Xbox (XSTS) authorization failed.",
            };
            return Err(AppError::Other(msg.into()));
        }
        return Err(AppError::Other("Xbox (XSTS) authorization failed".into()));
    }
    extract(resp.json().await?)
}

#[derive(Deserialize)]
struct McLoginResponse {
    access_token: String,
    #[serde(default)]
    expires_in: i64,
}

/// Minecraft access token and its lifetime (seconds).
pub struct McAuth {
    pub access_token: String,
    pub expires_in: i64,
}

/// Log in to Minecraft services with the XSTS token + user hash.
pub async fn minecraft_login(
    client: &reqwest::Client,
    user_hash: &str,
    xsts_token: &str,
) -> Result<McAuth> {
    let body = json!({
        "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")
    });

    let resp = client.post(MC_LOGIN_URL).json(&body).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        // A newly-registered Azure app must be approved by Mojang for the
        // Minecraft API — until then this endpoint returns "Invalid app
        // registration". Give an actionable message instead of the raw body.
        if body.contains("Invalid app registration") {
            return Err(AppError::Other(
                "This launcher's Azure app isn't approved for Minecraft yet. \
                 Newly-created apps must be allowlisted by Mojang — apply at \
                 https://aka.ms/mce-reviewappid (this can take a while). Offline \
                 mode works in the meantime."
                    .into(),
            ));
        }
        return Err(AppError::Other(format!(
            "Minecraft services login failed ({status}): {}",
            body.chars().take(300).collect::<String>()
        )));
    }
    let parsed: McLoginResponse = resp.json().await?;
    Ok(McAuth {
        access_token: parsed.access_token,
        expires_in: if parsed.expires_in > 0 {
            parsed.expires_in
        } else {
            86400
        },
    })
}

/// A Minecraft account profile.
#[derive(Debug, Clone, Deserialize)]
pub struct McProfile {
    pub id: String,
    pub name: String,
}

/// Fetch the Minecraft profile (fails if the account does not own the game).
pub async fn minecraft_profile(client: &reqwest::Client, mc_access_token: &str) -> Result<McProfile> {
    let resp = client
        .get(MC_PROFILE_URL)
        .bearer_auth(mc_access_token)
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Other(
            "This Microsoft account does not own Minecraft: Java Edition.".into(),
        ));
    }
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "Failed to fetch Minecraft profile ({status}): {}",
            body.chars().take(300).collect::<String>()
        )));
    }
    Ok(resp.json().await?)
}
