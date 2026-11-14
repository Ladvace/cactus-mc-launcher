//! AccountModal.svelte: offline and Microsoft accounts, switching, removing, the device-code
//! sign-in, and the active account's SkinPanel (with SkinEditor and the CPU SkinViewer).

pub mod editor;
pub mod panel;
pub mod skins;
pub mod viewer;

use std::cell::RefCell;

use rune::text::Span;
use rune::{Cursor, Cx, El, KeyEvent, Tx, col, el, rich, row, text};

use crate::app::{App, Route};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

pub use skins::skin_face;

#[derive(Default)]
pub struct State {
    pub skins: skins::Cache,
    pub panel: panel::State,
    pub editor: editor::State,
    /// The control under the pointer, for the `:hover { border-color: accent }` looks.
    pub hovered: Option<String>,
    /// The account uuid the skin panel is mounted for, as the view last saw it.
    mounted: RefCell<Option<String>>,
}

pub(crate) fn set_hovered(app: &mut App, key: &str, over: bool) {
    let hovered = &mut app.account_modal.hovered;
    if over {
        *hovered = Some(key.to_string());
    } else if hovered.as_deref() == Some(key) {
        *hovered = None;
    }
}

/// Once, after the app state is built.
pub fn on_start(app: &mut App) {
    app.account_modal.skins = skins::Cache::load(app);
}

/// A backend event, after the core stores took it.
pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

/// Window keys nothing focused took; true consumes the key.
pub fn on_key(_app: &mut App, _key: &KeyEvent, _tx: &mut Tx) -> bool {
    false
}

fn close(app: &mut App) {
    app.ui.accounts_open = false;
}

/// Mount or unmount the skin panel to follow the modal and the active account (Svelte
/// remounts it on open and `{#key active.id}`); the work runs as a job, outside the view.
fn sync_panel(app: &App) {
    let active = if app.ui.accounts_open { app.active_account().cloned() } else { None };
    let want = active.as_ref().map(|account| account.uuid.clone());
    let mut mounted = app.account_modal.mounted.borrow_mut();
    if *mounted == want {
        return;
    }
    *mounted = want;
    app.fx.post(move |app| match active {
        Some(acc) => panel::mount(app, &acc),
        None => {
            panel::unmount(app);
            app.account_modal.editor.open = false;
        }
    });
}

/// Push whatever of these overlays is open onto `layer`.
pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    sync_panel(app);
    if !app.ui.accounts_open {
        return;
    }
    let theme = app.theme();
    let mut popups = Vec::new();
    let mut body = col().key("accounts-body");
    if let Some(acc) = app.active_account() {
        if app.account_modal.panel.uuid.as_deref() == Some(acc.uuid.as_str()) {
            body = body.child(panel::view(app, cx, acc, &mut popups));
        }
    }
    body = body.child(list(app, &theme)).child(add_area(app, &theme)).child(achievements(&theme));
    layer.push(ui::modal(&theme, "accounts", tr("account.title"), 460.0, body, None, close));
    layer.extend(popups);
    editor::view(app, cx, layer);
}

/// `.row`: a sunken box whose border lights up when active or hovered.
fn row_box(app: &App, theme: &Theme, key: &str, active: bool) -> El {
    let hovered = app.account_modal.hovered.as_deref() == Some(key);
    let hover_key = key.to_string();
    row()
        .key(key.to_string())
        .center_y()
        .gap(12.0)
        .w_full()
        .px(12.0)
        .py(10.0)
        .fill(theme.bg_input)
        .border(2.0, if active || hovered { theme.accent } else { theme.border })
        .on_hover_change(move |app: &mut App, over, _| set_hovered(app, &hover_key, over))
}

fn info(theme: &Theme, name: String, kind: String) -> El {
    col()
        .min_w(0.0)
        .shrink()
        .gap(1.0)
        .child(text(name).font(theme.sans(13.5, 600.0)).color(theme.text).truncate())
        .child(text(kind).font(theme.sans(11.5, 400.0)).color(theme.text_muted))
}

fn badge(theme: &Theme) -> El {
    el().px(9.0).py(3.0).border(2.0, theme.accent).child(text(tr("account.active")).font(theme.pixel(11.0)).color(theme.accent))
}

