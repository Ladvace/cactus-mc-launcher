//! The one state the whole window views. It holds what the Svelte stores held
//! (`src/lib/stores/*.svelte.ts`): instances, settings, accounts, launch and install progress, the
//! `ui` store's open overlays, and each screen's own state in its module.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use rune::{Cx, El, KeyEvent, Tx, Waker};
use serde::Deserialize;
use serde_json::Value;

use crate::auth::{AccountInfo, AccountStore};
use crate::core::fx::{Fx, Job};
use crate::core::i18n;
use crate::core::images::Images;
use crate::core::prefs::Prefs;
use crate::core::toast::Toasts;
use crate::instance::Instance;
use crate::instance::store::InstanceStore;
use crate::launch::{LaunchState, ServerState};
use crate::settings::{Settings, SettingsStore};
use crate::theme::Theme;
use crate::{overlays, screens, shell, tunnel};

pub const IDENTIFIER: &str = "com.ladvace.cactuslauncher";

/// The SvelteKit routes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Home,
    Browse,
    Servers,
    /// `/share`
    Community,
    /// `/share/creator`
    ShareCreator,
    /// `/share/[handle]`
    ShareBoard(String),
    Achievements,
    Settings,
    Instance(String),
}

impl Route {
    /// For the dock's active state: `/share/…` routes all light the Community button.
    pub fn section(&self) -> &'static str {
        match self {
            Route::Home => "/",
            Route::Browse => "/browse",
            Route::Servers => "/servers",
            Route::Community | Route::ShareCreator | Route::ShareBoard(_) => "/share",
            Route::Achievements => "/achievements",
            Route::Settings => "/settings",
            Route::Instance(_) => "/instance",
        }
    }
}

/// `InstanceRuntime` in launch.svelte.ts.
#[derive(Clone, Debug, Default)]
pub struct LaunchRuntime {
    /// "" (idle) | preparing | downloading | launching | running | exited | error
    pub state: String,
    pub message: Option<String>,
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub logs: VecDeque<String>,
}

const MAX_LOG_LINES: usize = 2000;

