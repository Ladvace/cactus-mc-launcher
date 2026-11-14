//! Community: `src/routes/share/+page.svelte`, `share/creator`, `share/[handle]`, BoardView,
//! FriendsList, PresencePanel, ServerWeather, Sparkline; the `boardAuth` and `followedBoards`
//! stores live here, `presence` in `play`.

mod api;
mod board;
mod creator;
mod play;

use std::time::Duration;

use rune::{Color, Cursor, Cx, El, Shadow, TextEdit, col, el, row, text};
use rune_kit::{Menu, MenuItem, Placement, TextInput};
use serde_json::json;

use crate::app::{App, Ignore, Route};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const SESSION_KEY: &str = "cactus:boardSession";
const FOLLOWED_KEY: &str = "cactus:followedBoards";
const IMPORTED_KEY: &str = "cactus:importedFrom";

#[derive(Clone, Debug, Default, PartialEq)]
enum Tab {
    #[default]
    Discover,
    Play,
    Board(String),
}

/// What the share card shows after an import by code.
struct Imported {
    id: String,
    name: String,
    installed: usize,
    skipped: usize,
}

#[derive(Default)]
pub struct State {
    prefs_loaded: bool,
    // boardAuth.svelte.ts
    session: Option<api::Session>,
    logging_in: bool,
    auth_error: Option<String>,
    // followedBoards.svelte.ts
    followed: Vec<String>,
    /// The kit menu standing in for an open `Select`, by key.
    open_select: Option<String>,

    active: Tab,
    has_board: bool,
    query: TextEdit,
    query_gen: u64,
    results: Vec<api::BoardCard>,
    searching: bool,
    code: TextEdit,
    code_busy: bool,
    importing: bool,
    result: Option<Imported>,

    board: board::State,
    play: play::State,
    creator: creator::State,
}

pub fn on_enter(app: &mut App) {
    load_prefs(app);
    app.share.open_select = None;
    match app.route.clone() {
        Route::Community => {
            let share = &mut app.share;
            share.active = Tab::Discover;
            share.query.replace_text("");
            share.results.clear();
            share.searching = false;
            share.code.replace_text("");
            share.result = None;
            check_has_board(app);
        }
        Route::ShareCreator => creator::on_enter(app),
        Route::ShareBoard(handle) => board::open(app, &handle),
        _ => {}
    }
}

pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let page = match &app.route {
        Route::ShareCreator => creator::view(app, cx, layer),
        Route::ShareBoard(handle) => board_page(app, cx, handle),
        _ => community(app, cx, layer),
    };
    col().key("share-scroll").w_full().h_full().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(page)
}

fn load_prefs(app: &mut App) {
    if app.share.prefs_loaded {
        return;
    }
    app.share.prefs_loaded = true;
    app.share.session = app.prefs.read(SESSION_KEY, None);
    app.share.followed = app.prefs.read(FOLLOWED_KEY, Vec::new());
    play::load_prefs(app);
}

// boardAuth

fn token(app: &App) -> Option<String> {
    app.share.session.as_ref().map(|session| session.token.clone())
}

/// The session is the active account's (what the creator page and presence wait for).
fn session_matches(app: &App) -> bool {
    match (&app.share.session, app.active_account()) {
        (Some(session), Some(account)) => session.uuid == account.uuid,
        _ => false,
    }
}

/// Adopt a session the API layer re-minted on expiry (`onBoardSessionRefresh`).
fn adopt(app: &mut App, fresh: Option<api::Session>) {
    if let Some(session) = fresh {
        app.prefs.write(SESSION_KEY, &session);
        app.share.session = Some(session);
    }
}

fn login(app: &mut App) {
    app.share.logging_in = true;
    app.share.auth_error = None;
    let host = app.fx.host.clone();
    app.fx.spawn(api::login(host), |app, result| {
        app.share.logging_in = false;
        match result {
            Ok(session) => {
                app.prefs.write(SESSION_KEY, &session);
                app.share.session = Some(session);
                signed_in(app);
            }
            Err(e) => app.share.auth_error = Some(e),
        }
    });
}

/// The `$effect` in PresencePanel and the creator page: sign in to the boards service as the
/// active account, once; after a failure the user retries by hand.
fn ensure_login(app: &mut App) {
    if !api::configured() || app.active_account().is_none() {
        return;
    }
    if !app.share.logging_in && app.share.auth_error.is_none() && !session_matches(app) {
        login(app);
    }
}

/// What reacts to `boardAuth.token` appearing.
fn signed_in(app: &mut App) {
    match app.route {
        Route::ShareCreator => creator::load_boards(app),
        Route::Community => {
            check_has_board(app);
            if app.share.active == Tab::Play {
                play::poll(app);
            }
        }
        _ => {}
    }
}

