pub mod microsoft;
pub mod xbox;

use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, Result};
use crate::paths;

// ===========================================================================
// Azure application client ID.
//
// Provided at build time via the `AZURE_CLIENT_ID` environment variable or a
// gitignored `src-tauri/.env` file (see `build.rs` and `.env.example`).
// Register a free app at https://portal.azure.com (Entra ID → App
// registrations), enable "Allow public client flows", and put the Application
// (client) ID in `.env`. Until then, Microsoft login is disabled and offline
// mode still works.
// ===========================================================================
const AZURE_CLIENT_ID: &str = match option_env!("AZURE_CLIENT_ID") {
    Some(id) => id,
    None => "",
};

fn ensure_client_id() -> Result<&'static str> {
    if AZURE_CLIENT_ID.is_empty() {
        return Err(AppError::Other(
            "Microsoft login isn't configured yet. Add your Azure client ID in \
             src-tauri/src/auth/mod.rs (AZURE_CLIENT_ID)."
                .into(),
        ));
    }
    Ok(AZURE_CLIENT_ID)
}

/// Whether Microsoft login is available (client ID configured).
pub fn is_configured() -> bool {
    !AZURE_CLIENT_ID.is_empty()
}

// --- Persisted account model ------------------------------------------------

/// A signed-in Microsoft/Minecraft account, including refresh material.
/// Never sent to the frontend as-is (see `AccountInfo`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// Minecraft profile UUID (no dashes).
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub ms_refresh_token: String,
    pub mc_access_token: String,
    /// Unix seconds when the Minecraft token expires.
    pub expires_at: i64,
}

/// Public account view (no secrets).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub kind: String,
}

fn to_info(account: &Account) -> AccountInfo {
    AccountInfo {
        id: account.id.clone(),
        username: account.username.clone(),
        uuid: account.uuid.clone(),
        kind: "microsoft".into(),
    }
}

/// Full account state for the frontend: accounts plus which one is active
/// (`activeId == null` means offline mode).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsState {
    pub accounts: Vec<AccountInfo>,
    pub active_id: Option<String>,
    pub microsoft_configured: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct AccountData {
    #[serde(default)]
    accounts: Vec<Account>,
    #[serde(default)]
    active_id: Option<String>,
}

/// Thread-safe account store persisted to `accounts.json`.
#[derive(Default)]
pub struct AccountStore {
    inner: Mutex<AccountData>,
}

impl AccountStore {
    pub fn load(&self, app: &AppHandle) -> Result<()> {
        let file = paths::data_dir(app)?.join("accounts.json");
        if file.exists() {
            if let Ok(text) = std::fs::read_to_string(&file) {
                if let Ok(data) = serde_json::from_str::<AccountData>(&text) {
                    *self.inner.lock().unwrap() = data;
                }
            }
        }
        Ok(())
    }

    fn persist(&self, app: &AppHandle, data: &AccountData) -> Result<()> {
        let file = paths::data_dir(app)?.join("accounts.json");
        std::fs::write(&file, serde_json::to_string_pretty(data)?)?;
        Ok(())
    }

    pub fn state(&self) -> AccountsState {
        let data = self.inner.lock().unwrap();
        AccountsState {
            accounts: data.accounts.iter().map(to_info).collect(),
            active_id: data.active_id.clone(),
            microsoft_configured: is_configured(),
        }
    }

    /// The active Microsoft account, or `None` for offline mode.
    pub fn active_account(&self) -> Option<Account> {
        let data = self.inner.lock().unwrap();
        let id = data.active_id.as_ref()?;
        data.accounts.iter().find(|account| &account.id == id).cloned()
    }

    /// Insert or replace an account by id.
    pub fn upsert(&self, app: &AppHandle, account: Account) -> Result<()> {
        let mut data = self.inner.lock().unwrap();
        if let Some(existing) = data.accounts.iter_mut().find(|entry| entry.id == account.id) {
            *existing = account;
        } else {
            data.accounts.push(account);
        }
        self.persist(app, &data)
    }