impl LaunchRuntime {
    pub fn is_busy(&self) -> bool {
        matches!(self.state.as_str(), "preparing" | "downloading" | "launching")
    }
    pub fn is_running(&self) -> bool {
        self.state == "running"
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Progress {
    pub current: usize,
    pub total: usize,
    pub message: String,
}

pub fn to_percent(current: usize, total: usize) -> Option<u32> {
    (total > 0).then(|| ((current as f64 / total as f64) * 100.0).round() as u32)
}

/// install.svelte.ts: modpack downloads by instance, and one not yet tied to an instance.
#[derive(Default)]
pub struct Install {
    pub progress: BTreeMap<String, Progress>,
    pub pending: Option<Progress>,
    /// Downloads started whose first progress event hasn't arrived (the dock shows them at once).
    pub starting: usize,
    start_tokens: VecDeque<u64>,
    next_token: u64,
}

impl Install {
    /// `installStore.beginStart`: a download kicked off; the dock shows it until its first event.
    pub fn begin_start(&mut self) -> u64 {
        let token = self.next_token;
        self.next_token += 1;
        self.start_tokens.push_back(token);
        self.starting = self.start_tokens.len();
        token
    }
    pub fn end_start(&mut self, token: u64) {
        self.start_tokens.retain(|queued| *queued != token);
        self.starting = self.start_tokens.len();
    }
    /// Events don't carry the token, so a first event retires the oldest one.
    fn consume_start(&mut self) {
        self.start_tokens.pop_front();
        self.starting = self.start_tokens.len();
    }

    pub fn is_installing(&self, id: &str) -> bool {
        self.progress.contains_key(id)
    }
    pub fn percent(&self, id: &str) -> Option<u32> {
        self.progress.get(id).and_then(|progress| to_percent(progress.current, progress.total))
    }
    pub fn any_active(&self) -> bool {
        self.starting > 0 || self.pending.is_some() || !self.progress.is_empty()
    }
    pub fn overall_message(&self) -> String {
        self.progress
            .values()
            .next()
            .or(self.pending.as_ref())
            .map_or_else(|| "Installing…".into(), |progress| progress.message.clone())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
    pub expires_in: u64,
    #[serde(skip)]
    pub authorizing: bool,
}

/// accounts.svelte.ts
#[derive(Default)]
pub struct Accounts {
    pub list: Vec<AccountInfo>,
    pub active_id: Option<String>,
    pub microsoft_configured: bool,
    pub loaded: bool,
    pub device_code: Option<DeviceCode>,
    pub login_error: Option<String>,
    pub logging_in: bool,
    cancelling: bool,
}

pub type PickSticker = Rc<dyn Fn(&mut App, String)>;

/// ui.svelte.ts: which overlays are open. Open them through the `App` methods below.
#[derive(Default)]
pub struct Ui {
    pub create_instance_open: bool,
    pub accounts_open: bool,
    pub command_palette_open: bool,
    pub changelog_open: bool,
    pub crafting_open: bool,
    /// Instance id and the window point the menu opens at.
    pub instance_menu: Option<(String, f32, f32)>,
    pub group_menu: Option<(String, f32, f32)>,
    pub sticker_picker: Option<(String, PickSticker)>,
    /// The instance the group picker is choosing a group for.
    pub group_for: Option<String>,
    pub folder_open: Option<String>,
    /// Pre-selected install target when Browse opens from an instance's "Find mods".
    pub browse_instance_id: Option<String>,
    /// LauncherImport (home's "Import from launcher").
    pub launcher_import_open: bool,
    /// `cactus:onboarded` in prefs.
    pub onboarded: bool,
}

pub struct App {
    pub fx: Fx,
    jobs: Receiver<Job>,
    _runtime: tokio::runtime::Runtime,
    pub images: Images,
    pub prefs: Prefs,
    /// Seconds since start, from `poll` (virtual when headless).
    pub now: f64,

    pub route: Route,
    history: Vec<Route>,

    pub instances: Vec<Instance>,
    pub instances_loaded: bool,
    pub settings: Settings,
    pub settings_loaded: bool,
    pub accounts: Accounts,
    pub launch: BTreeMap<String, LaunchRuntime>,
    pub install: Install,
    pub toasts: Toasts,
    pub ui: Ui,

    pub shell: shell::State,
    pub home: screens::home::State,
    pub instance_page: screens::instance::State,
    pub browse: screens::browse::State,
    pub servers: screens::servers::State,
    pub achievements: screens::achievements::State,
    pub share: screens::share::State,
    pub settings_page: screens::settings::State,
    pub dock: overlays::dock::State,
    pub account_modal: overlays::accounts::State,
    pub modals: overlays::modals::State,
    pub extras: overlays::extras::State,
}

pub fn init() -> App {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("tokio runtime");

    let host = crate::host::Host::new(IDENTIFIER);
    host.manage(InstanceStore::default());
    host.manage(SettingsStore::default());
    host.manage(LaunchState::default());
    host.manage(ServerState::default());
    host.manage(AccountStore::default());
    host.manage(tunnel::TunnelState::default());
    // Stores read their files once at start. Tauri's setup aborted on a failure; here the app
    // still opens, and says what it couldn't read once it's up.
    let load_errors: Vec<(&str, String)> = [
        ("startup.instancesUnreadable", host.state::<InstanceStore>().load(&host)),
        ("startup.settingsUnreadable", host.state::<SettingsStore>().load(&host)),
        ("startup.accountsUnreadable", host.state::<AccountStore>().load(&host)),
    ]
    .into_iter()
    .filter_map(|(key, result)| result.err().map(|error| (key, error.to_string())))
    .collect();
    let instances_unreadable = load_errors.iter().any(|(key, _)| *key == "startup.instancesUnreadable");

    let (fx, jobs) = Fx::new(host.clone(), runtime.handle().clone());
    let sink = fx.clone();
    host.set_event_sink(move |name, payload| {
        let name = name.to_string();
        sink.post(move |app| app.on_event(&name, payload));
    });

    let prefs = Prefs::load_for(&host);
    let settings = crate::commands::get_settings(host.state());
    i18n::set_locale(&settings.language);
    let instances = crate::commands::list_instances(host.state());

    let mut app = App {
        fx,
        jobs,
        _runtime: runtime,
        images: Images::default(),
        now: 0.0,
        route: Route::Home,
        history: Vec::new(),
        instances_loaded: true,
        settings,
        settings_loaded: true,
        accounts: Accounts::default(),
        launch: BTreeMap::new(),
        install: Install::default(),
        toasts: Toasts::default(),
        // The webview's localStorage didn't come along: someone with instances has been through
        // onboarding already.
        // An unreadable instances folder isn't a first run either.
        ui: Ui { onboarded: prefs.read("cactus:onboarded", !instances.is_empty() || instances_unreadable), ..Ui::default() },
        prefs,
        instances,
        shell: Default::default(),
        home: Default::default(),
        instance_page: Default::default(),
        browse: Default::default(),
        servers: Default::default(),
        achievements: Default::default(),
        share: Default::default(),
        settings_page: Default::default(),
        dock: Default::default(),
        account_modal: Default::default(),
        modals: Default::default(),
        extras: Default::default(),
    };
    for (key, error) in load_errors {
        app.toasts.error(i18n::tf(key, &[("error", &error)]));
    }
    app.refresh_accounts();
    screens::on_enter(&mut app);
    overlays::on_start(&mut app);
    app
}

pub fn setup(app: &mut App, waker: Waker) {
    app.fx.set_waker(waker.clone());
    app.images.init(rune::ImageCache::new(waker));
}

pub fn poll(app: &mut App, now: f64, tx: &mut Tx) -> bool {
    app.now = now;
    let mut changed = false;
    while let Ok(job) = app.jobs.try_recv() {
        job(app);
        changed = true;
    }
    let (expired, next) = app.toasts.tick(now);
    if let Some(secs) = next {
        tx.wake_after(std::time::Duration::from_secs_f64(secs.max(0.05)));
    }
    changed | expired | shell::poll(app, now, tx)
}

pub fn on_key(app: &mut App, key: &KeyEvent, tx: &mut Tx) -> bool {
    overlays::on_key(app, key, tx)
}

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    shell::view(app, cx)
}

impl App {
    pub fn theme(&self) -> Theme {
        Theme::from_settings(&self.settings)
    }

    // Navigation (goto).

    pub fn go(&mut self, route: Route) {
        if route == self.route {
            return;
        }
        let prev = std::mem::replace(&mut self.route, route);
        self.history.push(prev);
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        screens::on_enter(self);
    }

    pub fn back(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.route = prev;
            screens::on_enter(self);
        } else {
            self.go(Route::Home);
        }
    }

    // Instances (instances.svelte.ts).

    pub fn instance(&self, id: &str) -> Option<&Instance> {
        self.instances.iter().find(|instance| instance.id == id)
    }

    pub fn refresh_instances(&mut self) {
        self.fx.blocking(|host| crate::commands::list_instances(host.state()), |app, list| {
            app.instances = list;
            app.instances_loaded = true;
        });
    }

    /// `instancesStore.update`: patch, then refresh; `done` gets the result.
    pub fn update_instance(
        &mut self,
        id: &str,
        patch: crate::instance::UpdateInstance,
        done: impl FnOnce(&mut App, crate::error::Result<Instance>) + Send + 'static,
    ) {
        let id = id.to_string();
        self.fx.blocking(
            move |host| crate::commands::update_instance(host.clone(), host.state(), id, patch),
            move |app, result| {
                app.refresh_instances();
                done(app, result);
            },
        );
    }

    /// The `src` of an instance's icon for `Images::get`: a data URI as is, a file name made
    /// absolute against the instance folder.
    pub fn instance_icon_src(&self, inst: &Instance) -> Option<String> {
        let icon = inst.icon.as_deref().filter(|name| !name.is_empty())?;
        if icon.starts_with("data:") || icon.starts_with("http") || icon.starts_with('/') {
            return Some(icon.to_string());
        }
        crate::paths::instance_dir(&self.fx.host, &inst.id)
            .ok()
            .map(|dir| dir.join(icon).to_string_lossy().into_owned())
    }

    // Settings (settings.svelte.ts).

    /// Persist first, then apply, as `settingsStore.save` does.
    pub fn save_settings(&mut self, next: Settings) {
        let host = self.fx.host.clone();
        let applied = next.clone();
        match crate::commands::save_settings(host.clone(), host.state(), next) {
            Ok(()) => {
                i18n::set_locale(&applied.language);
                self.settings = applied;
            }
            Err(e) => self.toasts.error(e.to_string()).ignore(),
        }
    }

    /// Change one setting: `app.update_settings(|settings| settings.ui_sounds = false)`.
    pub fn update_settings(&mut self, f: impl FnOnce(&mut Settings)) {
        let mut next = self.settings.clone();
        f(&mut next);
        self.save_settings(next);
    }

    // Launching (launch.svelte.ts).

    pub fn is_busy(&self, id: &str) -> bool {
        self.launch.get(id).is_some_and(|runtime| runtime.is_busy())
    }
    pub fn is_running(&self, id: &str) -> bool {
        self.launch.get(id).is_some_and(|runtime| runtime.is_running())
    }

    pub fn launch_instance(&mut self, id: &str) {
        let runtime = self.launch.entry(id.to_string()).or_default();
        runtime.logs.clear();
        runtime.state = "preparing".into();
        runtime.message = Some("Starting…".into());
        runtime.current = 0;
        runtime.total = 0;
        let host = self.fx.host.clone();
        let launch_id = id.to_string();
        self.fx.spawn(
            async move { crate::commands::launch_instance(host.clone(), host.state(), host.state(), host.state(), host.state(), launch_id).await },
            {
                let id = id.to_string();
                move |app, result| {
                    if let Err(e) = result {
                        let runtime = app.launch.entry(id).or_default();
                        runtime.state = "error".into();
                        runtime.message = Some(e.to_string());
                    }
                }
            },
        );
    }

    pub fn stop_instance(&mut self, id: &str) {
        let host = self.fx.host.clone();
        let _ = crate::commands::stop_instance(host.state(), host.state(), id.to_string());
    }

    // Accounts (accounts.svelte.ts).

    pub fn refresh_accounts(&mut self) {
        let stored = crate::commands::get_accounts(self.fx.host.state());
        self.accounts.list = stored.accounts;
        self.accounts.active_id = stored.active_id;
        self.accounts.microsoft_configured = stored.microsoft_configured;
        self.accounts.loaded = true;
    }

    pub fn active_account(&self) -> Option<&AccountInfo> {
        let id = self.accounts.active_id.as_deref()?;
        self.accounts.list.iter().find(|account| account.id == id)
    }

    pub fn active_name(&self) -> String {
        self.active_account().map_or_else(
            || if self.settings.offline_username.is_empty() { "Player".into() } else { self.settings.offline_username.clone() },
            |account| account.username.clone(),
        )
    }

    pub fn login_microsoft(&mut self) {
        let accounts = &mut self.accounts;
        accounts.logging_in = true;
        accounts.login_error = None;
        accounts.device_code = None;
        accounts.cancelling = false;
        let host = self.fx.host.clone();
        self.fx.spawn(async move { crate::commands::login_microsoft(host).await }, |app, result| {
            if let Err(e) = result {
                if !app.accounts.cancelling {
                    app.accounts.login_error = Some(e.to_string());
                }
            }
            app.refresh_accounts();
            let accounts = &mut app.accounts;
            accounts.logging_in = false;
            accounts.device_code = None;
            accounts.cancelling = false;
        });
    }

    pub fn cancel_login(&mut self) {
        if !self.accounts.logging_in {
            return;
        }
        self.accounts.cancelling = true;
        self.accounts.device_code = None;
        self.accounts.login_error = None;
        crate::commands::cancel_login();
    }

    pub fn set_active_account(&mut self, id: Option<String>) {
        let host = self.fx.host.clone();
        match crate::commands::set_active_account(host.clone(), host.state(), id.clone()) {
            Ok(_) => self.accounts.active_id = id,
            Err(e) => self.toasts.error(e.to_string()).ignore(),
        }
    }

    pub fn remove_account(&mut self, id: &str) {
        let host = self.fx.host.clone();
        if let Err(e) = crate::commands::remove_account(host.clone(), host.state(), id.to_string()) {
            self.toasts.error(e.to_string());
        }
        self.refresh_accounts();
    }

    // Overlays (ui.svelte.ts).

    pub fn open_instance_menu(&mut self, id: &str, x: f32, y: f32) {
        self.ui.group_menu = None;
        self.ui.instance_menu = Some((id.to_string(), x, y));
    }
    pub fn open_group_menu(&mut self, name: &str, x: f32, y: f32) {
        self.ui.instance_menu = None;
        self.ui.group_menu = Some((name.to_string(), x, y));
    }
    pub fn open_sticker_picker(&mut self, title: impl Into<String>, on_pick: impl Fn(&mut App, String) + 'static) {
        self.ui.instance_menu = None;
        self.ui.group_menu = None;
        self.ui.sticker_picker = Some((title.into(), Rc::new(on_pick)));
    }
    pub fn open_group_picker(&mut self, instance_id: &str) {
        self.ui.instance_menu = None;
        self.ui.group_for = Some(instance_id.to_string());
    }

    pub fn set_onboarded(&mut self) {
        self.ui.onboarded = true;
        self.prefs.write("cactus:onboarded", true);
    }

    // Backend events (`listen(...)` in the stores), then each module's own.

    fn on_event(&mut self, name: &str, payload: Value) {
        match name {
            "launch-status" => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct StatusEvent { instance_id: String, state: String, message: Option<String> }
                if let Ok(event) = serde_json::from_value::<StatusEvent>(payload.clone()) {
                    let runtime = self.launch.entry(event.instance_id).or_default();
                    runtime.state = event.state;
                    runtime.message = event.message;
                    if runtime.state == "exited" || runtime.state == "error" {
                        runtime.current = 0;
                        runtime.total = 0;
                        runtime.stage.clear();
                        self.refresh_instances();
                    }
                }
            }
            "launch-progress" => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct ProgressEvent { instance_id: String, stage: String, current: usize, total: usize }
                if let Ok(event) = serde_json::from_value::<ProgressEvent>(payload.clone()) {
                    let runtime = self.launch.entry(event.instance_id).or_default();
                    runtime.stage = event.stage;
                    runtime.current = event.current;
                    runtime.total = event.total;
                }
            }
            "launch-log" => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct LogEvent { instance_id: String, line: String }
                if let Ok(event) = serde_json::from_value::<LogEvent>(payload.clone()) {
                    let runtime = self.launch.entry(event.instance_id).or_default();
                    runtime.logs.push_back(event.line);
                    while runtime.logs.len() > MAX_LOG_LINES {
                        runtime.logs.pop_front();
                    }
                }
            }
            "modpack-progress" => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct ModpackEvent { instance_id: Option<String>, current: usize, total: usize, message: String }
                if let Ok(event) = serde_json::from_value::<ModpackEvent>(payload.clone()) {
                    let done = event.message == "Done";
                    let progress = Progress { current: event.current, total: event.total, message: event.message };
                    match event.instance_id {
                        None => self.install.pending = (!done).then_some(progress),
                        Some(id) => {
                            self.install.pending = None;
                            if done {
                                self.install.progress.remove(&id);
                                self.refresh_instances();
                            } else {
                                let fresh = self.install.progress.insert(id, progress).is_none();
                                if fresh {
                                    self.install.consume_start();
                                    self.refresh_instances();
                                }
                            }
                        }
                    }
                }
            }
            "auth-device-code" => {
                if let Ok(code) = serde_json::from_value::<DeviceCode>(payload.clone()) {
                    self.accounts.device_code = Some(code);
                }
            }
            "auth-login-done" => {
                if let Some(code) = &mut self.accounts.device_code {
                    code.authorizing = true;
                }
            }
            _ => {}
        }
        screens::on_event(self, name, &payload);
        overlays::on_event(self, name, &payload);
    }
}

/// `toasts.error(..)` returns the toast id; most callers don't need it.
pub trait Ignore {
    fn ignore(self);
}
impl Ignore for u64 {
    fn ignore(self) {}
}