fn check_has_board(app: &mut App) {
    let Some(token) = token(app).filter(|_| api::configured()) else { return };
    let host = app.fx.host.clone();
    app.fx.spawn(api::my_boards(host, token), |app, (result, fresh)| {
        adopt(app, fresh);
        if let Ok(boards) = result {
            app.share.has_board = !boards.is_empty();
        }
    });
}

// followedBoards

fn is_following(app: &App, handle: &str) -> bool {
    app.share.followed.iter().any(|followed| followed == handle)
}

fn set_following(app: &mut App, handle: &str, on: bool) {
    let list = &mut app.share.followed;
    if on {
        if list.iter().any(|followed| followed == handle) {
            return;
        }
        list.push(handle.to_string());
    } else {
        list.retain(|followed| followed != handle);
    }
    let list = list.clone();
    app.prefs.write(FOLLOWED_KEY, list);
}

/// `recordImport` in importedFrom.ts.
fn record_import(app: &mut App, instance_id: &str, handle: Option<&str>, snapshot_id: &str) {
    let mut map: serde_json::Map<String, serde_json::Value> = app.prefs.read(IMPORTED_KEY, serde_json::Map::new());
    let now = chrono::Utc::now().timestamp_millis();
    map.insert(instance_id.to_string(), json!({ "handle": handle, "snapshotId": snapshot_id, "importedAt": now }));
    app.prefs.write(IMPORTED_KEY, map);
}

/// Refresh instances, then run `then` (`await instancesStore.refresh()` before what follows).
fn refresh_then(app: &mut App, then: impl FnOnce(&mut App) + Send + 'static) {
    app.fx.blocking(|host| crate::commands::list_instances(host.state()), move |app, list| {
        app.instances = list;
        app.instances_loaded = true;
        then(app);
    });
}

/// Home's "Import from code": resolve a share code to a snapshot, import it, open the new
/// instance (`boardApi.resolveCode` then `boardApi.importSnapshot`). `done` gets the outcome.
pub fn import_from_code(app: &mut App, code: String, done: impl FnOnce(&mut App, Result<(), String>) + Send + 'static) {
    let code = code.trim().to_string();
    if code.is_empty() {
        return done(app, Err("Enter a share code".into()));
    }
    if !api::configured() {
        return done(app, Err(tr("community.boardsServiceNotConfigured")));
    }
    let host = app.fx.host.clone();
    let fut = async move {
        let id = api::resolve_code(code).await?;
        api::import_snapshot(host, id).await
    };
    app.fx.spawn(fut, move |app, result| match result {
        Ok(imported) => {
            let id = imported.instance.id.clone();
            refresh_then(app, move |app| {
                done(app, Ok(()));
                app.go(Route::Instance(id));
            });
        }
        Err(e) => done(app, Err(e)),
    });
}

// Shared pieces.

/// `.input` round a kit text field, so the field gets `on_change` / `on_submit`.
fn field(theme: &Theme, edit: &TextEdit, input: TextInput) -> El {
    ui::input_frame(theme, edit.focused).child(input.build(&theme.kit()))
}

fn text_input(
    theme: &Theme,
    edit: &TextEdit,
    placeholder: impl Into<String>,
    access: impl for<'a> Fn(&'a mut App) -> &'a mut TextEdit + 'static,
) -> TextInput {
    TextInput::new(edit).plain().width(None).text_style(theme.sans(13.0, 400.0)).placeholder(placeholder).on_edit(access)
}

