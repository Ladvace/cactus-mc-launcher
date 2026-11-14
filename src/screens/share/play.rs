//! The "Play together" tab: `FriendsList.svelte`, `PresencePanel.svelte` and the `presence`
//! store (`presence.svelte.ts`) that polls who's online.

use std::time::{Duration, Instant};

use rune::tok::TextStyle;
use rune::{Color, Cursor, Cx, El, TextEdit, col, el, row, text};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::api::{self, PresencePlayer};
use super::{Tab, adopt, card, field, link, login, note, select, text_input, token};
use crate::instance::ModLoader;
use crate::app::{App, Ignore, Route};
use crate::core::i18n::{t as tr, tf};
use crate::friends::{FriendsList, FriendsPrefs};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const PRESENCE_KEY: &str = "cactus:presence";
const POLL: Duration = Duration::from_secs(20);

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Persisted {
    enabled: bool,
    status: String,
    server_address: String,
    mc_version: String,
    loader: String,
}

#[derive(Default)]
pub struct State {
    // presence store
    saved: Persisted,
    players: Vec<PresencePlayer>,
    loading: bool,
    error: Option<String>,
    polling: bool,
    timer: u64,
    last_poll: Option<Instant>,
    // PresencePanel's own fields
    status: TextEdit,
    address: TextEdit,
    mc_version: TextEdit,
    loader: String,
    save_gen: u64,
    filter_version: String,
    filter_loader: String,
    // FriendsList
    friends: Option<FriendsList>,
    friend_prefs: Option<FriendsPrefs>,
    friends_loading: bool,
    friends_failed: bool,
    busy: bool,
    add_name: TextEdit,
}

pub fn load_prefs(app: &mut App) {
    let persisted: Persisted = app.prefs.read(PRESENCE_KEY, Persisted::default());
    let play = &mut app.share.play;
    play.status = TextEdit::new(persisted.status.clone());
    play.address = TextEdit::new(persisted.server_address.clone());
    play.mc_version = TextEdit::new(persisted.mc_version.clone());
    play.loader = persisted.loader.clone();
    play.saved = persisted;
}

fn persist(app: &mut App) {
    let saved = app.share.play.saved.clone();
    app.prefs.write(PRESENCE_KEY, saved);
}

/// The tab mounted: sign in, poll now and every 20 s, load friends.
pub fn open(app: &mut App) {
    super::ensure_login(app);
    poll(app);
    app.share.play.timer += 1;
    tick(app, app.share.play.timer);
    load_friends(app);
}

/// The tab unmounted: stop polling and drop off the list.
pub fn close(app: &mut App) {
    app.share.play.timer += 1;
    if app.share.play.saved.enabled {
        go_offline(app);
    }
}

fn tick(app: &mut App, generation: u64) {
    app.fx.spawn(async move { tokio::time::sleep(Duration::from_secs(1)).await }, move |app, _| {
        if app.share.play.timer != generation {
            return;
        }
        if app.route != Route::Community || app.share.active != Tab::Play {
            return close(app);
        }
        if app.share.play.last_poll.is_none_or(|at| at.elapsed() >= POLL) {
            poll(app);
        }
        tick(app, generation);
    });
}

fn presence_body(persisted: &Persisted) -> serde_json::Value {
    json!({ "status": persisted.status, "serverAddress": persisted.server_address, "mcVersion": persisted.mc_version, "loader": persisted.loader })
}

/// One cycle at a time: heartbeat (when online), then the list.
pub fn poll(app: &mut App) {
    let Some(token) = token(app) else { return };
    let play = &mut app.share.play;
    if play.polling {
        return;
    }
    play.polling = true;
    play.loading = play.players.is_empty();
    play.last_poll = Some(Instant::now());
    let heartbeat_body = play.saved.enabled.then(|| presence_body(&play.saved));
    let host = app.fx.host.clone();
    let fut = async move {
        let mut fresh = None;
        let mut token = token;
        if let Some(body) = heartbeat_body {
            let (result, refreshed) = api::set_presence(host.clone(), token.clone(), body).await;
            if let Some(refreshed) = &refreshed {
                token = refreshed.token.clone();
            }
            fresh = refreshed.or(fresh);
            if let Err(e) = result {
                return (Err(e), fresh);
            }
        }
        let (result, refreshed) = api::list_presence(host, token).await;
        (result, refreshed.or(fresh))
    };
    app.fx.spawn(fut, |app, (result, fresh)| {
        adopt(app, fresh);
        let play = &mut app.share.play;
        match result {
            Ok(players) => {
                play.players = players;
                play.error = None;
            }
            Err(e) => play.error = Some(e),
        }
        play.loading = false;
        play.polling = false;
    });
}

