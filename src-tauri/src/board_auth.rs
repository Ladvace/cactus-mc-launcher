//! Sign in to the boards service using the player's existing Minecraft account,
//! via a Mojang-signed player certificate. The launcher fetches its certificate
//! from Mojang (from the user's own IP — Mojang's Akamai WAF blocks Cloudflare's
//! IPs, so the backend can't call Mojang itself), signs the backend's one-time
//! challenge with the certificate's private key, and sends the backend the public
//! key + Mojang's signature over it. The backend verifies both fully offline. The
//! MC access token never leaves the launcher.

use base64::Engine;
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey};
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::Serialize;
use sha2::Sha256;
use tauri::{AppHandle, Manager};

use crate::auth::AccountStore;
use crate::error::{AppError, Result};

const MOJANG_CERTS: &str = "https://api.minecraftservices.com/player/certificates";

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

    // 1. One-time challenge (nonce) from the backend.
    let challenge: serde_json::Value = client
        .post(format!("{base}/v1/auth/challenge"))
        .timeout(std::time::Duration::from_secs(20))
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

    // 2. Fetch this account's Mojang-signed certificate (from our own IP).
    let cert_resp = client
        .post(MOJANG_CERTS)
        .bearer_auth(&account.mc_access_token)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !cert_resp.status().is_success() {
        return Err(AppError::Other(format!(
            "couldn't get your Minecraft certificate ({}). Try signing out and back in.",
            cert_resp.status()
        )));
    }
    let cert: serde_json::Value = cert_resp.json().await?;
    let str_at = |value: &serde_json::Value, path: &[&str]| -> Result<String> {
        let mut cur = value;
        for key in path {
            cur = cur
                .get(key)
                .ok_or_else(|| AppError::Other(format!("certificate missing {key}")))?;
        }
        cur.as_str()
            .map(str::to_string)
            .ok_or_else(|| AppError::Other("bad certificate field".into()))
    };
    let private_pem = str_at(&cert, &["keyPair", "privateKey"])?;
    let public_pem = str_at(&cert, &["keyPair", "publicKey"])?;
    let public_key_signature = str_at(&cert, &["publicKeySignatureV2"])?;
    let expires_iso = str_at(&cert, &["expiresAt"])?;
    let expires_ms = chrono::DateTime::parse_from_rfc3339(&expires_iso)
        .map_err(|err| AppError::Other(format!("bad certificate expiry: {err}")))?
        .timestamp_millis();

    // Re-export the public key as canonical SPKI DER — the exact bytes Mojang
    // signed over (Java's PublicKey.getEncoded()). Mojang mislabels the PEM, so
    // parse it as SPKI or PKCS#1, then emit SPKI.
    let public_key_der_b64 = public_key_spki_b64(&public_pem)?;
    let signature = sign_nonce(&private_pem, server_id.as_bytes())?;

    // 3. Send the proof for offline verification.
    let resp = client
        .post(format!("{base}/v1/auth/verify"))
        .timeout(std::time::Duration::from_secs(20))
        .json(&serde_json::json!({
            "serverId": server_id,
            "uuid": account.uuid,
            "name": account.username,
            "publicKey": public_key_der_b64,
            "publicKeySignature": public_key_signature,
            "expiresAt": expires_ms,
            "signature": signature,
        }))
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

/// The base64 body of a PEM block (the DER), with header/footer and whitespace
/// stripped.
fn pem_body(pem: &str) -> String {
    pem.lines()
        .filter(|line| !line.starts_with("-----"))
        .flat_map(|line| line.split_whitespace())
        .collect()
}

/// Parse the certificate's public key (SPKI or PKCS#1) and re-emit it as
/// canonical SPKI DER, base64-encoded — the form Mojang's signature is over.
fn public_key_spki_b64(public_pem: &str) -> Result<String> {
    let der = base64::engine::general_purpose::STANDARD
        .decode(pem_body(public_pem))
        .map_err(|err| AppError::Other(format!("bad public key: {err}")))?;
    let key = RsaPublicKey::from_public_key_der(&der)
        .or_else(|_| RsaPublicKey::from_pkcs1_der(&der))
        .map_err(|err| AppError::Other(format!("parse public key: {err}")))?;
    let spki = key
        .to_public_key_der()
        .map_err(|err| AppError::Other(format!("encode public key: {err}")))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(spki.as_bytes()))
}

/// Sign `data` with the certificate's RSA private key (RSASSA-PKCS1-v1_5,
/// SHA-256), returning a base64 signature. Mojang mislabels the PEM, so decode
/// the DER and try PKCS#8 then PKCS#1.
fn sign_nonce(private_pem: &str, data: &[u8]) -> Result<String> {
    let der = base64::engine::general_purpose::STANDARD
        .decode(pem_body(private_pem))
        .map_err(|err| AppError::Other(format!("bad private key: {err}")))?;
    let key = RsaPrivateKey::from_pkcs8_der(&der)
        .or_else(|_| RsaPrivateKey::from_pkcs1_der(&der))
        .map_err(|err| AppError::Other(format!("parse private key: {err}")))?;
    let signature = SigningKey::<Sha256>::new(key).sign(data);
    Ok(base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()))
}