/// `Select.svelte`'s trigger with a kit menu of its options (value, label). `current` that is
/// no option's value shows `placeholder`.
#[allow(clippy::too_many_arguments)]
fn select(
    app: &App,
    layer: &mut Vec<El>,
    key: &str,
    width: Option<f32>,
    options: &[(String, String)],
    current: &str,
    placeholder: &str,
    on_choose: impl Fn(&mut App, String) + Clone + 'static,
) -> El {
    let theme = app.theme();
    let open = app.share.open_select.as_deref() == Some(key);
    let label = options.iter().find(|(value, _)| value == current).map_or(placeholder.to_string(), |(_, name)| name.clone());
    let menu_key = key.to_string();
    let mut trigger = row()
        .key(format!("{key}-field"))
        .center_y()
        .gap(8.0)
        .px(14.0)
        .py(11.0)
        .fill(theme.bg_input)
        .border(2.0, if open { theme.accent } else { theme.border })
        .box_shadow(Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.28), inset: true })
        .role("combobox")
        .label(label.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| {
            app.share.open_select = if app.share.open_select.as_deref() == Some(menu_key.as_str()) { None } else { Some(menu_key.clone()) };
        })
        .child(text(label).font(theme.sans(13.0, 400.0)).color(theme.text).truncate().grow(1.0).shrink().min_w(0.0))
        .child(ui::colored_icon(&icons::CHEVRON_DOWN, 12.0, theme.text_muted));
    trigger = match width {
        Some(w) => trigger.w(w),
        None => trigger.w_full(),
    };
    let mut menu = Menu::new(key.to_string())
        .trigger(trigger)
        .open(open)
        .placement(Placement::BottomStart)
        .min_width(width.unwrap_or(160.0))
        .on_close(|app: &mut App| app.share.open_select = None);
    for (value, label) in options {
        let (choose, chosen) = (on_choose.clone(), value.clone());
        menu = menu.item(MenuItem::new(label.clone()).checked(value == current).on_select(move |app: &mut App| {
            app.share.open_select = None;
            choose(app, chosen.clone());
        }));
    }
    menu.build(&theme.kit(), layer)
}

/// `.offline`: a muted line with an icon.
fn note(theme: &Theme, icon: &'static rune::Icon, line: impl Into<String>) -> El {
    row().gap(6.0).center_y().child(ui::colored_icon(icon, 13.0, theme.text_muted)).child(ui::muted(theme, line, 12.5).wrap().shrink())
}

