//! `PlayersList.svelte`: a server's operators and whitelist.

use rune::{El, TextEdit, col, rich, row, text};

use super::{boxed, icon_btn, t_, text_field};
use crate::app::{App, Ignore};
use crate::core::i18n::tf;
use crate::icons;
use crate::players::{OpEntry, PlayerEntry};
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    pub ops: Vec<OpEntry>,
    pub whitelist: Vec<PlayerEntry>,
    pub op_name: TextEdit,
    pub whitelist_name: TextEdit,
    pub busy: bool,
}

pub fn load(app: &mut App) {
    let id = app.instance_page.id.clone();
    app.fx.blocking(
        move |host| {
            let result = crate::commands::read_ops(host.clone(), id.clone()).and_then(|ops| Ok((ops, crate::commands::read_whitelist(host.clone(), id.clone())?)));
            (result, id)
        },
        |app, (result, id)| {
            if app.instance_page.id != id {
                return;
            }
            match result {
                Ok((ops, whitelist)) => {
                    app.instance_page.players.ops = ops;
                    app.instance_page.players.whitelist = whitelist;
                }
                Err(e) => app.toasts.error(e.to_string()).ignore(),
            }
        },
    );
}

#[derive(Clone, Copy)]
enum Edit {
    AddOp,
    RemoveOp,
    AddWhitelist,
    RemoveWhitelist,
}

/// On a running server the change goes through the console; otherwise into the JSON files.
fn run(app: &mut App, edit: Edit, name: String) {
    app.instance_page.players.busy = true;
    let id = app.instance_page.id.clone();
    let running = app.is_running(&id);
    let host = app.fx.host.clone();
    app.fx.spawn(
        async move {
            if running {
                let command = match edit {
                    Edit::AddOp => format!("op {name}"),
                    Edit::RemoveOp => format!("deop {name}"),
                    Edit::AddWhitelist => format!("whitelist add {name}"),
                    Edit::RemoveWhitelist => format!("whitelist remove {name}"),
                };
                crate::commands::send_server_command(host.state(), id, command)?;
                tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                Ok(())
            } else {
                match edit {
                    Edit::AddOp => crate::commands::add_op(host, id, name, 4).await,
                    Edit::RemoveOp => crate::commands::remove_op(host, id, name),
                    Edit::AddWhitelist => crate::commands::add_whitelist(host, id, name).await,
                    Edit::RemoveWhitelist => crate::commands::remove_whitelist(host, id, name),
                }
            }
        },
        move |app, result: crate::error::Result<()>| {
            app.instance_page.players.busy = false;
            if let Err(e) = result {
                app.toasts.error(e.to_string());
            }
            match edit {
                Edit::AddOp => app.instance_page.players.op_name = TextEdit::default(),
                Edit::AddWhitelist => app.instance_page.players.whitelist_name = TextEdit::default(),
                _ => {}
            }
            load(app);
        },
    );
}

fn add_op(app: &mut App) {
    let name = app.instance_page.players.op_name.text.trim().to_string();
    if !name.is_empty() && !app.instance_page.players.busy {
        run(app, Edit::AddOp, name);
    }
}

fn add_to_whitelist(app: &mut App) {
    let name = app.instance_page.players.whitelist_name.text.trim().to_string();
    if !name.is_empty() && !app.instance_page.players.busy {
        run(app, Edit::AddWhitelist, name);
    }
}

fn block(theme: &Theme, key: &str, title: String, count: usize) -> El {
    col()
        .key(key.to_string())
        .w_full()
        .fill(theme.bg_card)
        .border(2.0, theme.border)
        .px(18.0)
        .py(16.0)
        .child(row().gap(8.0).center_y().child(ui::h3(theme, title)).child(ui::muted(theme, count.to_string(), 14.0)))
}