/// `.reauth` / `.remove`: a bare icon button tinted on hover.
fn icon_btn(app: &App, theme: &Theme, key: &str, icon: &'static rune::Icon, size: f32, tint: rune::Color, help: String, disabled: bool) -> El {
    let hovered = app.account_modal.hovered.as_deref() == Some(key) && !disabled;
    let hover_key = key.to_string();
    let mut button = el()
        .key(key.to_string())
        .padding(6.0)
        .role("button")
        .label(help.clone())
        .help(help)
        .focusable(!disabled)
        .on_hover_change(move |app: &mut App, over, _| set_hovered(app, &hover_key, over))
        .child(ui::colored_icon(icon, size, if hovered { tint } else { theme.text_muted }));
    if hovered {
        button = button.fill(tint.alpha(0.13));
    }
    if disabled { button.opacity(0.4).disabled(true) } else { button.cursor(Cursor::Pointer) }
}

fn list(app: &App, theme: &Theme) -> El {
    let accounts = &app.accounts;
    let offline_name = if app.settings.offline_username.is_empty() { tr("account.defaultPlayer") } else { app.settings.offline_username.clone() };
    let offline_active = accounts.active_id.is_none();
    let mut offline = row_box(app, theme, "account-offline", offline_active)
        .role("button")
        .label(offline_name.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.set_active_account(None))
        .child(avatar(theme).center().child(ui::colored_icon(&icons::USER, 18.0, theme.text_secondary)))
        .child(info(theme, offline_name, tr("account.offline")).grow(1.0));
    if offline_active {
        offline = offline.child(badge(theme));
    }
    let mut list = col().key("account-list").gap(6.0).child(offline);

    if !accounts.loaded {
        list = list.child(
            row_box(app, theme, "account-skel", false)
                .child(ui::skeleton(theme, Some(36.0), 36.0))
                .child(col().gap(4.0).child(ui::skeleton(theme, Some(100.0), 13.0)).child(ui::skeleton(theme, Some(60.0), 11.0))),
        );
        return list;
    }
    for acc in &accounts.list {
        let active = accounts.active_id.as_deref() == Some(acc.id.as_str());
        let id = acc.id.clone();
        let face = skin_face(app, &acc.uuid, 36);
        let pic = match app.images.get(&app.fx, &face) {
            Some(img) => avatar(theme).pixels(img.pixels()),
            None => avatar(theme),
        };
        let main = row()
            .key("main")
            .center_y()
            .gap(12.0)
            .grow(1.0)
            .min_w(0.0)
            .role("button")
            .label(acc.username.clone())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| app.set_active_account(Some(id.clone())))
            .child(pic.label(acc.username.clone()))
            .child(info(theme, acc.username.clone(), "Microsoft".into()));
        let mut entry = row_box(app, theme, &format!("account-{}", acc.id), active).child(main);
        if active {
            entry = entry.child(badge(theme));
        }
        let remove_id = acc.id.clone();
        entry = entry
            .child(
                icon_btn(app, theme, &format!("reauth-{}", acc.id), &icons::REFRESH, 14.0, theme.accent, tr("account.signInAgain"), accounts.logging_in)
                    .on_press(|app: &mut App| app.login_microsoft()),
            )
            .child(
                icon_btn(app, theme, &format!("remove-{}", acc.id), &icons::TRASH, 15.0, theme.danger, tr("account.removeAccount"), false)
                    .on_press(move |app: &mut App| app.remove_account(&remove_id)),
            );
        list = list.child(entry);
    }
    list
}

/// `.avatar`: 36 pt, a dark 2 pt edge, card fill.
fn avatar(theme: &Theme) -> El {
    el().size(36.0, 36.0).fill(theme.bg_card).border(2.0, rune::Color::BLACK.alpha(0.3))
}

