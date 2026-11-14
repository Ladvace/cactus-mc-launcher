//! `ServerAddress.svelte` (localhost / LAN address chips) and `ServerShare.svelte` (an ngrok
//! tunnel to the server's port).

use std::time::Duration;

use rune::{Cursor, El, TextEdit, Tx, el, row, text};

use super::{boxed, chip, t_, text_field};
use crate::app::App;
use crate::icons;
use crate::instance::{Instance, UpdateInstance};
use crate::theme::Theme;
use crate::ui::{self, Btn};

const DEFAULT_PORT: u16 = 25565;
const NGROK_AUTHTOKEN_URL: &str = "https://dashboard.ngrok.com/get-started/your-authtoken";

#[derive(Default)]
pub struct State {
    pub port: u16,
    pub lan_ip: Option<String>,
    /// The address last copied and when (`app.now`), for the check mark.
    pub copied: Option<(String, f64)>,
    pub tunnel: String,
    pub busy: bool,
    pub error: Option<String>,
    pub editing_token: bool,
    pub token: TextEdit,
}

/// serverAddress.ts `parseServerPort`.
fn parse_port(props: &str) -> u16 {
    for line in props.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("server-port") {
            if let Some(value) = rest.trim_start().strip_prefix('=') {
                let digits: String = value.trim_start().chars().take_while(char::is_ascii_digit).collect();
                if !digits.is_empty() {
                    return digits.parse().ok().filter(|port| *port != 0).unwrap_or(DEFAULT_PORT);
                }
            }
        }
    }
    DEFAULT_PORT
}

fn format_address(host: &str, port: u16) -> String {
    if port == DEFAULT_PORT { host.to_string() } else { format!("{host}:{port}") }
}

pub fn load(app: &mut App) {
    app.instance_page.server.port = DEFAULT_PORT;
    let id = app.instance_page.id.clone();
    app.fx.blocking(
        move |host| {
            let port = crate::commands::read_server_properties(host.clone(), id.clone()).map_or(DEFAULT_PORT, |props| parse_port(&props));
            (id, port, crate::commands::get_local_ip())
        },
        |app, (id, port, ip)| {
            if app.instance_page.id == id {
                app.instance_page.server.port = port;
                app.instance_page.server.lan_ip = ip;
            }
        },
    );
    let host = app.fx.host.clone();
    app.instance_page.server.tunnel = crate::tunnel::tunnel_status(host.state()).unwrap_or_default();
}

fn copy_chip(app: &mut App, tx: &mut Tx, addr: String) {
    tx.copy(addr.clone());
    app.instance_page.server.copied = Some((addr, app.now));
    tx.wake_after(Duration::from_millis(1600));
}

/// `ServerAddress.svelte`.
pub fn address(app: &App, theme: &Theme) -> El {
    let server = &app.instance_page.server;
    let copied = server.copied.as_ref().filter(|(_, at)| app.now - at < 1.5).map(|(addr, _)| addr.as_str());
    let port = if server.port == 0 { DEFAULT_PORT } else { server.port };
    let mut chips = row()
        .key("server-address")
        .flex_wrap()
        .gap(10.0)
        .center_y()
        .mt(12.0)
        .child(row().gap(6.0).center_y().child(ui::colored_icon(&icons::GLOBE, 13.0, theme.text_muted)).child(ui::muted(theme, t_("servers.serverAddress"), 12.5)));
    let mut addresses = vec![("addr-local", format_address("localhost", port), t_("servers.thisPc"))];
    if let Some(ip) = &server.lan_ip {
        addresses.push(("addr-lan", format_address(ip, port), t_("servers.lan")));
    }
    for (key, addr, tag) in addresses {
        let icon = if copied == Some(addr.as_str()) { &icons::CHECK } else { &icons::COPY };
        let to_copy = addr.clone();
        chips = chips.child(chip(theme, key, &addr, tag, icon, t_("servers.copy")).on_press_tx(move |app: &mut App, tx| copy_chip(app, tx, to_copy.clone())));
    }
    chips
}

fn start(app: &mut App, inst_token: String) {
    let global = app.settings.ngrok_authtoken.trim().to_string();
    let token = if inst_token.is_empty() { global } else { inst_token };
    let server = &mut app.instance_page.server;
    if token.is_empty() {
        server.token = TextEdit::default();
        server.editing_token = true;
        return;
    }
    server.busy = true;
    server.error = None;
    let port = if server.port == 0 { DEFAULT_PORT } else { server.port };
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::tunnel::tunnel_start(host.state(), token, port).await }, |app, result| {
        let server = &mut app.instance_page.server;
        server.busy = false;
        match result {
            Ok(addr) => server.tunnel = addr,
            Err(e) => server.error = Some(e.to_string()),
        }
    });
}

