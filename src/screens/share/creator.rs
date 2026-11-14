//! `/share/creator` (`src/routes/share/creator/+page.svelte`): create or edit your board,
//! publish an instance to it (or as a standalone code), post announcements.

use rune::{Cursor, Cx, El, TextEdit, col, el, row, text};
use serde_json::json;

use super::api::{self, Board};
use super::{adopt, board_skeleton, card, field, link, login, select, session_matches, text_input, token};
use crate::app::{App, Ignore, Route};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

pub struct State {
    boards: Vec<Board>,
    loading: bool,

    handle: TextEdit,
    display_name: TextEdit,
    kind: String,
    description: TextEdit,
    stream_url: TextEdit,
    server_address: TextEdit,
    creating: bool,

    /// "" = standalone (code only), or the board handle.
    publish_to: String,
    instance_id: String,
    format: String,
    changelog: TextEdit,
    publishing: bool,

    message: TextEdit,
    posting: bool,
    deleting_msg: Option<String>,

    edit_name: TextEdit,
    edit_desc: TextEdit,
    edit_stream: TextEdit,
    edit_server: TextEdit,
    edit_public: bool,
    saving_edit: bool,
    confirm_delete: bool,
}

impl Default for State {
    fn default() -> State {
        State {
            boards: Vec::new(),
            loading: false,
            handle: TextEdit::default(),
            display_name: TextEdit::default(),
            kind: "creator".into(),
            description: TextEdit::default(),
            stream_url: TextEdit::default(),
            server_address: TextEdit::default(),
            creating: false,
            publish_to: String::new(),
            instance_id: String::new(),
            format: "cactuspack".into(),
            changelog: TextEdit::default(),
            publishing: false,
            message: TextEdit::default(),
            posting: false,
            deleting_msg: None,
            edit_name: TextEdit::default(),
            edit_desc: TextEdit::default(),
            edit_stream: TextEdit::default(),
            edit_server: TextEdit::default(),
            edit_public: true,
            saving_edit: false,
            confirm_delete: false,
        }
    }
}

pub fn on_enter(app: &mut App) {
    app.share.creator = State::default();
    super::ensure_login(app);
    if api::configured() && app.share.session.is_some() {
        load_boards(app);
    }
}

pub fn load_boards(app: &mut App) {
    let Some(token) = token(app) else { return };
    app.share.creator.loading = true;
    let host = app.fx.host.clone();
    app.fx.spawn(api::my_boards(host, token), |app, (result, fresh)| {
        adopt(app, fresh);
        let creator = &mut app.share.creator;
        creator.loading = false;
        creator.boards = result.unwrap_or_default();
        if creator.publish_to.is_empty() {
            if let Some(board) = creator.boards.first() {
                creator.publish_to = board.handle.clone();
            }
        }
        // The `$effect` on `myBoard`: refill the edit form from the board.
        creator.confirm_delete = false;
        if let Some(board) = creator.boards.first().cloned() {
            creator.edit_name = TextEdit::new(board.display_name);
            creator.edit_desc = TextEdit::new(board.description.unwrap_or_default());
            creator.edit_stream = TextEdit::new(board.stream_url.unwrap_or_default());
            creator.edit_server = TextEdit::new(board.server_address.unwrap_or_default());
            creator.edit_public = board.is_public;
        }
        app.share.has_board = !app.share.creator.boards.is_empty();
    });
}

fn my_handle(app: &App) -> Option<String> {
    app.share.creator.boards.first().map(|board| board.handle.clone())
}

fn non_empty(edit: &TextEdit) -> Option<String> {
    Some(edit.text.trim().to_string()).filter(|value| !value.is_empty())
}