fn add_area(app: &App, theme: &Theme) -> El {
    let accounts = &app.accounts;
    let mut area = col().key("add-area").mt(18.0).pt(16.0).border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle);
    if !accounts.microsoft_configured {
        let (pre, code, post) = (tr("account.notConfigured"), ".env (AZURE_CLIENT_ID)", tr("account.notConfiguredSuffix"));
        let code_start = pre.len() + 1;
        let content = format!("{pre} {code} {post}");
        let span = Span { mono: true, size: Some(11.5), bg: Some(theme.bg_input), bg_pad: 3.0, ..Span::new(code_start, code_start + code.len()) };
        area = area.child(
            rich(content, vec![span])
                .font(rune::tok::TextStyle { line_height: 12.5 * 1.6, ..theme.sans(12.5, 400.0) })
                .color(theme.text_muted)
                .key("notice"),
        );
    } else if let Some(device) = &accounts.device_code {
        let code = device.user_code.clone();
        let copy = move |code: String| {
            move |app: &mut App, tx: &mut Tx| {
                tx.copy(code.clone());
                app.toasts.success(tr("account.codeCopied"));
            }
        };
        let hovered = app.account_modal.hovered.as_deref() == Some("device-code");
        let code_btn = el()
            .key("device-code")
            .px(20.0)
            .py(10.0)
            .fill(theme.bg_input)
            .border(2.0, if hovered { theme.accent } else { theme.border })
            .role("button")
            .label(code.clone())
            .help(tr("account.clickToCopy"))
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_hover_change(|app: &mut App, over, _| set_hovered(app, "device-code", over))
            .on_press_tx(copy(code.clone()))
            .child(
                text(code.clone())
                    .font(rune::tok::TextStyle { weight: 700.0, tracking: 28.0 * 0.12, ..theme.pixel(28.0) })
                    .color(theme.accent)
                    .selectable(true),
            );
        let uri = device.verification_uri.clone();
        let mut device_flow = col()
            .key("device-flow")
            .items_center()
            .gap(10.0)
            .child(text(tr("account.signInMicrosoft")).font(theme.sans(14.0, 600.0)).color(theme.text))
            .child(text(tr("account.enterCode")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary))
            .child(
                row().center_y().gap(10.0).child(code_btn).child(
                    Btn::new(tr("account.copy")).key("copy-code").ghost().sm().icon(&icons::COPY).on_press_tx(copy(code)).build(theme),
                ),
            )
            .child(
                Btn::new(tf("account.openUri", &[("uri", &device.verification_uri)]))
                    .key("open-uri")
                    .primary()
                    .on_press_tx(move |_: &mut App, tx: &mut Tx| {
                        let _ = tx.open_url(&uri);
                    })
                    .build(theme),
            )
            .child(
                row()
                    .center_y()
                    .gap(8.0)
                    .mt(4.0)
                    .child(ui::spinner(theme, 13.0))
                    .child(
                        text(if device.authorizing { tr("account.signingIn") } else { tr("account.waitingAuthorize") })
                            .font(theme.sans(12.5, 400.0))
                            .color(theme.text_muted),
                    ),
            );
        if !device.authorizing {
            device_flow = device_flow.child(Btn::new(tr("common.cancel")).key("cancel-login").ghost().sm().on_press(|app: &mut App| app.cancel_login()).build(theme));
        }
        area = area.child(device_flow);
    } else {
        area = area.child(
            Btn::new(if accounts.logging_in { tr("account.starting") } else { tr("account.addMicrosoft") })
                .key("add-microsoft")
                .primary()
                .icon(&icons::PLUS)
                .full()
                .disabled(accounts.logging_in)
                .on_press(|app: &mut App| app.login_microsoft())
                .build(theme),
        );
    }
    if let Some(err) = &accounts.login_error {
        area = area.child(text(err.clone()).font(theme.sans(12.5, 400.0)).color(theme.danger).wrap().mt(12.0).key("login-error"));
    }
    area
}

fn achievements(theme: &Theme) -> El {
    col()
        .key("achv-area")
        .items_center()
        .mt(14.0)
        .pt(14.0)
        .border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle)
        .child(
            Btn::new(tr("account.achievementsStats"))
                .key("achievements")
                .ghost()
                .icon(&icons::TROPHY)
                .full()
                .on_press(|app: &mut App| {
                    close(app);
                    app.go(Route::Achievements);
                })
                .build(theme),
        )
        .child(text(tr("account.achievementsHint")).font(theme.sans(11.0, 400.0)).color(theme.text_muted).mt(8.0))
}
