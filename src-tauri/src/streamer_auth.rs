//! Streamer sign-in for the hosted service. Supabase OAuth (Twitch/YouTube) via
//! the desktop loopback + PKCE flow: open the provider in the system browser,
//! catch the redirect on a localhost port, and exchange the code for a session.
//!
//! SUPABASE_URL + anon key are public values passed in from the frontend.

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::error::{AppError, Result};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // unix seconds
    pub user_id: String,
    pub display_name: String,
    pub provider: String,
}

/// A 64-char PKCE code verifier (256 bits from two UUIDs — no extra rng dep).
fn make_verifier() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn challenge(verifier: &str) -> String {
    let mut h = Sha256::new();
    h.update(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(h.finalize())
}

pub async fn login(supabase_url: &str, anon_key: &str, provider: &str) -> Result<Session> {
    let base = supabase_url.trim_end_matches('/');
    let verifier = make_verifier();
    let challenge = challenge(&verifier);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| AppError::Other(format!("loopback bind failed: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::Other(e.to_string()))?
        .port();
    let redirect = format!("http://127.0.0.1:{port}");

    let auth_url = format!(
        "{base}/auth/v1/authorize?provider={provider}&redirect_to={redirect}\
         &code_challenge={challenge}&code_challenge_method=s256"
    );
    open_browser(&auth_url);

    let code = accept_code(listener).await?;

    // Exchange the authorization code (PKCE).
    let resp = reqwest::Client::new()
        .post(format!("{base}/auth/v1/token?grant_type=pkce"))
        .header("apikey", anon_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "auth_code": code, "code_verifier": verifier }))
        .send()
        .await?
        .error_for_status()?;
    let tok: TokenResponse = resp.json().await?;

    let display_name = tok
        .user
        .user_metadata
        .and_then(|m| m.name.or(m.full_name).or(m.preferred_username).or(m.user_name))
        .unwrap_or_default();

    Ok(Session {
        access_token: tok.access_token,
        refresh_token: tok.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + tok.expires_in,
        user_id: tok.user.id,
        display_name,
        provider: provider.to_string(),
    })
}

/// Wait for the OAuth redirect on the loopback and return the `code` param.
async fn accept_code(listener: TcpListener) -> Result<String> {
    let (mut stream, _) = listener
        .accept()
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;

    let mut buf = vec![0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let request_line = req.lines().next().unwrap_or("");
    let target = request_line.split_whitespace().nth(1).unwrap_or("");
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");

    let mut code = None;
    let mut err = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "code" => code = Some(url_decode(v)),
                "error_description" => err = Some(url_decode(v)),
                "error" => err = err.or_else(|| Some(url_decode(v))),
                _ => {}
            }
        }
    }

    let body = "<html><body style='font-family:sans-serif;background:#17161a;color:#ebe8e0;text-align:center;padding-top:80px'>\
                <h2>Signed in ✓</h2><p>You can close this window and return to Drake Launcher.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;

    if let Some(e) = err {
        return Err(AppError::Other(format!("sign-in failed: {e}")));
    }
    code.ok_or_else(|| AppError::Other("no auth code returned".into()))
}

fn url_decode(s: &str) -> String {
    let bytes = s.replace('+', " ");
    let bytes = bytes.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    expires_in: i64,
    user: TokenUser,
}

#[derive(Deserialize)]
struct TokenUser {
    id: String,
    #[serde(default)]
    user_metadata: Option<UserMeta>,
}

#[derive(Deserialize)]
struct UserMeta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    user_name: Option<String>,
}
