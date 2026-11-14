//! `BoardView.svelte`, with `ServerWeather.svelte` and `Sparkline.svelte` for server boards.

use rune::tok::TextStyle;
use rune::{Color, Cursor, Cx, El, Shape, canvas, col, el, row, text};

use super::api::{self, Board, ServerSample};
use super::{card, is_following, record_import, refresh_then, set_following};
use crate::app::{App, Ignore};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::server_ping::ServerStatus;
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    handle: String,
    board: Option<Board>,
    loading: bool,
    error: Option<String>,
    importing_id: Option<String>,
    imported_ids: Vec<String>,
    weather: Weather,
}

/// ServerWeather's state.
#[derive(Default)]
struct Weather {
    address: String,
    status: Option<ServerStatus>,
    loading: bool,
    failed: bool,
    history: Vec<ServerSample>,
}

/// Mount for `handle`: fetch the board.
pub fn open(app: &mut App, handle: &str) {
    let state = &mut app.share.board;
    if state.loading && state.handle == handle {
        return;
    }
    *state = State { handle: handle.to_string(), ..State::default() };
    if !api::configured() {
        state.error = Some(tr("community.boardsServiceNotConfigured"));
        return;
    }
    state.loading = true;
    let requested = handle.to_string();
    app.fx.spawn(api::board(requested.clone()), move |app, result| {
        let state = &mut app.share.board;
        if state.handle != requested {
            return;
        }
        state.loading = false;
        match result {
            Ok(board) => {
                let addr = board.server_address.clone().filter(|addr| board.kind == "server" && !addr.is_empty());
                state.board = Some(board);
                if let Some(addr) = addr {
                    weather_open(app, addr);
                }
            }
            Err(e) => state.error = Some(e),
        }
    });
}

fn import_instance(app: &mut App, id: String) {
    if app.share.board.importing_id.is_some() {
        return;
    }
    let Some(handle) = app.share.board.board.as_ref().map(|board| board.handle.clone()) else { return };
    app.share.board.importing_id = Some(id.clone());
    let host = app.fx.host.clone();
    app.fx.spawn(api::import_snapshot(host, id.clone()), move |app, result| match result {
        Ok(outcome) => {
            record_import(app, &outcome.instance.id, Some(&handle), &id);
            app.share.board.imported_ids.push(id);
            refresh_then(app, |app| {
                app.share.board.importing_id = None;
                app.toasts.success(tr("community.importedToast")).ignore();
            });
        }
        Err(e) => {
            app.share.board.importing_id = None;
            app.toasts.error(e).ignore();
        }
    });
}

