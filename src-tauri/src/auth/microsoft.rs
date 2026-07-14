use serde::Deserialize;

use crate::error::{AppError, Result};

const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// A device-code challenge to show the user.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

/// A Microsoft OAuth token pair.
#[derive(Debug, Clone)]
pub struct MsToken {
    pub access_token: String,
    pub refresh_token: String,
    /// MS access-token lifetime; we track the Minecraft token expiry instead.
    #[allow(dead_code)]
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// One outcome of polling the token endpoint during device-code login.
pub enum PollOutcome {
    Pending,
    SlowDown,
    Success(MsToken),
}

/// Request a device code to start the login flow.
pub async fn request_device_code(client: &reqwest::Client, client_id: &str) -> Result<DeviceCode> {
    let resp = client
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "failed to start Microsoft login: {body}"
        )));
    }
    Ok(resp.json().await?)
}

/// Poll the token endpoint once. The caller loops, sleeping `interval` seconds
/// between calls (increasing it on `SlowDown`).
pub async fn poll_token(
    client: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<PollOutcome> {
    let resp: TokenResponse = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", DEVICE_CODE_GRANT),
            ("device_code", device_code),
        ])
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.error.as_deref() {
        return match err {
            "authorization_pending" => Ok(PollOutcome::Pending),
            "slow_down" => Ok(PollOutcome::SlowDown),
            "expired_token" => Err(AppError::Other(
                "The login code expired. Please try again.".into(),
            )),
            "authorization_declined" => {
                Err(AppError::Other("Login was declined.".into()))
            }
            other => Err(AppError::Other(format!("Microsoft login failed: {other}"))),
        };
    }

    match (resp.access_token, resp.refresh_token, resp.expires_in) {
        (Some(access_token), Some(refresh_token), Some(expires_in)) => {
            Ok(PollOutcome::Success(MsToken {
                access_token,
                refresh_token,
                expires_in,
            }))
        }
        _ => Err(AppError::Other(
            "Microsoft returned an incomplete token response".into(),
        )),
    }
}

/// Exchange a refresh token for a fresh access token (Microsoft may rotate the
/// refresh token, so callers should persist the returned one).
pub async fn refresh(
    client: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<MsToken> {
    let resp: TokenResponse = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.error {
        return Err(AppError::Other(format!(
            "Session expired, please sign in again ({err})"
        )));
    }

    match (resp.access_token, resp.refresh_token, resp.expires_in) {
        (Some(access_token), Some(refresh_token), Some(expires_in)) => Ok(MsToken {
            access_token,
            refresh_token,
            expires_in,
        }),
        _ => Err(AppError::Other("incomplete refresh response".into())),
    }
}