fn create_board(app: &mut App) {
    let Some(token) = token(app) else { return };
    let creator = &mut app.share.creator;
    if creator.creating {
        return;
    }
    creator.creating = true;
    let handle = creator.handle.text.trim().to_string();
    let new_board = api::NewBoard {
        handle: handle.to_lowercase(),
        display_name: non_empty(&creator.display_name).unwrap_or(handle),
        kind: creator.kind.clone(),
        description: non_empty(&creator.description),
        stream_url: non_empty(&creator.stream_url),
        server_address: non_empty(&creator.server_address),
    };
    let host = app.fx.host.clone();
    app.fx.spawn(api::create_board(host, token, new_board), |app, (result, fresh)| {
        adopt(app, fresh);
        app.share.creator.creating = false;
        match result {
            Ok(_) => {
                let creator = &mut app.share.creator;
                for edit in [&mut creator.handle, &mut creator.display_name, &mut creator.description, &mut creator.stream_url, &mut creator.server_address] {
                    edit.replace_text("");
                }
                load_boards(app);
                app.toasts.success(tr("community.boardCreated")).ignore();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

fn publish(app: &mut App) {
    let Some(token) = token(app) else { return };
    let creator = &mut app.share.creator;
    if creator.instance_id.is_empty() || creator.publishing {
        return;
    }
    creator.publishing = true;
    let target = Some(creator.publish_to.clone()).filter(|handle| !handle.is_empty());
    let standalone = target.is_none();
    let host = app.fx.host.clone();
    app.fx.spawn(api::publish(host, creator.instance_id.clone(), creator.format.clone(), token, target, creator.changelog.text.clone()), move |app, result| {
        app.share.creator.publishing = false;
        match result {
            Ok(_) => {
                app.share.creator.changelog.replace_text("");
                let msg = if standalone { tr("community.publishedStandalone") } else { tr("community.publishedToBoard") };
                app.toasts.success(msg).ignore();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

fn post_message(app: &mut App) {
    let (Some(token), Some(handle)) = (token(app), my_handle(app)) else { return };
    let creator = &mut app.share.creator;
    let body = creator.message.text.trim().to_string();
    if body.is_empty() || creator.posting {
        return;
    }
    creator.posting = true;
    let host = app.fx.host.clone();
    app.fx.spawn(api::post_message(host, token, handle, body), |app, (result, fresh)| {
        adopt(app, fresh);
        app.share.creator.posting = false;
        match result {
            Ok(()) => {
                app.share.creator.message.replace_text("");
                load_boards(app);
                app.toasts.success(tr("community.announcementPosted")).ignore();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

fn delete_message(app: &mut App, id: String) {
    let (Some(token), Some(handle)) = (token(app), my_handle(app)) else { return };
    if app.share.creator.deleting_msg.is_some() {
        return;
    }
    app.share.creator.deleting_msg = Some(id.clone());
    let host = app.fx.host.clone();
    app.fx.spawn(api::delete_message(host, token, handle, id), |app, (result, fresh)| {
        adopt(app, fresh);
        app.share.creator.deleting_msg = None;
        match result {
            Ok(()) => {
                load_boards(app);
                app.toasts.success(tr("community.announcementDeleted")).ignore();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

fn save_edit(app: &mut App) {
    let (Some(token), Some(handle)) = (token(app), my_handle(app)) else { return };
    let creator = &mut app.share.creator;
    if creator.saving_edit {
        return;
    }
    creator.saving_edit = true;
    let patch = json!({
        "displayName": creator.edit_name.text.trim(),
        "description": creator.edit_desc.text.trim(),
        "streamUrl": creator.edit_stream.text.trim(),
        "serverAddress": creator.edit_server.text.trim(),
        "isPublic": creator.edit_public,
    });
    let host = app.fx.host.clone();
    app.fx.spawn(api::update_board(host, token, handle, patch), |app, (result, fresh)| {
        adopt(app, fresh);
        app.share.creator.saving_edit = false;
        match result {
            Ok(()) => {
                load_boards(app);
                app.toasts.success(tr("community.boardSaved")).ignore();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

fn delete_board(app: &mut App) {
    let (Some(token), Some(handle)) = (token(app), my_handle(app)) else { return };
    let host = app.fx.host.clone();
    app.fx.spawn(api::delete_board(host, token, handle), |app, (result, fresh)| {
        adopt(app, fresh);
        match result {
            Ok(()) => {
                app.share.creator.publish_to.clear();
                app.share.creator.confirm_delete = false;
                load_boards(app);
                app.toasts.success(tr("community.boardDeleted")).ignore();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

/// The creator page's `timeAgo`, translated.
fn time_ago(iso: &str) -> String {
    let Ok(at) = chrono::DateTime::parse_from_rfc3339(iso) else { return String::new() };
    let secs = (chrono::Utc::now() - at.with_timezone(&chrono::Utc)).num_seconds().max(0) as f64;
    if secs < 90.0 {
        return tr("community.justNow");
    }
    let minutes = (secs / 60.0).round();
    if minutes < 60.0 {
        return tf("community.minutesAgo", &[("count", &minutes.to_string())]);
    }
    let hours = (minutes / 60.0).round();
    if hours < 24.0 {
        return tf("community.hoursAgo", &[("count", &hours.to_string())]);
    }
    tf("community.daysAgo", &[("count", &(hours / 24.0).round().to_string())])
}

fn panel(theme: &Theme, key: &str) -> El {
    card(theme.bg_card, theme.border, 2.0, 20.0, 18.0).key(key.to_string()).mb(16.0)
}

fn muted(theme: &Theme, line: impl Into<String>) -> El {
    ui::muted(theme, line, 13.0).wrap().mb(12.0)
}

fn input(theme: &Theme, edit: &TextEdit, placeholder: String, access: fn(&mut App) -> &mut TextEdit) -> El {
    field(theme, edit, text_input(theme, edit, placeholder, access))
}

pub fn view(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let creator = &app.share.creator;
    let my_board = creator.boards.first();
    let mut page = col().key("creator").w_full().max_w(720.0).self_center().px(32.0).py(24.0)
        .child(el().mb(8.0).self_start().child(super::link(&theme, "back", format!("← {}", tr("nav.community")), 13.0, theme.text_secondary, |app| app.go(Route::Community))))
        .child(ui::heading(&theme, if my_board.is_some() { tr("community.yourBoard") } else { tr("community.createBoard") }, 24.0).mb(18.0));

    if !api::configured() {
        return page.child(panel(&theme, "not-configured").child(ui::muted(&theme, tr("community.boardsNotConfigured"), 13.0).wrap()));
    }
    let Some(account) = app.active_account() else {
        return page.child(
            panel(&theme, "add-account")
                .child(ui::h3(&theme, tr("community.addMicrosoftAccount")).mb(8.0))
                .child(muted(&theme, tr("community.boardsTiedToAccount")))
                .child(el().self_start().child(
                    Btn::new(tr("community.addAccount")).primary().icon(&icons::USER).key("add-account").on_press(|app| app.ui.accounts_open = true).build(&theme),
                )),
        );
    };
    if !session_matches(app) {
        if let Some(error) = &app.share.auth_error {
            return page.child(
                panel(&theme, "session-failed")
                    .child(row().mb(12.0)
                        .child(ui::muted(&theme, format!("{} ", tr("community.creatorSessionFailedPrefix")), 13.0))
                        .child(text(account.username.clone()).font(theme.sans(13.0, 600.0)).color(theme.text_muted))
                        .child(ui::muted(&theme, ".", 13.0)))
                    .child(text(error.clone()).font(theme.sans(13.0, 400.0)).color(theme.danger).wrap().mb(10.0))
                    .child(el().self_start().child(Btn::new(tr("common.retry")).ghost().key("retry").on_press(login).build(&theme))),
            );
        }
        return page.child(board_skeleton(&theme));
    }

    let who = app.share.session.as_ref().map(|session| session.name.clone()).unwrap_or_default();
    page = page.child(
        row().key("who").justify_between().center_y().mb(16.0)
            .child(row()
                .child(ui::secondary(&theme, format!("{} ", tr("community.signedInAs")), 13.0))
                .child(text(who).font(theme.sans(13.0, 600.0)).color(theme.text_secondary)))
            .child(link(&theme, "switch-account", tr("community.switchAccount"), 12.0, theme.text_muted, |app| app.ui.accounts_open = true)),
    );
    if creator.loading {
        page = page.child(board_skeleton(&theme));
    }

    if let Some(board) = my_board {
        let handle = board.handle.clone();
        let mut form = panel(&theme, "edit-board")
            .child(row().justify_between().center_y().gap(10.0).mb(8.0)
                .child(ui::h3(&theme, tf("community.yourBoardHandle", &[("handle", &board.handle)])))
                .child(link(&theme, "view-public", tr("community.viewPublicPage"), 12.0, theme.text_muted, move |app| app.go(Route::ShareBoard(handle.clone())))))
            .child(input(&theme, &creator.edit_name, tr("community.displayNamePlaceholder"), |app| &mut app.share.creator.edit_name))
            .child(input(&theme, &creator.edit_desc, tr("community.descriptionOptional"), |app| &mut app.share.creator.edit_desc).mt(10.0));
        if board.kind == "streamer" {
            form = form.child(input(&theme, &creator.edit_stream, tr("community.streamUrlPlaceholder"), |app| &mut app.share.creator.edit_stream).mt(10.0));
        }
        if board.kind == "server" {
            form = form.child(input(&theme, &creator.edit_server, tr("community.serverAddressPlaceholder"), |app| &mut app.share.creator.edit_server).mt(10.0));
        }
        let public = creator.edit_public;
        form = form.child(
            row().key("public").mt(10.0).gap(8.0).center_y().self_start().role("checkbox").checked(public).label(tr("community.publicSearchable"))
                .focusable(true)
                .cursor(Cursor::Pointer)
                .on_press(|app: &mut App| app.share.creator.edit_public = !app.share.creator.edit_public)
                .child(ui::checkbox(&theme, public).a11y_hidden())
                .child(ui::secondary(&theme, tr("community.publicSearchable"), 13.0).a11y_hidden()),
        );
        let mut actions = row().key("edit-actions").mt(10.0).gap(10.0).flex_wrap()
            .child(Btn::new(if creator.saving_edit { tr("community.saving") } else { tr("community.saveChanges") }).primary().key("save").disabled(creator.saving_edit).on_press(save_edit).build(&theme));
        actions = if creator.confirm_delete {
            actions
                .child(Btn::new(tr("community.confirmDelete")).danger().key("confirm-delete").on_press(delete_board).build(&theme))
                .child(Btn::new(tr("common.cancel")).ghost().key("cancel-delete").on_press(|app| app.share.creator.confirm_delete = false).build(&theme))
        } else {
            actions.child(Btn::new(tr("community.deleteBoard")).danger().key("delete-board").on_press(|app| app.share.creator.confirm_delete = true).build(&theme))
        };
        page = page.child(form.child(actions));
    }

    if creator.boards.is_empty() && !creator.loading {
        let kinds = [
            ("creator".to_string(), tr("community.kindCreator")),
            ("streamer".to_string(), tr("community.kindStreamer")),
            ("server".to_string(), tr("community.kindServer")),
        ];
        let mut form = panel(&theme, "create-board")
            .child(ui::h3(&theme, tr("community.createYourBoard")).mb(8.0))
            .child(row().gap(10.0).flex_wrap()
                .child(input(&theme, &creator.handle, tr("community.handlePlaceholder"), |app| &mut app.share.creator.handle).grow(1.0).min_w(150.0))
                .child(input(&theme, &creator.display_name, tr("community.displayNamePlaceholder"), |app| &mut app.share.creator.display_name).grow(1.0).min_w(150.0))
                .child(select(app, layer, "board-kind", Some(160.0), &kinds, &creator.kind, "", |app, value| app.share.creator.kind = value)))
            .child(input(&theme, &creator.description, tr("community.shortDescriptionOptional"), |app| &mut app.share.creator.description).mt(10.0));
        if creator.kind == "streamer" {
            form = form.child(input(&theme, &creator.stream_url, tr("community.streamUrlPlaceholder"), |app| &mut app.share.creator.stream_url).mt(10.0));
        }
        if creator.kind == "server" {
            form = form.child(input(&theme, &creator.server_address, tr("community.serverAddressPlaceholder"), |app| &mut app.share.creator.server_address).mt(10.0));
        }
        let can_create = !creator.creating && !creator.handle.text.trim().is_empty();
        form = form.child(el().mt(10.0).self_start().child(
            Btn::new(if creator.creating { tr("common.creating") } else { tr("community.createBoardButton") }).primary().key("create").disabled(!can_create).on_press(create_board).build(&theme),
        ));
        page = page.child(form);
    }

    let mut targets = vec![(String::new(), tr("community.standaloneCodeOnly"))];
    if let Some(board) = my_board {
        targets.push((board.handle.clone(), format!("@{}", board.handle)));
    }
    let instances: Vec<(String, String)> =
        app.instances.iter().map(|instance| (instance.id.clone(), format!("{} · {} {}", instance.name, instance.loader.id(), instance.mc_version))).collect();
    let formats = [("cactuspack".to_string(), ".cactuspack".to_string()), ("mrpack".to_string(), ".mrpack".to_string())];
    let line = if creator.publish_to.is_empty() {
        tr("community.snapshotStandalone")
    } else {
        tf("community.snapshotToBoard", &[("handle", &creator.publish_to)])
    };
    page = page.child(
        panel(&theme, "publish")
            .child(ui::h3(&theme, tr("community.publishInstance")).mb(8.0))
            .child(muted(&theme, line))
            .child(row().gap(10.0).flex_wrap()
                .child(select(app, layer, "publish-to", Some(160.0), &targets, &creator.publish_to, "", |app, value| app.share.creator.publish_to = value))
                .child(select(app, layer, "publish-instance", Some(160.0), &instances, &creator.instance_id, &tr("community.chooseInstance"), |app, value| app.share.creator.instance_id = value))
                .child(select(app, layer, "publish-format", Some(160.0), &formats, &creator.format, "", |app, value| app.share.creator.format = value)))
            .child(input(&theme, &creator.changelog, tr("community.changelogPlaceholder"), |app| &mut app.share.creator.changelog).mt(10.0))
            .child(el().mt(10.0).self_start().child(
                Btn::new(if creator.publishing { tr("community.publishing") } else { tr("community.publish") })
                    .primary()
                    .key("publish")
                    .disabled(creator.publishing || creator.instance_id.is_empty())
                    .on_press(publish)
                    .build(&theme),
            )),
    );

    if let Some(board) = my_board {
        let area = rune::text_input(&creator.message, theme.sans(13.0, 400.0))
            .placeholder(tr("community.announcementPlaceholder"), theme.text_muted)
            .color(theme.text)
            .selection_color(theme.accent.alpha(0.6))
            .on_edit(|app: &mut App| &mut app.share.creator.message)
            .grow(1.0)
            .min_h(36.0);
        let mut announce = panel(&theme, "announce")
            .child(ui::h3(&theme, tf("community.postAnnouncementTo", &[("handle", &board.handle)])).mb(8.0))
            .child(ui::input_frame(&theme, creator.message.focused).py(9.0).items_start().child(area))
            .child(el().mt(10.0).self_start().child(
                Btn::new(if creator.posting { tr("community.posting") } else { tr("community.post") })
                    .ghost()
                    .key("post")
                    .disabled(creator.posting || creator.message.text.trim().is_empty())
                    .on_press(post_message)
                    .build(&theme),
            ));
        if !board.messages.is_empty() {
            let mut list = col().key("msgs").mt(14.0).gap(6.0);
            for msg in &board.messages {
                let deleting = creator.deleting_msg.as_deref() == Some(msg.id.as_str());
                let id = msg.id.clone();
                let mut delete_button = el().key("del").px(4.0).py(2.0).role("button").label(tr("community.deleteAnnouncement")).help(tr("community.deleteAnnouncement"));
                delete_button = if deleting {
                    delete_button.opacity(0.5).disabled(true).child(ui::muted(&theme, "…", 14.0))
                } else {
                    delete_button.focusable(true).cursor(Cursor::Pointer).hover(rune::Color::WHITE.alpha(0.06)).on_press(move |app: &mut App| delete_message(app, id.clone()))
                        .child(ui::colored_icon(&icons::TRASH, 14.0, theme.text_muted))
                };
                list = list.child(
                    row().key(format!("msg-{}", msg.id)).items_start().gap(10.0).px(14.0).py(12.0).fill(theme.bg_input).border(2.0, theme.border_subtle)
                        .child(col().grow(1.0).shrink().min_w(0.0)
                            .child(text(msg.body.clone()).font(theme.sans(13.0, 400.0)).color(theme.text).wrap_anywhere())
                            .child(ui::muted(&theme, time_ago(&msg.created_at), 11.0)))
                        .child(delete_button),
                );
            }
            announce = announce.child(list);
        }
        page = page.child(announce);
    }
    page
}