/// A text-only button (`.link`, `.back`): `color`, hover in `hover`.
fn link(theme: &Theme, key: &str, label: impl Into<String>, size: f32, color: Color, on_press: impl Fn(&mut App) + 'static) -> El {
    let label = label.into();
    el().key(key.to_string())
        .role("button")
        .label(label.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover(Color::WHITE.alpha(0.05))
        .on_press(move |app: &mut App| on_press(app))
        .child(text(label).font(theme.sans(size, 400.0)).color(color))
}

/// A bordered card: `border` takes no layout space here, so the padding adds the CSS border.
fn card(fill: Color, border: Color, border_width: f32, px: f32, py: f32) -> El {
    col().fill(fill).border(border_width, border).px(px + border_width).py(py + border_width)
}

/// `.skeleton` card of the creator page and BoardView's loading.
fn board_skeleton(theme: &Theme) -> El {
    card(theme.bg_card, theme.border, 2.0, 20.0, 18.0)
        .mb(16.0)
        .child(
            row().gap(14.0).center_y()
                .child(ui::skeleton(theme, Some(44.0), 44.0))
                .child(col().gap(8.0).grow(1.0)
                    .child(el().w_frac(0.4).child(ui::skeleton(theme, None, 15.0)))
                    .child(el().w_frac(0.7).child(ui::skeleton(theme, None, 11.0)))),
        )
}

/// `/share/[handle]`.
fn board_page(app: &App, cx: &mut Cx<App>, handle: &str) -> El {
    let theme = app.theme();
    col().key("board-page").w_full().max_w(900.0).self_center().px(32.0).py(24.0)
        .child(el().mb(12.0).self_start().child(link(&theme, "back", format!("← {}", tr("nav.community")), 13.0, theme.text_secondary, |app| app.go(Route::Community))))
        .child(board::view(app, cx, handle))
}

// `/share`

fn select_tab(app: &mut App, tab: Tab) {
    let was_play = app.share.active == Tab::Play;
    app.share.active = tab.clone();
    match tab {
        Tab::Play if !was_play => play::open(app),
        Tab::Board(handle) => board::open(app, &handle),
        _ => {}
    }
    if was_play && app.share.active != Tab::Play {
        play::close(app);
    }
}

fn community(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let share = &app.share;
    let hero = row().key("hero").justify_between().items_start().gap(16.0).mb(18.0)
        .child(col().shrink()
            .child(ui::heading(&theme, tr("nav.community"), 26.0))
            .child(ui::secondary(&theme, tr("community.subtitle"), 14.0).mt(6.0).wrap()))
        .child(
            Btn::new(if share.has_board { tr("community.editBoard") } else { tr("community.createBoard") })
                .ghost()
                .icon(if share.has_board { &icons::EDIT } else { &icons::PLUS })
                .key("creator")
                .on_press(|app| app.go(Route::ShareCreator))
                .build(&theme),
        );

    let content = match &share.active {
        Tab::Play => col().key("play").child(play::friends(app, cx)).child(play::presence(app, cx, layer)),
        Tab::Board(handle) => col().key(format!("board-{handle}")).child(board::view(app, cx, handle)),
        Tab::Discover => discover(app),
    };

    col().key("community").w_full().max_w(940.0).self_center().px(32.0).py(28.0)
        .child(hero)
        .child(tabbar(app))
        .child(content)
}

fn tabbar(app: &App) -> El {
    let theme = app.theme();
    let share = &app.share;
    let mut handles = share.followed.clone();
    if let Tab::Board(handle) = &share.active {
        if !handles.contains(handle) {
            handles.push(handle.clone());
        }
    }
    let tab = |key: String, label: String, tab: Tab| {
        let on = share.active == tab;
        el().key(key)
            .px(14.0)
            .pt(8.0)
            .pb(8.0 + 3.0)
            .border_sides([0.0, 0.0, 3.0, 0.0], if on { theme.accent } else { Color::TRANSPARENT })
            .role("tab")
            .checked(on)
            .label(label.clone())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .hover(Color::WHITE.alpha(0.04))
            .on_press(move |app: &mut App| select_tab(app, tab.clone()))
            .child(text(label).font(theme.pixel(13.0)).color(if on { theme.accent } else { theme.text_secondary }))
    };
    let mut bar = row().key("tabbar").gap(4.0).center_y().mb(18.0).border_sides([0.0, 0.0, 2.0, 0.0], theme.border_subtle)
        .child(tab("tab-discover".into(), tr("community.discover"), Tab::Discover));
    for handle in handles {
        bar = bar.child(tab(format!("tab-@{handle}"), format!("@{handle}"), Tab::Board(handle.clone())));
    }
    let on = share.active == Tab::Play;
    let mut play = row().key("tab-play")
        .gap(6.0)
        .center_y()
        .px(14.0)
        .py(8.0)
        .fill(theme.accent)
        .role("tab")
        .checked(on)
        .label(tr("community.playTogether"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover(theme.accent_hover)
        .on_press(|app: &mut App| select_tab(app, Tab::Play))
        .child(ui::colored_icon(&icons::USERS, 14.0, theme.accent_contrast))
        .child(text(tr("community.playTogether")).font(theme.pixel(13.0)).color(theme.accent_contrast));
    if on {
        play = play.box_shadow(Shadow { x: 0.0, y: 0.0, blur: 0.0, spread: 2.0, color: theme.accent_soft, inset: false });
    }
    bar.child(el().grow(1.0)).child(play)
}

fn discover(app: &App) -> El {
    let theme = app.theme();
    let share = &app.share;
    let mut body = col().key("discover");
    if api::configured() {
        let input = text_input(&theme, &share.query, tr("community.searchPlaceholder"), |app| &mut app.share.query).on_change(|app: &mut App, _| on_query(app));
        let mut search = row().key("search").center_y().gap(8.0).px(14.0).min_h(42.0).mb(12.0).fill(theme.bg_input).border(2.0, theme.border)
            .child(ui::colored_icon(&icons::SEARCH, 16.0, theme.text_muted))
            .child(el().grow(1.0).child(input.build(&theme.kit())));
        if share.searching {
            search = search.child(ui::spinner(&theme, 13.0));
        }
        body = body.child(search);
        if !share.results.is_empty() {
            let mut list = col().key("results").gap(6.0).mb(18.0);
            for board in &share.results {
                let handle = board.handle.clone();
                list = list.child(
                    row().key(format!("result-{}", board.handle)).center_y().gap(12.0).px(14.0).py(12.0).fill(theme.bg_card).border(2.0, theme.border)
                        .role("button")
                        .label(board.display_name.clone())
                        .focusable(true)
                        .cursor(Cursor::Pointer)
                        .hover(theme.bg_hover)
                        .on_press(move |app: &mut App| select_tab(app, Tab::Board(handle.clone())))
                        .child(kind_badge(&theme, &board.kind, 9.0))
                        .child(col()
                            .child(text(board.display_name.clone()).font(theme.sans(13.5, 600.0)).color(theme.text))
                            .child(ui::muted(&theme, format!("@{} · {}", board.handle, board.owner_name), 12.0))),
                );
            }
            body = body.child(list);
        }
    } else {
        body = body.child(note(&theme, &icons::GLOBE, tr("community.discoveryOffline")).mb(14.0));
    }
    body.child(share_card(app))
        .child(tip(&theme, &icons::DOWNLOAD, tr("community.importFileTip"), tr("common.import")))
        .child(tip(&theme, &icons::UPLOAD, tr("community.exportTip"), tr("instanceMenu.exportToFile")))
}

/// `.kind`: the board kind, small caps in a hairline box.
fn kind_badge(theme: &Theme, kind: &str, size: f32) -> El {
    el().px(6.0).py(3.0).border(1.0, theme.border)
        .child(text(kind.to_uppercase()).font(theme.pixel(size)).color(theme.text_muted))
}

fn tip(theme: &Theme, icon: &'static rune::Icon, line: String, strong: String) -> El {
    row().gap(6.0).center_y().mt(16.0).mx(2.0)
        .child(ui::colored_icon(icon, 13.0, theme.text_muted))
        .child(ui::muted(theme, line, 12.5))
        .child(text(strong).font(theme.sans(12.5, 600.0)).color(theme.text_muted))
}

fn share_card(app: &App) -> El {
    let theme = app.theme();
    let share = &app.share;
    let mut panel = card(theme.bg_card, theme.border, 2.0, 20.0, 18.0).key("share-card").child(ui::h3(&theme, tr("community.shareInstance")).mb(12.0));
    if let Some(imported) = &share.result {
        let mut line = tf("community.importedInstance", &[("name", &imported.name), ("count", &imported.installed.to_string())]);
        if imported.skipped > 0 {
            line += &tf("community.skippedSuffix", &[("count", &imported.skipped.to_string())]);
        }
        line.push('.');
        let id = imported.id.clone();
        panel = panel
            .child(row().gap(6.0).center_y()
                .child(ui::colored_icon(&icons::CHECK, 15.0, theme.accent))
                .child(text(line).font(theme.sans(13.0, 400.0)).color(theme.accent).wrap().shrink()))
            .child(row().gap(8.0).mt(10.0)
                .child(Btn::new(tr("community.open")).primary().sm().key("open-imported").on_press(move |app| app.go(Route::Instance(id.clone()))).build(&theme))
                .child(Btn::new(tr("community.importAnother")).ghost().sm().key("import-another").on_press(|app| app.share.result = None).build(&theme)));
    } else if share.importing {
        panel = panel.child(ui::muted(&theme, tr("common.importing"), 14.0));
    } else if api::configured() {
        let input = text_input(&theme, &share.code, tr("community.shareCodePlaceholder"), |app| &mut app.share.code).on_submit(|app: &mut App, _| import_by_code(app));
        let can_submit = !share.code_busy && !share.code.text.trim().is_empty();
        panel = panel.child(
            row().key("code").self_start().center_y().gap(6.0).pl(14.0).pr(8.0).py(6.0).fill(theme.bg_input).border(2.0, theme.border)
                .child(el().w(190.0).child(input.build(&theme.kit())))
                .child(Btn::new(tr("community.go")).primary().sm().key("code-go").disabled(!can_submit).on_press(import_by_code).build(&theme)),
        );
    }
    panel
}

/// The search field changed: debounce 300 ms, then search (`$effect` on `debounced`).
fn on_query(app: &mut App) {
    app.share.query_gen += 1;
    let generation = app.share.query_gen;
    app.fx.spawn(async move { tokio::time::sleep(Duration::from_millis(300)).await }, move |app, _| {
        if app.share.query_gen != generation {
            return;
        }
        let query = app.share.query.text.trim().to_string();
        if query.is_empty() {
            app.share.results.clear();
            return;
        }
        app.share.searching = true;
        app.fx.spawn(api::search(query), move |app, results| {
            if app.share.query_gen != generation {
                return;
            }
            app.share.results = results.unwrap_or_default();
            app.share.searching = false;
        });
    });
}

fn import_by_code(app: &mut App) {
    let code = app.share.code.text.trim().to_string();
    if code.is_empty() || app.share.code_busy {
        return;
    }
    app.share.code_busy = true;
    app.share.result = None;
    app.fx.spawn(api::resolve_code(code), |app, result| match result {
        Err(e) => {
            app.share.code_busy = false;
            app.toasts.error(e).ignore();
        }
        Ok(snapshot_id) => {
            app.share.importing = true;
            let host = app.fx.host.clone();
            app.fx.spawn(api::import_snapshot(host, snapshot_id.clone()), move |app, result| match result {
                Err(e) => {
                    app.share.code_busy = false;
                    app.share.importing = false;
                    app.toasts.error(e).ignore();
                }
                Ok(outcome) => {
                    record_import(app, &outcome.instance.id, None, &snapshot_id);
                    let imported = Imported { id: outcome.instance.id.clone(), name: outcome.instance.name.clone(), installed: outcome.installed, skipped: outcome.skipped.len() };
                    refresh_then(app, move |app| {
                        app.share.result = Some(imported);
                        app.share.code_busy = false;
                        app.share.importing = false;
                    });
                }
            });
        }
    });
}