pub fn view(app: &App, cx: &mut Cx<App>, handle: &str) -> El {
    let theme = app.theme();
    let state = &app.share.board;
    if state.handle != handle || state.loading {
        return ui::muted(&theme, tr("common.loading"), 14.0);
    }
    if let Some(error) = &state.error {
        return text(error.clone()).font(theme.body()).color(theme.danger).wrap();
    }
    let Some(board) = &state.board else { return el() };

    let mut page = col().key("board-view").child(head(app, &theme, board));
    if board.kind == "streamer" {
        if let Some(url) = board.stream_url.clone().filter(|url| !url.is_empty()) {
            page = page.child(
                row().key("stream").w_full().center_y().gap(8.0).px(18.0).py(14.0).mb(14.0)
                    .border(2.0, color("#9147ff"))
                    .linear_gradient(std::f32::consts::FRAC_PI_2, &[(0.0, color("#9147ff").alpha(0.25)), (1.0, Color::TRANSPARENT)])
                    .role("button")
                    .label(tr("community.watchStream"))
                    .focusable(true)
                    .cursor(Cursor::Pointer)
                    .hover(Color::WHITE.alpha(0.05))
                    .on_press(move |_: &mut App| {
                        let _ = open::that_detached(&url);
                    })
                    .child(ui::colored_icon(&icons::VIDEO, 16.0, Color::WHITE))
                    .child(text(tr("community.watchStream")).font(theme.sans(14.0, 400.0)).color(Color::WHITE))
                    .child(el().grow(1.0))
                    .child(ui::muted(&theme, tr("community.opensInBrowser"), 11.0)),
            );
        }
    }
    if board.kind == "server" {
        if let Some(addr) = board.server_address.clone().filter(|addr| !addr.is_empty()) {
            page = page
                .child(
                    row().key("server").w_full().center_y().gap(8.0).px(18.0).py(14.0).mb(6.0).fill(theme.bg_input).border(2.0, theme.border)
                        .child(ui::colored_icon(&icons::GLOBE, 16.0, theme.text))
                        .child(text(addr).font(theme.pixel(14.0)).color(theme.accent).selectable(true)),
                )
                .child(
                    col().key("weather-strip").w_full().px(18.0).pt(8.0).pb(10.0).mb(14.0).fill(theme.bg_card)
                        .border_sides([0.0, 2.0, 2.0, 2.0], theme.border_subtle)
                        .child(weather(app, &theme)),
                );
        }
    }

    if !board.messages.is_empty() {
        let mut messages = col().key("messages").mb(18.0).child(ui::h3(&theme, tr("community.announcements")).mb(10.0));
        for msg in &board.messages {
            messages = messages.child(
                card(theme.bg_card, theme.border_subtle, 2.0, 12.0, 10.0).key(format!("msg-{}", msg.id)).mb(6.0)
                    .child(text(msg.body.clone()).font(theme.sans(13.0, 400.0)).color(theme.text).wrap())
                    .child(ui::muted(&theme, time_ago(&msg.created_at), 11.0)),
            );
        }
        page = page.child(messages);
    }

    let mut instances = col().key("insts").child(ui::h3(&theme, tr("community.instances")).mb(10.0));
    if board.instances.is_empty() {
        instances = instances.child(ui::muted(&theme, tr("community.noInstancesPublished"), 14.0));
    } else {
        // `repeat(auto-fill, minmax(240px, 1fr))`, gap 12.
        let w = cx.layout_size("board-grid").map_or(836.0, |[w, _]| w);
        let columns = (((w + 12.0) / 252.0).floor() as usize).max(1);
        let mut grid = col().key("board-grid").w_full().gap(12.0);
        for chunk in board.instances.chunks(columns) {
            let mut grid_row = row().gap(12.0).items_stretch();
            for inst in chunk {
                grid_row = grid_row.child(instance(app, &theme, inst).w(0.0).grow(1.0));
            }
            for _ in chunk.len()..columns {
                grid_row = grid_row.child(el().w(0.0).grow(1.0));
            }
            grid = grid.child(grid_row);
        }
        instances = instances.child(grid);
    }
    page.child(instances)
}

fn head(app: &App, theme: &Theme, board: &Board) -> El {
    let stripe = match board.kind.as_str() {
        "server" => color("#5b8a3a"),
        "creator" => color("#9b59d0"),
        _ => theme.accent,
    };
    let following = is_following(app, &board.handle);
    let mut main = col().grow(1.0).shrink().min_w(0.0)
        .child(text(board.kind.to_uppercase()).font(TextStyle { tracking: 10.0 * 0.06, ..theme.pixel(10.0) }).color(theme.text_muted))
        .child(ui::heading(theme, board.display_name.clone(), 22.0).my(4.0))
        .child(ui::muted(theme, format!("@{} · by {}", board.handle, board.owner_name), 12.5));
    if let Some(description) = board.description.clone().filter(|description| !description.is_empty()) {
        main = main.child(text(description).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).wrap().max_w(520.0).mt(8.0));
    }
    let handle = board.handle.clone();
    row().key("board-head").justify_between().items_start().gap(16.0).pl(24.0).pr(22.0).py(20.0).mb(14.0)
        .fill(theme.bg_card)
        .border_sides([2.0, 2.0, 2.0, 4.0], theme.border)
        .child(el().w(4.0).h_full().absolute(0.0, 0.0).fill(stripe))
        .child(main)
        .child(
            Btn::new(if following { tr("community.following") } else { tr("community.follow") })
                .ghost()
                .key("follow")
                .on_press(move |app| set_following(app, &handle, !following))
                .build(theme),
        )
}