fn stop(app: &mut App) {
    let host = app.fx.host.clone();
    crate::tunnel::tunnel_stop(host.state());
    app.instance_page.server.tunnel.clear();
}

fn save_token(app: &mut App) {
    let token = app.instance_page.server.token.text.trim().to_string();
    if token.is_empty() {
        return;
    }
    let id = app.instance_page.id.clone();
    app.update_instance(&id, UpdateInstance { ngrok_authtoken: Some(token.clone()), ..Default::default() }, move |app, result| {
        if let Err(e) = result {
            app.toasts.error(e.to_string());
            return;
        }
        let server = &mut app.instance_page.server;
        server.editing_token = false;
        server.token = TextEdit::default();
        start(app, token);
    });
}

fn link(theme: &Theme, key: &str, label: String, on_press: impl Fn(&mut App) + 'static) -> El {
    el().key(key.to_string())
        .role("button")
        .label(label.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| on_press(app))
        .child(text(label).font(theme.sans(11.5, 400.0)).color(theme.accent))
}

/// `ServerShare.svelte`.
pub fn share(app: &App, theme: &Theme, inst: &Instance) -> El {
    let server = &app.instance_page.server;
    let inst_token = inst.ngrok_authtoken.as_deref().unwrap_or("").trim().to_string();
    let has_token = !inst_token.is_empty() || !app.settings.ngrok_authtoken.trim().is_empty();
    let mut panel = row()
        .key("server-share")
        .w_full()
        .flex_wrap()
        .gap(10.0)
        .center_y()
        .mt(10.0)
        .child(row().gap(6.0).center_y().child(ui::colored_icon(&icons::GLOBE, 13.0, theme.text_muted)).child(ui::muted(theme, t_("server.shareOnline"), 12.5)));

    if !server.tunnel.is_empty() {
        let addr = server.tunnel.clone();
        panel = panel
            .child(chip(theme, "tunnel-addr", &server.tunnel, t_("server.tagInternet"), &icons::COPY, t_("server.copy")).on_press_tx(move |app: &mut App, tx| {
                tx.copy(addr.clone());
                app.toasts.success("Copied.");
            }))
            .child(Btn::new(t_("server.stop")).ghost().sm().key("tunnel-stop").on_press(stop).build(theme))
            .child(ui::muted(theme, t_("server.shareWarn"), 11.5).wrap().w_full());
    } else if server.editing_token {
        let field = text_field(theme, &server.token, t_("server.tokenPlaceholder"), |app| &mut app.instance_page.server.token)
            .secure()
            .key("token-input")
            .on_submit(|app: &mut App, _| save_token(app));
        panel = panel
            .child(boxed(theme, &server.token, field).w(260.0))
            .child(Btn::new(t_("server.saveAndShare")).primary().sm().key("token-save").disabled(server.token.text.trim().is_empty()).on_press(save_token).build(theme))
            .child(link(theme, "token-get", t_("server.getOne"), |app| {
                if let Err(e) = open::that_detached(NGROK_AUTHTOKEN_URL) {
                    app.toasts.error(e.to_string());
                }
            }));
    } else {
        let token = inst_token.clone();
        panel = panel.child(
            Btn::new(if server.busy { t_("server.starting") } else { t_("server.shareViaNgrok") })
                .primary()
                .sm()
                .key("tunnel-start")
                .disabled(server.busy)
                .on_press(move |app| start(app, token.clone()))
                .build(theme),
        );
        if has_token {
            let key_source = format!("{} {}", t_("server.keyPrefix"), if inst_token.is_empty() { t_("nav.settings") } else { t_("server.keyThisServer") });
            let mut source_row = row().gap(6.0).center_y().child(ui::secondary(theme, key_source, 11.5));
            if !inst_token.is_empty() {
                source_row = source_row.child(link(theme, "token-clear", t_("server.useGlobal"), |app| {
                    let id = app.instance_page.id.clone();
                    app.update_instance(&id, UpdateInstance { ngrok_authtoken: Some(String::new()), ..Default::default() }, |app, result| {
                        if let Err(e) = result {
                            app.toasts.error(e.to_string());
                        }
                    });
                }));
            }
            panel = panel.child(source_row);
        } else {
            panel = panel.child(ui::muted(theme, t_("server.addKeyHint"), 11.5));
        }
    }
    if let Some(error) = &server.error {
        panel = panel.child(text(error.clone()).font(theme.sans(12.0, 400.0)).color(theme.danger).wrap().w_full());
    }
    panel
}