fn heartbeat(app: &mut App) {
    let Some(token) = token(app) else { return };
    let body = presence_body(&app.share.play.saved);
    let host = app.fx.host.clone();
    app.fx.spawn(api::set_presence(host, token, body), |app, (_, fresh)| adopt(app, fresh));
}

fn go_offline(app: &mut App) {
    let Some(token) = token(app) else { return };
    let host = app.fx.host.clone();
    app.fx.spawn(api::clear_presence(host, token), |app, (_, fresh)| adopt(app, fresh));
}

fn set_enabled(app: &mut App, on: bool) {
    app.share.play.saved.enabled = on;
    persist(app);
    if on {
        poll(app);
    } else {
        go_offline(app);
    }
}

/// `saveFields`: the panel's fields become the store's, persisted, and sent if online.
fn save_fields(app: &mut App) {
    let play = &mut app.share.play;
    let clip = |edit: &TextEdit, max: usize| edit.text.trim().chars().take(max).collect::<String>();
    let next = Persisted {
        enabled: play.saved.enabled,
        status: clip(&play.status, 120),
        server_address: clip(&play.address, 80),
        mc_version: clip(&play.mc_version, 32),
        loader: play.loader.clone(),
    };
    let changed = next.status != play.saved.status
        || next.server_address != play.saved.server_address
        || next.mc_version != play.saved.mc_version
        || next.loader != play.saved.loader;
    play.saved = next;
    if !changed {
        return;
    }
    persist(app);
    if app.share.play.saved.enabled {
        heartbeat(app);
    }
}

/// Typing settles (the field's `onblur` here): save after a pause.
fn save_soon(app: &mut App) {
    app.share.play.save_gen += 1;
    let generation = app.share.play.save_gen;
    app.fx.spawn(async move { tokio::time::sleep(Duration::from_millis(800)).await }, move |app, _| {
        if app.share.play.save_gen == generation {
            save_fields(app);
        }
    });
}

fn loader_label(id: Option<&str>) -> String {
    let id = id.unwrap_or("");
    ModLoader::ALL.iter().find(|loader| loader.id() == id).map_or(id.to_string(), |loader| loader.label().to_string())
}

/// PresencePanel's `timeAgo`: now, just now, Nm ago.
fn time_ago(iso: &str) -> String {
    let Ok(at) = chrono::DateTime::parse_from_rfc3339(iso) else { return tr("community.now") };
    let secs = (chrono::Utc::now() - at.with_timezone(&chrono::Utc)).num_seconds().max(0) as f64;
    if secs < 30.0 {
        tr("community.now")
    } else if secs < 90.0 {
        tr("community.justNow")
    } else {
        tf("community.minutesAgo", &[("count", &((secs / 60.0).round() as i64).to_string())])
    }
}

fn skin_face(uuid: &str, size: u32) -> String {
    format!("https://minotar.net/helm/{uuid}/{}.png", size * 2)
}

// FriendsList

fn load_friends(app: &mut App) {
    if app.active_account().is_none() {
        app.share.play.friends = None;
        return;
    }
    let play = &mut app.share.play;
    play.friends_loading = true;
    play.friends_failed = false;
    let host = app.fx.host.clone();
    let fut = async move {
        let list = crate::commands::get_friends(host.clone()).await?;
        let prefs = crate::commands::get_friend_prefs(host).await.ok();
        Ok::<_, crate::error::AppError>((list, prefs))
    };
    app.fx.spawn(fut, |app, result| {
        let play = &mut app.share.play;
        play.friends_loading = false;
        match result {
            Ok((list, prefs)) => {
                play.friends = Some(list);
                play.friend_prefs = prefs;
            }
            Err(_) => {
                play.friends_failed = true;
                play.friends = None;
            }
        }
    });
}