fn instance(app: &App, theme: &Theme, inst: &api::BoardInstance) -> El {
    let state = &app.share.board;
    let importing = state.importing_id.as_deref() == Some(inst.id.as_str());
    let imported = state.imported_ids.contains(&inst.id);
    let mut body = col().gap(3.0)
        .child(text(inst.name.clone()).font(theme.sans(14.0, 600.0)).color(theme.text).wrap())
        .child(ui::muted(
            theme,
            format!("{} · {}", inst.mod_loader.as_deref().unwrap_or("vanilla"), inst.mc_version.as_deref().unwrap_or("?")),
            12.0,
        ));
    if let Some(changelog) = inst.changelog.clone().filter(|changelog| !changelog.is_empty()) {
        body = body.child(
            text(format!("“{changelog}”")).font(TextStyle { italic: true, ..theme.sans(12.5, 400.0) }).color(theme.text_secondary).wrap().mt(4.0),
        );
    }
    body = body.child(ui::muted(theme, format!("{} {}", tr("community.updated"), time_ago(&inst.created_at)), 11.0));
    let button = if importing {
        Btn::new(tr("common.importing")).disabled(true)
    } else if imported {
        Btn::new(tr("community.importedLabel")).icon(&icons::CHECK)
    } else {
        Btn::new(tr("common.import")).icon(&icons::DOWNLOAD)
    };
    let id = inst.id.clone();
    card(theme.bg_card, theme.border, 2.0, 14.0, 14.0).key(format!("inst-{}", inst.id)).gap(10.0)
        .child(body)
        .child(el().self_start().child(button.primary().sm().key("import").on_press(move |app| import_instance(app, id.clone())).build(theme)))
}

/// `timeAgo` of `$lib/time` for an ISO string.
fn time_ago(iso: &str) -> String {
    let at = chrono::DateTime::parse_from_rfc3339(iso).ok().map(|date| date.with_timezone(&chrono::Utc));
    ui::fmt::time_ago(at, "never")
}

fn color(hex: &str) -> Color {
    crate::theme::parse_color(hex).unwrap_or(Color::BLACK)
}

// ServerWeather

fn weather_open(app: &mut App, address: String) {
    app.share.board.weather = Weather { address: address.clone(), ..Weather::default() };
    weather_refresh(app);
    app.fx.spawn(api::server_history(address.clone(), 24), move |app, result| {
        let weather = &mut app.share.board.weather;
        if weather.address == address {
            weather.history = result.unwrap_or_default();
        }
    });
}

fn weather_refresh(app: &mut App) {
    let weather = &mut app.share.board.weather;
    if weather.address.is_empty() {
        return;
    }
    weather.loading = true;
    weather.failed = false;
    let address = weather.address.clone();
    app.fx.spawn(crate::server_ping::ping_server(address.clone()), move |app, result| {
        let weather = &mut app.share.board.weather;
        if weather.address != address {
            return;
        }
        weather.loading = false;
        match result {
            Ok(status) => weather.status = Some(status),
            Err(_) => {
                weather.status = None;
                weather.failed = true;
            }
        }
    });
}