    pub fn set_active(&self, app: &AppHandle, id: Option<String>) -> Result<()> {
        let mut data = self.inner.lock().unwrap();
        // Ignore ids that don't exist (except None = offline).
        if let Some(id) = &id {
            if !data.accounts.iter().any(|account| &account.id == id) {
                return Err(AppError::Other("account not found".into()));
            }
        }
        data.active_id = id;
        self.persist(app, &data)
    }

    pub fn remove(&self, app: &AppHandle, id: &str) -> Result<()> {
        let mut data = self.inner.lock().unwrap();
        data.accounts.retain(|account| account.id != id);
        if data.active_id.as_deref() == Some(id) {
            data.active_id = None;
        }
        self.persist(app, &data)
    }
}

// --- Login orchestration ----------------------------------------------------

fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("cactus-launcher/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCodeEvent {
    user_code: String,
    verification_uri: String,
    message: String,
    expires_in: u64,
}

/// Run the full device-code login: get a code, poll for the token, then walk
/// the Xbox → XSTS → Minecraft chain. Emits `auth-device-code` so the UI can
/// show the code, and `auth-login-done` on success.
pub async fn login(app: &AppHandle) -> Result<AccountInfo> {
    let client_id = ensure_client_id()?;
    let http = http_client()?;

    let device_code = microsoft::request_device_code(&http, client_id).await?;
    let _ = app.emit(
        "auth-device-code",
        DeviceCodeEvent {
            user_code: device_code.user_code.clone(),
            verification_uri: device_code.verification_uri.clone(),
            message: device_code.message.clone(),
            expires_in: device_code.expires_in,
        },
    );

    let deadline = Utc::now().timestamp() + device_code.expires_in as i64;
    let mut interval = device_code.interval.max(1);

    let ms_token = loop {
        if Utc::now().timestamp() > deadline {
            return Err(AppError::Other("Login timed out. Please try again.".into()));
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
        match microsoft::poll_token(&http, client_id, &device_code.device_code).await? {
            microsoft::PollOutcome::Pending => continue,
            microsoft::PollOutcome::SlowDown => {
                interval += 5;
                continue;
            }
            microsoft::PollOutcome::Success(token) => break token,
        }
    };

    let account = full_chain(&http, ms_token).await?;

    let store = app.state::<AccountStore>();
    store.upsert(app, account.clone())?;
    store.set_active(app, Some(account.id.clone()))?;

    let _ = app.emit("auth-login-done", ());
    Ok(to_info(&account))
}

/// Walk an MS token through Xbox/XSTS/Minecraft to a full `Account`.
async fn full_chain(http: &reqwest::Client, ms_token: microsoft::MsToken) -> Result<Account> {
    let xbl = xbox::xbl_authenticate(http, &ms_token.access_token).await?;
    let xsts = xbox::xsts_authorize(http, &xbl.token).await?;
    let mc = xbox::minecraft_login(http, &xsts.user_hash, &xsts.token).await?;
    let profile = xbox::minecraft_profile(http, &mc.access_token).await?;

    Ok(Account {
        id: profile.id.clone(),
        username: profile.name,
        uuid: profile.id,
        ms_refresh_token: ms_token.refresh_token,
        mc_access_token: mc.access_token,
        expires_at: Utc::now().timestamp() + mc.expires_in,
    })
}

/// Refresh an account's Minecraft token using its stored MS refresh token.
pub async fn refresh_account(
    app: &AppHandle,
    http: &reqwest::Client,
    account: &Account,
) -> Result<Account> {
    let client_id = ensure_client_id()?;
    let ms_token = microsoft::refresh(http, client_id, &account.ms_refresh_token).await?;
    let refreshed = full_chain(http, ms_token).await?;
    app.state::<AccountStore>().upsert(app, refreshed.clone())?;
    Ok(refreshed)
}

/// The active account with a valid (refreshed if needed) token, or `None` for
/// offline mode.
pub async fn active_valid_account(
    app: &AppHandle,
    http: &reqwest::Client,
) -> Result<Option<Account>> {
    let store = app.state::<AccountStore>();
    let Some(account) = store.active_account() else {
        return Ok(None);
    };
    if account.expires_at - 60 > Utc::now().timestamp() {
        return Ok(Some(account));
    }
    let refreshed = refresh_account(app, http, &account).await?;
    Ok(Some(refreshed))
}