fn toggle_pref(app: &mut App, which: &'static str) {
    let play = &mut app.share.play;
    let Some(prefs) = play.friend_prefs.clone() else { return };
    if play.busy {
        return;
    }
    play.busy = true;
    let (enabled, invites) = match which {
        "friendsEnabled" => (!prefs.friends_enabled, prefs.accept_invites),
        _ => (prefs.friends_enabled, !prefs.accept_invites),
    };
    let host = app.fx.host.clone();
    app.fx.spawn(crate::commands::set_friend_prefs(host, enabled, invites), |app, result| {
        app.share.play.busy = false;
        match result {
            Ok(prefs) => app.share.play.friend_prefs = Some(prefs),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn mutate(app: &mut App, name: Option<String>, profile_id: Option<String>, add: bool, success_message: Option<String>) {
    if app.share.play.busy {
        return;
    }
    app.share.play.busy = true;
    let host = app.fx.host.clone();
    app.fx.spawn(crate::commands::friend_update(host, name, profile_id, add), move |app, result| {
        app.share.play.busy = false;
        match result {
            Ok(list) => {
                app.share.play.friends = Some(list);
                if let Some(message) = success_message {
                    app.toasts.success(message).ignore();
                }
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn add_friend(app: &mut App) {
    let name: String = app.share.play.add_name.text.trim().chars().take(16).collect();
    if name.is_empty() {
        return;
    }
    app.share.play.add_name.replace_text("");
    let message = tf("community.friendRequestSent", &[("name", &name)]);
    mutate(app, Some(name), None, true, Some(message));
}

/// A 26-point square icon button (`.ic`).
fn icon_button(theme: &Theme, key: String, icon: &'static rune::Icon, help: String, busy: bool, on_press: impl Fn(&mut App) + 'static) -> El {
    let button = el().key(key).size(26.0, 26.0).center().fill(theme.bg_input).border(1.0, theme.border).role("button").label(help.clone()).help(help)
        .child(ui::colored_icon(icon, 13.0, theme.text_muted));
    if busy {
        button.opacity(0.5).disabled(true)
    } else {
        button.focusable(true).cursor(Cursor::Pointer).hover(Color::WHITE.alpha(0.06)).on_press(move |app: &mut App| on_press(app))
    }
}

pub fn friends(app: &App, _cx: &mut Cx<App>) -> El {
    let theme = app.theme();
    let play = &app.share.play;
    // The friends API is new/unstable; if the account can't use it, stay quiet.
    if app.active_account().is_none() || play.friends_failed {
        return el();
    }
    let busy = play.busy;
    let refresh = {
        let disabled = play.friends_loading || busy;
        let button = el().key("friends-refresh").size(24.0, 24.0).center().fill(theme.bg_card).border(1.0, theme.border)
            .role("button").label(tr("community.refresh")).help(tr("community.refresh"))
            .child(ui::colored_icon(&icons::REFRESH, 12.0, theme.text_muted));
        if disabled { button.opacity(0.5).disabled(true) } else { button.focusable(true).cursor(Cursor::Pointer).hover(Color::WHITE.alpha(0.06)).on_press(load_friends) }
    };
    let mut panel = col().key("friends").mb(18.0)
        .child(row().center_y().gap(8.0).mb(10.0).child(ui::h3(&theme, tr("community.friends"))).child(el().grow(1.0)).child(refresh));

    if let Some(prefs) = &play.friend_prefs {
        let pref = |key: &'static str, label: String, on: bool| {
            let mut checkbox = row().key(format!("pref-{key}")).gap(7.0).center_y().role("checkbox").checked(on).label(label.clone())
                .child(ui::checkbox(&theme, on).a11y_hidden())
                .child(ui::secondary(&theme, label, 12.5).a11y_hidden());
            if busy {
                checkbox = checkbox.opacity(0.6);
            } else {
                checkbox = checkbox.focusable(true).cursor(Cursor::Pointer).on_press(move |app: &mut App| toggle_pref(app, key));
            }
            checkbox
        };
        panel = panel.child(row().gap(16.0).mb(10.0)
            .child(pref("friendsEnabled", tr("community.friendsEnabled"), prefs.friends_enabled))
            .child(pref("acceptInvites", tr("community.acceptInvites"), prefs.accept_invites)));
        if !prefs.friends_enabled {
            panel = panel.child(text(tr("community.friendsOffWarning")).font(theme.sans(12.5, 400.0)).color(theme.warning).wrap().mb(10.0));
        }
    }

    let input = text_input(&theme, &play.add_name, tr("community.addFriendPlaceholder"), |app| &mut app.share.play.add_name)
        .spellcheck(false)
        .on_submit(|app: &mut App, _| add_friend(app));
    let can_add = !busy && !play.add_name.text.trim().is_empty();
    panel = panel.child(row().gap(8.0).mb(12.0).center_y()
        .child(field(&theme, &play.add_name, input).grow(1.0))
        .child(Btn::new(tr("common.add")).primary().sm().key("add-friend").disabled(!can_add).on_press(add_friend).build(&theme)));

    if play.friends_loading && play.friends.is_none() {
        return panel.child(ui::muted(&theme, tr("common.loading"), 12.5));
    }
    let Some(data) = &play.friends else { return panel };
    let online: Vec<&str> = play.players.iter().map(|player| player.uuid.as_str()).collect();
    let section_label = |title: String| {
        text(title.to_uppercase()).font(TextStyle { tracking: 11.0 * 0.05, ..theme.sans(11.0, 400.0) }).color(theme.text_muted).mt(8.0).mb(6.0)
    };
    let friend_row = |key: String, uuid: &str, name: &str| {
        row().key(key).center_y().gap(9.0).px(11.0).py(8.0).fill(theme.bg_card).border(2.0, theme.border)
            .child(el().size(26.0, 26.0).border(1.0, theme.border).child(ui::image(app, &skin_face(uuid, 30), 26.0, 26.0)))
            .child(text(name.to_string()).font(theme.sans(13.0, 400.0)).color(theme.text).truncate().grow(1.0).shrink().min_w(0.0))
    };
    if !data.incoming.is_empty() {
        let mut requests = col().gap(6.0).mb(6.0);
        for request in &data.incoming {
            let (accept_id, decline_id, name) = (request.profile_id.clone(), request.profile_id.clone(), request.name.clone());
            requests = requests.child(friend_row(format!("in-{}", request.profile_id), &request.profile_id, &request.name)
                .child(icon_button(&theme, "accept".into(), &icons::CHECK, tr("community.accept"), busy, move |app| {
                    let message = tf("community.friendAdded", &[("name", &name)]);
                    mutate(app, None, Some(accept_id.clone()), true, Some(message));
                }))
                .child(icon_button(&theme, "decline".into(), &icons::CLOSE, tr("community.decline"), busy, move |app| mutate(app, None, Some(decline_id.clone()), false, None))));
        }
        panel = panel.child(section_label(tr("community.requestsReceived"))).child(requests);
    }
    if !data.friends.is_empty() {
        let mut friend_list = col().gap(6.0).mb(6.0);
        for friend in &data.friends {
            let mut entry = friend_row(format!("friend-{}", friend.profile_id), &friend.profile_id, &friend.name);
            if online.contains(&friend.profile_id.replace('-', "").as_str()) {
                entry = entry.child(el().size(8.0, 8.0).radius(4.0).fill(theme.success).shadow(0.0, 5.0, theme.success.alpha(0.9)).help(tr("community.onlineInCactus")));
            }
            let (id, name) = (friend.profile_id.clone(), friend.name.clone());
            entry = entry.child(icon_button(&theme, "remove".into(), &icons::CLOSE, tr("community.removeFriend"), busy, move |app| {
                let message = tf("community.friendRemoved", &[("name", &name)]);
                mutate(app, None, Some(id.clone()), false, Some(message));
            }));
            friend_list = friend_list.child(entry);
        }
        panel = panel.child(section_label(tr("community.yourFriends"))).child(friend_list);
    }
    if !data.outgoing.is_empty() {
        let mut requests = col().gap(6.0).mb(6.0);
        for request in &data.outgoing {
            let id = request.profile_id.clone();
            requests = requests.child(friend_row(format!("out-{}", request.profile_id), &request.profile_id, &request.name)
                .child(ui::muted(&theme, tr("community.pending"), 11.0))
                .child(icon_button(&theme, "cancel".into(), &icons::CLOSE, tr("community.cancelRequest"), busy, move |app| mutate(app, None, Some(id.clone()), false, None))));
        }
        panel = panel.child(section_label(tr("community.requestsSent"))).child(requests);
    }
    if data.empty {
        panel = panel.child(ui::muted(&theme, tr("community.noFriendsYet"), 12.5));
    }
    panel
}

// PresencePanel

pub fn presence(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let share = &app.share;
    let play = &share.play;
    if !api::configured() {
        return note(&theme, &icons::GLOBE, tr("community.playTogetherOffline"));
    }
    if app.active_account().is_none() {
        if !app.accounts.list.is_empty() {
            return note(&theme, &icons::USER, tr("community.offlineModeSwitch"))
                .child(link(&theme, "switch-account", tr("community.switchAccount"), 12.5, theme.accent, |app| app.ui.accounts_open = true));
        }
        return note(&theme, &icons::USER, tr("community.addAccountToSeeOnline"));
    }
    if share.logging_in && share.session.is_none() {
        return row().gap(6.0).center_y().child(ui::spinner(&theme, 14.0)).child(ui::muted(&theme, tr("community.connecting"), 12.5));
    }
    if share.session.is_none() {
        let error_suffix = share.auth_error.as_ref().map(|error| format!(": {error}")).unwrap_or_default();
        return note(&theme, &icons::USER, format!("{}{error_suffix}.", tr("community.couldntConnect")))
            .child(link(&theme, "retry", tr("common.retry"), 12.5, theme.accent, login));
    }

    let my_uuid = share.session.as_ref().map(|session| session.uuid.as_str()).unwrap_or("");
    let me = play.players.iter().find(|player| player.uuid == my_uuid);
    let passes = |player: &&PresencePlayer| {
        (play.filter_loader.is_empty() || player.loader.as_deref().unwrap_or("") == play.filter_loader)
            && (play.filter_version.is_empty() || player.mc_version.as_deref().unwrap_or("") == play.filter_version)
    };
    let others: Vec<&PresencePlayer> = play.players.iter().filter(|player| player.uuid != my_uuid).filter(passes).collect();

    let mut panel = col().key("presence").child(me_card(app, &theme, layer));

    let mut versions: Vec<String> = play.players.iter().filter_map(|player| player.mc_version.clone()).filter(|version| !version.is_empty()).collect();
    versions.sort();
    versions.dedup();
    let loader_opts: Vec<(String, String)> = std::iter::once((String::new(), tr("community.anyLoader")))
        .chain(ModLoader::ALL.iter().map(|loader| (loader.id().to_string(), loader.label().to_string())))
        .collect();
    let version_opts: Vec<(String, String)> =
        std::iter::once((String::new(), tr("community.anyVersion"))).chain(versions.into_iter().map(|version| (version.clone(), version))).collect();
    panel = panel.child(
        row().key("list-head").center_y().gap(10.0).mb(12.0)
            .child(ui::h3(&theme, tr("community.onlineNow")))
            .child(ui::muted(&theme, (others.len() + usize::from(me.is_some())).to_string(), 12.0))
            .child(el().grow(1.0))
            .child(row().center_y().gap(6.0)
                .child(select(app, layer, "filter-loader", Some(150.0), &loader_opts, &play.filter_loader, "", |app, loader| app.share.play.filter_loader = loader))
                .child(select(app, layer, "filter-version", Some(150.0), &version_opts, &play.filter_version, "", |app, version| app.share.play.filter_version = version))
                .child(Btn::new(tr("community.refresh")).ghost().sm().key("presence-refresh").disabled(play.loading).on_press(poll).build(&theme))),
    );
    if let Some(error) = &play.error {
        panel = panel.child(note(&theme, &icons::GLOBE, error.clone()).mb(10.0));
    }
    if others.is_empty() && me.is_none() {
        return panel.child(
            col().key("empty").center().gap(10.0).px(20.0).py(40.0)
                .child(el().opacity(0.6).child(ui::colored_icon(&icons::USERS, 30.0, theme.text_muted)))
                .child(ui::muted(&theme, tr("community.noOneOnline"), 14.0).wrap().align(rune::Align::Center)),
        );
    }
    let mut list = col().key("players").gap(8.0);
    if let Some(me) = me {
        list = list.child(player_row(&theme, app, me, true));
    }
    for player in others {
        list = list.child(player_row(&theme, app, player, false));
    }
    panel.child(list)
}

fn me_card(app: &App, theme: &Theme, layer: &mut Vec<El>) -> El {
    let play = &app.share.play;
    let on = play.saved.enabled;
    let track = el().size(44.0, 24.0).padding(4.0).center_y()
        .fill(if on { theme.accent_soft } else { theme.bg_input })
        .border(2.0, if on { theme.accent } else { theme.border })
        .child(el().size(16.0, 16.0).ml(if on { 20.0 } else { 0.0 }).fill(if on { theme.accent } else { theme.text_muted }).motion(rune::Spring::new(0.14, 0.0)));
    let switch = row().key("appear-online").center_y().gap(10.0).role("switch").checked(on).label(tr("community.appearOnline"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| set_enabled(app, !on))
        .child(track)
        .child(col()
            .child(text(tr("community.appearOnline")).font(theme.sans(13.0, 600.0)).color(theme.text))
            .child(ui::muted(theme, tr("community.appearOnlineHint"), 12.0)));
    let mut panel = card(theme.bg_card, theme.border, 2.0, 16.0, 14.0).key("me").mb(18.0).child(switch);
    if !on {
        return panel;
    }
    let field_label = |title: String| ui::secondary(theme, title, 12.5);
    let entry = |edit: &TextEdit, placeholder: String, access: fn(&mut App) -> &mut TextEdit| {
        let input = text_input(theme, edit, placeholder, access).on_change(|app: &mut App, _| save_soon(app)).on_submit(|app: &mut App, _| save_fields(app));
        field(theme, edit, input)
    };
    let loader_opts: Vec<(String, String)> = std::iter::once((String::new(), tr("community.any")))
        .chain(ModLoader::ALL.iter().map(|loader| (loader.id().to_string(), loader.label().to_string())))
        .collect();
    panel = panel
        .child(row().key("fields").flex_wrap().gap(12.0).mt(14.0)
            .child(col().gap(5.0).grow(1.0).min_w(200.0)
                .child(field_label(tr("community.status")))
                .child(entry(&play.status, tr("community.statusPlaceholder"), |app| &mut app.share.play.status)))
            .child(col().gap(5.0).w(130.0)
                .child(field_label(tr("community.mcVersion")))
                .child(entry(&play.mc_version, "1.20.1".into(), |app| &mut app.share.play.mc_version)))
            .child(col().gap(5.0).w(130.0)
                .child(field_label(tr("community.loader")))
                .child(select(app, layer, "presence-loader", None, &loader_opts, &play.loader, "", |app, loader| {
                    app.share.play.loader = loader;
                    save_fields(app);
                })))
            .child(col().gap(5.0).grow(1.0).min_w(200.0)
                .child(field_label(tr("community.serverAddressLabel")))
                .child(entry(&play.address, tr("community.serverAddressExample"), |app| &mut app.share.play.address))))
        .child(ui::muted(theme, tr("community.connectHint"), 12.0).wrap().mt(10.0));
    panel
}

fn player_row(theme: &Theme, app: &App, player: &PresencePlayer, is_me: bool) -> El {
    let mut name = row().gap(4.0).center_y().child(text(player.name.clone()).font(theme.sans(13.5, 600.0)).color(theme.text));
    if is_me {
        name = name.child(el().px(4.0).border(1.0, theme.accent).child(text(tr("community.you").to_uppercase()).font(theme.pixel(8.0)).color(theme.accent)));
    }
    let status = if player.status.is_empty() { tr("community.online") } else { player.status.clone() };
    let mut entry = row().key(format!("player-{}", player.uuid)).center_y().gap(12.0).px(14.0).py(12.0).fill(theme.bg_card)
        .border(2.0, if is_me { theme.accent } else { theme.border })
        .child(el().size(32.0, 32.0).border(2.0, Color::BLACK.alpha(0.3)).child(ui::image(app, &skin_face(&player.uuid, 32), 32.0, 32.0)))
        .child(col().grow(1.0).shrink().min_w(0.0)
            .child(name)
            .child(ui::muted(theme, status, 12.0).truncate().max_w(320.0)));
    if player.mc_version.as_deref().is_some_and(|version| !version.is_empty()) || player.loader.as_deref().is_some_and(|loader| !loader.is_empty()) {
        let version_suffix = player.mc_version.as_deref().filter(|version| !version.is_empty()).map(|version| format!(" {version}")).unwrap_or_default();
        entry = entry.child(el().px(6.0).py(2.0).border(1.0, theme.border)
            .child(text(format!("{}{version_suffix}", loader_label(player.loader.as_deref())).to_uppercase()).font(TextStyle { tracking: 9.0 * 0.04, ..theme.pixel(9.0) }).color(theme.text_muted)));
    }
    entry = entry.child(ui::muted(theme, time_ago(&player.updated_at), 11.0));
    if let Some(addr) = player.server_address.clone().filter(|addr| !addr.is_empty()) {
        entry = entry.child(
            Btn::new(addr.clone())
                .ghost()
                .sm()
                .icon(&icons::COPY)
                .key("copy-address")
                .help(tr("community.copyServerAddress"))
                .on_press_tx(move |app, tx| {
                    tx.copy(addr.clone());
                    app.toasts.success(tf("community.copied", &[("address", &addr)])).ignore();
                })
                .build(theme),
        );
    }
    entry
}