fn weather(app: &App, theme: &Theme) -> El {
    let weather = &app.share.board.weather;
    let dot = |fill: Color| el().size(9.0, 9.0).radius(4.5).fill(fill);
    let mut line = row().key("weather").center_y().gap(8.0);
    if weather.loading && weather.status.is_none() {
        line = line
            .child(dot(theme.accent).looping(rune_motion::LoopSpec::opacity(1.0, 0.0, 0.3, 1.0)))
            .child(ui::secondary(theme, tr("servers.checking"), 13.0));
    } else if let Some(status) = &weather.status {
        line = line
            .child(dot(color("#5b8a3a")).box_shadow(rune::Shadow { x: 0.0, y: 0.0, blur: 0.0, spread: 3.0, color: color("#5b8a3a").alpha(0.2), inset: false }))
            .child(row()
                .child(text(status.online.to_string()).font(theme.pixel(13.0)).color(theme.text))
                .child(text("/").font(theme.pixel(13.0)).color(theme.text_muted).mx(1.0))
                .child(text(status.max.to_string()).font(theme.pixel(13.0)).color(theme.text)))
            .child(ui::secondary(theme, tr("servers.onlineLabel"), 13.0));
        if !status.players.is_empty() {
            let more = if status.players.len() > 3 { "…" } else { "" };
            line = line.child(
                ui::muted(theme, format!("· {}{more}", status.players.iter().take(3).cloned().collect::<Vec<_>>().join(", ")), 13.0)
                    .truncate()
                    .max_w(300.0)
                    .help(status.players.join(", ")),
            );
        }
        line = line.child(el().grow(1.0)).child(ui::muted(theme, tf("servers.pingMs", &[("ms", &status.ping_ms.to_string())]), 11.0));
    } else if weather.failed {
        line = line.child(dot(theme.text_muted)).child(ui::secondary(theme, tr("servers.offline"), 13.0));
    }
    let mut refresh = el().key("weather-refresh").px(4.0).py(2.0).role("button").label(tr("servers.refresh")).help(tr("servers.refresh"))
        .child(text("↻").font(theme.sans(13.0, 400.0)).color(theme.text_muted));
    refresh = if weather.loading { refresh.opacity(0.4).disabled(true) } else { refresh.cursor(Cursor::Pointer).focusable(true).on_press(weather_refresh) };
    line = line.child(refresh);
    if let Some(motd) = weather.status.as_ref().map(|status| status.motd.clone()).filter(|motd| !motd.is_empty()) {
        line = line.help(motd);
    }

    let mut strip = col().child(line);
    if weather.history.len() > 1 {
        let values: Vec<Option<f64>> = weather.history.iter().map(|sample| sample.online).collect();
        let peak = weather.history.iter().map(|sample| sample.online.unwrap_or(0.0)).fold(0.0, f64::max);
        strip = strip.child(
            col().key("history").mt(8.0)
                .child(sparkline(theme, &values, 200.0, 40.0))
                .child(row().justify_between().mt(2.0)
                    .child(ui::muted(theme, tr("servers.last24h"), 10.5))
                    .child(ui::muted(theme, tf("servers.peak", &[("peak", &peak.to_string())]), 10.5))),
        );
    }
    strip
}

/// `Sparkline.svelte`: an accent polyline over `values` in a `width`×`height` view box stretched
/// to the full width (aspect kept, as the SVG's `height: auto`); `None` breaks the line.
fn sparkline(theme: &Theme, values: &[Option<f64>], width: f32, height: f32) -> El {
    let max = values.iter().map(|value| value.unwrap_or(0.0)).fold(1.0, f64::max) as f32;
    let n = values.len();
    let mut shape = Shape::new().view_box(width, height);
    let mut run: Vec<[f32; 2]> = Vec::new();
    for (i, value) in values.iter().enumerate() {
        let x = if n > 1 { i as f32 / (n - 1) as f32 * width } else { width / 2.0 };
        match value {
            Some(value) => run.push([x, height - 2.0 - (*value as f32 / max) * (height - 4.0)]),
            None => {
                if run.len() > 1 {
                    shape = shape.stroke(std::mem::take(&mut run), 1.5, theme.accent);
                }
                run.clear();
            }
        }
    }
    if run.len() > 1 {
        shape = shape.stroke(run, 1.5, theme.accent);
    }
    canvas(shape).key("spark").w_full().aspect_ratio(width / height)
}