fn player_row(theme: &Theme, key: String, name: &str, extra: Option<String>, remove_button: El) -> El {
    let mut entry = row()
        .key(key)
        .gap(10.0)
        .center_y()
        .px(10.0)
        .py(8.0)
        .fill(theme.bg_input)
        .border(2.0, theme.border_subtle)
        .child(text(name.to_string()).font(theme.sans(13.0, 600.0)).color(theme.text).grow(1.0).shrink().truncate());
    if let Some(detail) = extra {
        entry = entry.child(ui::muted(theme, detail, 11.0));
    }
    entry.child(remove_button)
}

pub fn view(app: &App, theme: &Theme) -> El {
    let players = &app.instance_page.players;
    let running = app.is_running(&app.instance_page.id);
    let lead = ui::muted(theme, if running { t_("server.playersLiveHint") } else { t_("server.playersStoppedHint") }, 12.5).wrap().max_w(560.0);

    let add_row = |key: &str, edit: &TextEdit, access: fn(&mut App) -> &mut TextEdit, label: String, on_add: fn(&mut App)| {
        let mut field = text_field(theme, edit, t_("server.playerNamePlaceholder"), access).key(format!("{key}-input")).on_submit(move |app: &mut App, _| on_add(app));
        if players.busy {
            field = field.disabled(true);
        }
        row()
            .gap(8.0)
            .max_w(360.0)
            .mb(12.0)
            .center_y()
            .child(boxed(theme, edit, field).grow(1.0))
            .child(Btn::new(label).primary().sm().key(format!("{key}-add")).disabled(players.busy || edit.text.trim().is_empty()).on_press(on_add).build(theme))
    };

    let mut ops = block(theme, "ops", t_("server.operators"), players.ops.len())
        .child(ui::muted(theme, t_("server.opsHint"), 12.0).wrap().mt(4.0).mb(12.0))
        .child(add_row("op", &players.op_name, |app| &mut app.instance_page.players.op_name, t_("server.op"), add_op));
    if players.ops.is_empty() {
        ops = ops.child(ui::muted(theme, t_("server.noOperators"), 12.5));
    } else {
        let mut rows = col().gap(6.0);
        for op in &players.ops {
            let name = op.name.clone();
            let remove_button = icon_btn(theme, format!("deop-{}", op.uuid), &icons::TRASH, 13.0, 3.0, t_("server.removeOp"), players.busy, move |app| run(app, Edit::RemoveOp, name.clone()));
            rows = rows.child(player_row(theme, format!("op-{}", op.uuid), &op.name, Some(tf("server.opLevel", &[("level", &op.level.to_string())])), remove_button));
        }
        ops = ops.child(rows);
    }

    let (prefix, code, suffix) = (t_("server.whitelistHintPrefix"), "white-list", t_("server.whitelistHintSuffix"));
    let hint = rich(format!("{prefix}{code}{suffix}"), vec![rune::Span::new(prefix.len(), prefix.len() + code.len()).color(theme.accent)])
        .font(theme.sans(12.0, 400.0))
        .color(theme.text_muted)
        .wrap()
        .mt(4.0)
        .mb(12.0);
    let mut whitelist = block(theme, "whitelist", t_("server.whitelist"), players.whitelist.len())
        .child(hint)
        .child(add_row("wl", &players.whitelist_name, |app| &mut app.instance_page.players.whitelist_name, t_("common.add"), add_to_whitelist));
    if players.whitelist.is_empty() {
        whitelist = whitelist.child(ui::muted(theme, t_("server.whitelistEmpty"), 12.5));
    } else {
        let mut rows = col().gap(6.0);
        for player in &players.whitelist {
            let name = player.name.clone();
            let remove_button = icon_btn(theme, format!("unwl-{}", player.uuid), &icons::TRASH, 13.0, 3.0, t_("common.remove"), players.busy, move |app| run(app, Edit::RemoveWhitelist, name.clone()));
            rows = rows.child(player_row(theme, format!("wl-{}", player.uuid), &player.name, None, remove_button));
        }
        whitelist = whitelist.child(rows);
    }

    col().w_full().gap(18.0).child(lead).child(ops).child(whitelist)
}
