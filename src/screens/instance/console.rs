//! The Logs tab (clients) and Console tab (servers, with a command line).

use rune::{Color, Cx, El, ScrollSpec, Shadow, col, el, row, text};

use super::{t_, text_field};
use crate::app::{App, Ignore};
use crate::core::i18n::tf;
use crate::instance::{Instance, InstanceKind};
use crate::theme::Theme;
use crate::ui::{self, Btn};

fn send(app: &mut App) {
    let command = app.instance_page.command.text.trim().to_string();
    let id = app.instance_page.id.clone();
    if command.is_empty() || !app.is_running(&id) {
        return;
    }
    let host = app.fx.host.clone();
    match crate::commands::send_server_command(host.state(), id, command) {
        Ok(()) => app.instance_page.command = rune::TextEdit::default(),
        Err(e) => app.toasts.error(e.to_string()).ignore(),
    }
}

pub fn view(app: &App, theme: &Theme, inst: &Instance, cx: &mut Cx<App>) -> El {
    let is_server = inst.kind == InstanceKind::Server;
    let runtime = app.launch.get(&inst.id);
    let state = runtime.map_or("", |runtime| runtime.state.as_str());
    let lines = runtime.map_or(0, |runtime| runtime.logs.len());
    let running = app.is_running(&inst.id);

    let pill_color = match state {
        "running" => (theme.accent, theme.accent),
        "error" => (theme.danger, theme.danger),
        _ => (theme.text_secondary, theme.border),
    };
    let pill = el().px(10.0).py(3.0).fill(theme.bg_card).border(2.0, pill_color.1).child(
        text(state.to_uppercase())
            .font(rune::tok::TextStyle { tracking: 11.0 * 0.04, ..theme.pixel(11.0) })
            .color(pill_color.0),
    );
    let mut panel = col()
        .w_full()
        .child(row().gap(12.0).center_y().mb(12.0).child(pill.key("state-pill")).child(ui::muted(theme, tf("instance.lines", &[("count", &lines.to_string())]), 12.0)));

    match runtime.filter(|runtime| !runtime.logs.is_empty()) {
        None => {
            let msg = if is_server { t_("instance.noOutputServer") } else { t_("instance.noOutputGame") };
            panel = panel.child(ui::muted(theme, msg, 14.0).wrap().py(24.0));
        }
        Some(runtime) => {
            let page_h = cx.layout_size("instance-page").map_or(760.0, |size| size[1]);
            let joined = runtime.logs.iter().map(String::as_str).collect::<Vec<_>>().join("\n");
            panel = panel.child(
                col()
                    .key("log-view")
                    .w_full()
                    .max_h(page_h * 0.6)
                    .min_h(0.0)
                    .fill(theme.bg_input)
                    .border(2.0, theme.border)
                    .box_shadow(Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.3), inset: true })
                    .scroll(ScrollSpec { stick_bottom: true })
                    .child(
                        text(joined)
                            .font(rune::tok::TextStyle { line_height: 18.0, ..theme.mono(12.0) })
                            .color(theme.text_secondary)
                            .wrap_anywhere()
                            .selectable(true)
                            .px(14.0)
                            .py(12.0),
                    ),
            );
        }
    }

    if is_server {
        let page = &app.instance_page;
        let placeholder = if running { t_("instance.commandPlaceholder") } else { t_("instance.commandPlaceholderStopped") };
        let mut field = text_field(theme, &page.command, placeholder, |app| &mut app.instance_page.command)
            .font(theme.mono(12.5))
            .on_submit(|app: &mut App, _| send(app))
            .key("command")
            .py(10.0);
        if !running {
            field = field.disabled(true).color(theme.text_muted);
        }
        panel = panel.child(
            row()
                .key("console-input")
                .gap(8.0)
                .center_y()
                .mt(10.0)
                .px(10.0)
                .fill(theme.bg_input)
                .border(2.0, theme.border)
                .child(text(">").font(rune::tok::TextStyle { weight: 700.0, ..theme.mono(13.0) }).color(theme.accent))
                .child(field)
                .child(Btn::new(t_("instance.send")).primary().sm().key("send").disabled(!running || page.command.text.trim().is_empty()).on_press(send).build(theme)),
        );
    }
    panel
}
