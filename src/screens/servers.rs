//! Servers: `src/routes/servers/+page.svelte`, with `lib/servers.ts` and `stores/servers.svelte.ts`.

use std::collections::HashMap;

use rune::{Color, Cursor, Cx, El, ScrollSpec, TextEdit, Tx, col, el, row, text};
use rune_kit::{Menu, MenuItem, Select};
use serde::{Deserialize, Serialize};

use crate::app::{App, Ignore};
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::instance::InstanceKind;
use crate::server_browse::{BrowseParams, BrowseServer};
use crate::server_ping::ServerStatus;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const KEY: &str = "cactus:servers";

/// `FeaturedServer` in lib/servers.ts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

fn featured() -> Vec<Server> {
    let server = |name: &str, address: &str, description: &str, tags: [&str; 2], website: &str| Server {
        name: name.into(),
        address: address.into(),
        description: description.into(),
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
        website: Some(website.into()),
    };
    vec![
        server("Hypixel", "mc.hypixel.net", "The largest minigame network — Bed Wars, SkyBlock, and more.", ["Minigames", "SkyBlock"], "https://hypixel.net"),
        server("Wynncraft", "play.wynncraft.com", "A long-running open-world MMORPG built inside Minecraft.", ["MMORPG", "Adventure"], "https://wynncraft.com"),
        server("Complex Gaming", "hub.mc-complex.com", "Multi-gamemode network best known for its Pixelmon servers.", ["Pixelmon", "Survival"], "https://www.mc-complex.com"),
        server("ManaCube", "play.manacube.net", "Parkour, Islands (SkyBlock), Survival and seasonal events.", ["Parkour", "SkyBlock"], "https://manacube.com"),
        server("Purple Prison", "purpleprison.net", "One of the most popular Minecraft prison servers.", ["Prison", "Economy"], "https://purpleprison.com"),
    ]
}

pub enum Ping {
    Loading,
    Online(ServerStatus),
    Offline,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Mine,
    Discover,
}

const SORTS: [(&str, &str); 3] = [("players", "servers.sortPlayers"), ("votes", "servers.sortVotes"), ("rating", "servers.sortRating")];

#[derive(Default)]
pub struct State {
    /// The store's list; `None` until first read from prefs.
    servers: Option<Vec<Server>>,
    pings: HashMap<String, Ping>,
    show_add: bool,
    new_name: TextEdit,
    new_address: TextEdit,
    view: View,
    /// The card under the pointer (its remove button shows).
    hover: Option<String>,
    /// (name, address) of the server whose "add to instance" menu is open.
    add_menu: Option<(String, String)>,

    discover_query: TextEdit,
    discover_debounced: String,
    discover_typed: u64,
    discover_sort: Option<&'static str>,
    sort_open: bool,
    discover_hits: Vec<BrowseServer>,
    discover_page: u32,
    discover_has_more: bool,
    discover_loading: bool,
    discover_loading_more: bool,
    discover_error: Option<String>,
    /// Bumped per search so a slow response can't overwrite a newer one.
    discover_gen: u64,
}

impl State {
    fn sort(&self) -> &'static str {
        self.discover_sort.unwrap_or("players")
    }
}

// The store.

fn normalize(address: &str) -> String {
    address.trim().to_lowercase()
}

fn list(app: &App) -> &[Server] {
    app.servers.servers.as_deref().unwrap_or(&[])
}

fn persist(app: &mut App) {
    let list = app.servers.servers.clone().unwrap_or_default();
    app.prefs.write(KEY, list);
}

fn store_add(app: &mut App, server: Server) -> bool {
    let address = server.address.trim().to_string();
    let servers = app.servers.servers.get_or_insert_with(Vec::new);
    if address.is_empty() || servers.iter().any(|existing| normalize(&existing.address) == normalize(&address)) {
        return false;
    }
    let name = if server.name.trim().is_empty() { address.clone() } else { server.name.trim().to_string() };
    servers.push(Server { name, address, ..server });
    persist(app);
    true
}

fn store_remove(app: &mut App, address: &str) {
    let normalized = normalize(address);
    if let Some(list) = &mut app.servers.servers {
        list.retain(|existing| normalize(&existing.address) != normalized);
    }
    app.servers.pings.remove(address);
    persist(app);
}

fn store_reset(app: &mut App) {
    app.servers.servers = Some(featured());
    persist(app);
    refresh(app);
}

// Actions.

fn strip_codes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '§' {
            chars.next();
        } else {
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn display_name(srv: &BrowseServer) -> String {
    let name = strip_codes(&srv.name);
    if name.is_empty() { srv.address.clone() } else { name }
}

fn ping_one(app: &mut App, address: &str) {
    app.servers.pings.insert(address.to_string(), Ping::Loading);
    let addr = address.to_string();
    let target = addr.clone();
    app.fx.spawn(async move { crate::server_ping::ping_server(target).await }, move |app, result| {
        // A removed server's late reply is dropped.
        if app.servers.pings.contains_key(&addr) {
            app.servers.pings.insert(addr, match result {
                Ok(status) => Ping::Online(status),
                Err(_) => Ping::Offline,
            });
        }
    });
}

fn refresh(app: &mut App) {
    let addresses: Vec<String> = list(app).iter().map(|server| server.address.clone()).collect();
    for address in addresses {
        ping_one(app, &address);
    }
}

fn add_server(app: &mut App) {
    let address = app.servers.new_address.text.trim().chars().take(120).collect::<String>();
    if address.is_empty() {
        return;
    }
    let name: String = app.servers.new_name.text.trim().chars().take(40).collect();
    let name = if name.is_empty() { address.clone() } else { name };
    if !store_add(app, Server { name, address: address.clone(), description: String::new(), tags: Vec::new(), website: None }) {
        app.toasts.error(tr("servers.alreadyInList")).ignore();
        return;
    }
    ping_one(app, &address);
    app.servers.new_name = TextEdit::default();
    app.servers.new_address = TextEdit::default();
    app.servers.show_add = false;
}

fn discover_search(app: &mut App) {
    let state = &mut app.servers;
    state.discover_loading = true;
    state.discover_error = None;
    state.discover_page = 1;
    state.discover_gen += 1;
    let generation = state.discover_gen;
    let params = BrowseParams { query: state.discover_debounced.clone(), sort: Some(state.sort().into()), page: 1 };
    app.fx.spawn(async move { crate::server_browse::browse_servers(params).await }, move |app, result| {
        let state = &mut app.servers;
        if state.discover_gen != generation {
            return;
        }
        match result {
            Ok(page) => {
                state.discover_hits = page.servers;
                state.discover_has_more = page.has_more;
            }
            Err(e) => {
                state.discover_error = Some(e.to_string());
                state.discover_hits.clear();
            }
        }
        state.discover_loading = false;
    });
}

fn discover_more(app: &mut App) {
    let state = &mut app.servers;
    state.discover_loading_more = true;
    let generation = state.discover_gen;
    let next = state.discover_page + 1;
    let params = BrowseParams { query: state.discover_debounced.clone(), sort: Some(state.sort().into()), page: next };
    app.fx.spawn(async move { crate::server_browse::browse_servers(params).await }, move |app, result| {
        let state = &mut app.servers;
        state.discover_loading_more = false;
        if state.discover_gen != generation {
            return;
        }
        match result {
            Ok(page) => {
                state.discover_hits.extend(page.servers);
                state.discover_page = next;
                state.discover_has_more = page.has_more;
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

/// The search field's 350 ms debounce.
fn on_query_change(app: &mut App) {
    app.servers.discover_typed += 1;
    let typed = app.servers.discover_typed;
    app.fx.after(std::time::Duration::from_millis(350), move |app| {
        let state = &mut app.servers;
        if state.discover_typed == typed && state.discover_debounced != state.discover_query.text {
            state.discover_debounced = state.discover_query.text.clone();
            if state.view == View::Discover {
                discover_search(app);
            }
        }
    });
}

fn set_view(app: &mut App, target: View) {
    if app.servers.view == target {
        return;
    }
    app.servers.view = target;
    app.servers.add_menu = None;
    if target == View::Discover {
        discover_search(app);
    }
}

fn add_from_discover(app: &mut App, i: usize) {
    let Some(srv) = app.servers.discover_hits.get(i) else { return };
    let name = display_name(srv);
    let server = Server {
        name: name.clone(),
        address: srv.address.clone(),
        description: strip_codes(&srv.description),
        tags: if srv.country.is_empty() { Vec::new() } else { vec![srv.country.clone()] },
        website: None,
    };
    let address = server.address.clone();
    if store_add(app, server) {
        app.toasts.success(tf("servers.addedToMine", &[("name", &name)])).ignore();
        ping_one(app, &address);
    } else {
        app.toasts.error(tr("servers.alreadyInList")).ignore();
    }
}

fn client_instances(app: &App) -> Vec<&crate::instance::Instance> {
    app.instances.iter().filter(|inst| inst.kind == InstanceKind::Client).collect()
}

fn on_add_to(app: &mut App, name: String, address: String) {
    if client_instances(app).is_empty() {
        app.toasts.error(tr("servers.createInstanceFirst")).ignore();
        return;
    }
    if app.servers.add_menu.as_ref().is_some_and(|(_, open_address)| *open_address == address) {
        app.servers.add_menu = None;
        return;
    }
    app.servers.add_menu = Some((name, address));
}

fn add_to_instance(app: &mut App, instance_id: String) {
    let Some((name, address)) = app.servers.add_menu.take() else { return };
    let inst_name = app.instance(&instance_id).map_or_else(|| tr("servers.theInstance"), |inst| inst.name.clone());
    app.fx.blocking(
        move |host| crate::commands::add_server_to_instance(host.clone(), instance_id, name.clone(), address).map(|_| name),
        move |app, result| match result {
            Ok(name) => app.toasts.success(tf("servers.addedToInstance", &[("server", &name), ("instance", &inst_name)])).ignore(),
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        },
    );
}

fn copy(app: &mut App, tx: &mut Tx, address: &str) {
    tx.copy(address.to_string());
    app.toasts.success(tf("servers.copiedAddress", &[("address", address)])).ignore();
}

pub fn on_enter(app: &mut App) {
    if app.servers.servers.is_none() {
        app.servers.servers = Some(app.prefs.read::<Option<Vec<Server>>>(KEY, None).unwrap_or_else(featured));
    }
    app.servers.pings.clear();
    app.servers.add_menu = None;
    refresh(app);
    if app.servers.view == View::Discover {
        discover_search(app);
    }
}

pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

// View.

/// `toLocaleString()` for counts: 12,345.
fn grouped(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// `grid-template-columns: repeat(auto-fill, minmax(min, 1fr))` with equal-height rows.
fn grid(width: f32, min: f32, gap: f32, items: Vec<El>) -> El {
    let columns = (((width + gap) / (min + gap)).floor() as usize).max(1);
    let cell = ((width - gap * (columns as f32 - 1.0)) / columns as f32).floor();
    let mut lines = col().w_full().gap(gap);
    let mut items = items.into_iter().peekable();
    let mut line_index = 0;
    while items.peek().is_some() {
        let mut line = row().key(format!("r{line_index}")).gap(gap).items_stretch();
        for item in items.by_ref().take(columns) {
            line = line.child(item.w(cell));
        }
        lines = lines.child(line);
        line_index += 1;
    }
    lines
}

fn tag(theme: &Theme, label: impl Into<String>) -> El {
    row().px(7.0).py(1.5).gap(4.0).center_y().radius(999.0).fill(theme.text.alpha(0.08))
        .child(text(label.into()).font(theme.sans(11.2, 400.0)).color(theme.text_muted).truncate())
}

fn status(theme: &Theme, online: Option<i64>) -> El {
    match online {
        Some(count) => text(format!("● {}", tf("servers.countOnline", &[("count", &grouped(count))]))).font(theme.sans(11.5, 400.0)).color(theme.success),
        None => text(format!("● {}", tr("servers.offline"))).font(theme.sans(11.5, 400.0)).color(theme.text_muted),
    }
}

fn icon_box(app: &App, theme: &Theme, favicon: Option<&str>) -> El {
    let frame = el().size(28.0, 28.0).center().radius(6.0).clip_rounded(6.0).fill(theme.text.alpha(0.08));
    match favicon {
        Some(src) => frame.child(ui::image(app, src, 28.0, 28.0)),
        None => frame.child(ui::colored_icon(&icons::GLOBE, 18.0, theme.text_muted)),
    }
}

fn card(theme: &Theme, key: String) -> El {
    col().key(key).gap(8.8).pt(14.4).pb(14.4).px(16.0).fill(theme.bg_card).border(1.0, theme.border).radius(4.0)
}

fn card_head(theme: &Theme, icon: El, name: String, status: El) -> El {
    row().gap(8.8).center_y().pr(19.2)
        .child(icon)
        .child(text(name).font(theme.pixel(16.3)).color(theme.text).truncate().grow(1.0).shrink().min_w(0.0))
        .child(status)
}

fn desc(theme: &Theme, description: String) -> El {
    text(description).font(rune::tok::TextStyle { line_height: 13.6 * 1.45, ..theme.sans(13.6, 400.0) }).color(theme.text_secondary).max_lines(3)
}

fn addr_row(theme: &Theme, address: &str, actions: El) -> El {
    // `margin-top: auto` pins the row to the card's bottom.
    col().grow(1.0).justify_end().child(
        row().gap(8.0).center_y().justify_between()
            .child(text(address.to_string()).font(theme.mono(12.8)).color(theme.text).truncate().shrink().min_w(0.0))
            .child(actions.gap(5.6)),
    )
}

/// The "add to an instance" button and, while it's open, its menu of client instances.
fn add_to_button(app: &App, layer: &mut Vec<El>, key: &str, icon: &'static rune::Icon, name: String, address: String) -> El {
    let theme = app.theme();
    let open = app.servers.add_menu.as_ref().is_some_and(|(_, open_address)| *open_address == address);
    let btn = Btn::icon_only(icon).ghost().sm().key(key.to_string()).help(tr("servers.addToInstanceList"))
        .on_press(move |app| on_add_to(app, name.clone(), address.clone()))
        .build(&theme).label(tr("servers.addToInstanceList"));
    if !open {
        return btn;
    }
    let mut menu = Menu::new(format!("{key}-menu")).trigger(btn).open(true).on_close(|app: &mut App| app.servers.add_menu = None);
    // The kit truncates labels to the menu's width; the Svelte menu grows to fit them (min 176).
    let labels: Vec<String> = client_instances(app).iter().map(|inst| format!("{} · {}", inst.name, inst.mc_version)).collect();
    let widest = labels.iter().map(|label| label.chars().count()).max().unwrap_or(0) as f32;
    menu = menu.min_width((widest * 7.4 + 56.0).clamp(176.0, 420.0));
    for (inst, label) in client_instances(app).into_iter().zip(labels) {
        let id = inst.id.clone();
        menu = menu.item(
            MenuItem::new(label)
                .icon(&icons::PLUS)
                .on_select(move |app: &mut App| add_to_instance(app, id.clone())),
        );
    }
    menu.build(&theme.kit(), layer)
}

fn mine_card(app: &App, layer: &mut Vec<El>, server: &Server) -> El {
    let theme = app.theme();
    let ping = app.servers.pings.get(&server.address);
    let loading = matches!(ping, None | Some(Ping::Loading));
    let addr = server.address.clone();

    let icon = match ping {
        None | Some(Ping::Loading) => el().size(28.0, 28.0).child(ui::skeleton(&theme, Some(28.0), 28.0).radius(6.0)),
        Some(Ping::Online(reply)) => icon_box(app, &theme, reply.favicon.as_deref()),
        Some(Ping::Offline) => icon_box(app, &theme, None),
    };
    let status_el = if loading {
        ui::skeleton(&theme, Some(66.0), 12.0)
    } else {
        status(&theme, match ping {
            Some(Ping::Online(reply)) => Some(reply.online),
            _ => None,
        })
    };
    let hovered = app.servers.hover.as_deref() == Some(addr.as_str());
    let remove = el()
        .key("remove")
        .size(20.0, 20.0)
        .center()
        .radius(6.0)
        .role("button")
        .label(tf("servers.removeServerAria", &[("name", &server.name)]))
        .help(tr("common.remove"))
        .cursor(Cursor::Pointer)
        .hover(theme.danger.alpha(0.15))
        .opacity(if hovered { 1.0 } else { 0.0 })
        .on_press({
            let addr = addr.clone();
            move |app: &mut App| store_remove(app, &addr)
        })
        .child(text("✕").font(theme.sans(12.8, 400.0)).color(theme.text_muted));

    let mut tile = card(&theme, format!("srv-{addr}"))
        .on_hover_change({
            let addr = addr.clone();
            move |app: &mut App, inside, _| {
                if inside {
                    app.servers.hover = Some(addr.clone());
                } else if app.servers.hover.as_deref() == Some(addr.as_str()) {
                    app.servers.hover = None;
                }
            }
        })
        .child(card_head(&theme, icon, server.name.clone(), status_el));
    if !server.description.is_empty() {
        tile = tile.child(desc(&theme, server.description.clone()));
    }
    if !server.tags.is_empty() {
        tile = tile.child(row().gap(4.8).flex_wrap().children(server.tags.iter().map(|label| tag(&theme, label.clone()))));
    }
    let mut actions = row()
        .child(
            Btn::new(tr("servers.copy")).icon(&icons::COPY).sm().key("copy").help(tr("servers.copyAddress"))
                .on_press_tx({
                    let addr = addr.clone();
                    move |app, tx| copy(app, tx, &addr)
                })
                .build(&theme),
        )
        .child(add_to_button(app, layer, &format!("addto-{addr}"), &icons::PLUS, server.name.clone(), addr.clone()));
    if let Some(site) = server.website.clone() {
        actions = actions.child(
            Btn::icon_only(&icons::GLOBE).ghost().sm().key("site").help(tr("servers.openWebsite"))
                .on_press(move |_| {
                    let _ = open::that(&site);
                })
                .build(&theme).label(tr("servers.openWebsite")),
        );
    }
    tile = tile.child(addr_row(&theme, &addr, actions));
    // The remove button sits in the card's top-right corner (top: .4rem; right: .4rem).
    el().key(format!("cell-{addr}")).child(tile.w_full().h_full()).child(
        row().absolute(0.0, 6.4).w_full().justify_end().pr(6.4).child(remove),
    )
}

fn discover_card(app: &App, layer: &mut Vec<El>, i: usize, srv: &BrowseServer) -> El {
    let theme = app.theme();
    let name = display_name(srv);
    let addr = srv.address.clone();
    let mut tile = card(&theme, format!("d-{addr}"))
        .child(card_head(&theme, icon_box(app, &theme, srv.favicon.as_deref()), name.clone(), status(&theme, srv.online.then_some(srv.players as i64))));
    let description = strip_codes(&srv.description);
    if !description.is_empty() {
        tile = tile.child(desc(&theme, description));
    }
    let mut tags = row().gap(4.8).flex_wrap();
    if !srv.version.is_empty() {
        tags = tags.child(tag(&theme, strip_codes(&srv.version)).max_w(180.0));
    }
    if !srv.country.is_empty() {
        tags = tags.child(tag(&theme, srv.country.clone()));
    }
    if srv.votes > 0 {
        tags = tags.child(
            row().px(7.0).py(1.5).gap(4.0).center_y().radius(999.0).fill(theme.text.alpha(0.08))
                .child(ui::colored_icon(&icons::HEART, 10.0, theme.text_muted))
                .child(text(ui::fmt::count(srv.votes)).font(theme.sans(11.2, 400.0)).color(theme.text_muted)),
        );
    }
    tile = tile.child(tags);
    let actions = row()
        .child(
            Btn::icon_only(&icons::COPY).sm().key("copy").help(tr("servers.copyAddress"))
                .on_press_tx({
                    let addr = addr.clone();
                    move |app, tx| copy(app, tx, &addr)
                })
                .build(&theme).label(tr("servers.copyAddress")),
        )
        .child(add_to_button(app, layer, &format!("addto-{addr}"), &icons::CUBE, name, addr.clone()))
        .child(
            Btn::new(tr("common.add")).icon(&icons::PLUS).primary().sm().key("add").help(tr("servers.addToMine"))
                .on_press(move |app| add_from_discover(app, i))
                .build(&theme),
        );
    tile.child(addr_row(&theme, &addr, actions))
}

fn view_tabs(theme: &Theme, view: View) -> El {
    let tab_button = |key: &str, label: String, icon: Option<&'static rune::Icon>, target: View| {
        let active = view == target;
        let ink = if active { theme.accent_contrast } else { theme.text_secondary };
        let mut tab = row().key(key.to_string()).gap(6.0).center_y().px(16.0).py(7.0)
            .role("tab").checked(active).focusable(true).cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| set_view(app, target));
        if active {
            tab = tab.fill(theme.accent);
        }
        if let Some(icon) = icon {
            tab = tab.child(ui::colored_icon(icon, 14.0, ink));
        }
        tab.child(text(label).font(theme.sans(13.6, 600.0)).color(ink))
    };
    row().key("seg").gap(2.0).padding(3.0).fill(theme.bg_input).border(2.0, theme.border).mb(17.6).self_start()
        .child(tab_button("seg-mine", tr("servers.myServers"), None, View::Mine))
        .child(tab_button("seg-discover", tr("servers.discover"), Some(&icons::COMPASS), View::Discover))
}

fn field(theme: &Theme, edit: &TextEdit, placeholder: String, access: impl for<'a> Fn(&'a mut App) -> &'a mut TextEdit + 'static) -> El {
    let input = rune::text_input(edit, theme.sans(13.6, 400.0))
        .single_line()
        .placeholder(placeholder, theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(access)
        .on_submit(|app: &mut App, _| add_server(app))
        .grow(1.0);
    row().grow(1.0).min_w(160.0).center_y().px(11.2).py(8.0).fill(theme.bg_input).radius(4.0)
        .border(1.0, if edit.focused { theme.accent } else { theme.border })
        .child(input)
}

fn link(theme: &Theme, key: &str, label: String, color: Color, on_press: impl Fn(&mut App) + 'static) -> El {
    col().key(key.to_string()).role("link").focusable(true).cursor(Cursor::Pointer).on_press(move |app: &mut App| on_press(app))
        .child(text(label).font(theme.sans(13.6, 400.0)).color(color))
        .child(el().h(1.0).w_full().fill(color))
}

fn mine(app: &App, width: f32, layer: &mut Vec<El>, body: El) -> El {
    let theme = app.theme();
    let mut body = body;
    if app.servers.show_add {
        body = body.child(
            row().key("add-form").gap(8.0).mb(16.0).flex_wrap().center_y()
                .child(field(&theme, &app.servers.new_name, tr("servers.namePlaceholder"), |app| &mut app.servers.new_name).key("new-name"))
                .child(field(&theme, &app.servers.new_address, tr("servers.addressPlaceholder"), |app| &mut app.servers.new_address).key("new-address"))
                .child(Btn::new(tr("common.add")).primary().sm().key("add-submit").on_press(add_server).build(&theme))
                .child(Btn::new(tr("common.cancel")).ghost().sm().key("add-cancel").on_press(|app| app.servers.show_add = false).build(&theme)),
        );
    }
    let servers = list(app);
    if servers.is_empty() {
        body = body.child(
            row().key("empty").center_y().mb(14.4)
                .child(text(tr("servers.emptyPrefix")).font(theme.sans(14.4, 400.0)).color(theme.text_muted))
                .child(link(&theme, "restore", tr("servers.restoreDefaults"), theme.accent, store_reset))
                .child(text(".").font(theme.sans(14.4, 400.0)).color(theme.text_muted)),
        );
    }
    let cards = servers.iter().map(|server| mine_card(app, layer, server)).collect();
    body.child(grid(width, 260.0, 14.4, cards))
}

fn discover(app: &App, width: f32, layer: &mut Vec<El>, body: El) -> El {
    let theme = app.theme();
    let state = &app.servers;
    let search = rune::text_input(&state.discover_query, theme.sans(13.0, 400.0))
        .single_line()
        .key("d-search")
        .placeholder(tr("servers.discoverSearch"), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(|app: &mut App| &mut app.servers.discover_query)
        .on_input_change(|app: &mut App, _| on_query_change(app))
        .grow(1.0);
    let search = ui::input_frame(&theme, state.discover_query.focused).min_h(0.0).py(9.0).gap(10.0).grow(1.0)
        .child(ui::colored_icon(&icons::SEARCH, 16.0, theme.text_muted))
        .child(search);
    let labels: Vec<String> = SORTS.iter().map(|(_, key)| tr(key)).collect();
    let current = SORTS.iter().position(|(value, _)| *value == state.sort()).map(|index| labels[index].clone());
    let sort = Select::new("d-sort", &labels)
        .width(180.0)
        .current(current.as_deref())
        .open(state.sort_open)
        .on_toggle(|app: &mut App| app.servers.sort_open = !app.servers.sort_open)
        .on_close(|app: &mut App| app.servers.sort_open = false)
        .on_choose(|app: &mut App, label: &str, _: &mut Tx| {
            app.servers.sort_open = false;
            if let Some((value, _)) = SORTS.iter().find(|(_, key)| tr(key) == label) {
                if app.servers.sort() != *value {
                    app.servers.discover_sort = Some(value);
                    discover_search(app);
                }
            }
        })
        .build(&theme.kit(), layer);
    let mut body = body.child(row().key("d-toolbar").gap(9.6).mb(16.0).center_y().child(search).child(sort));

    if state.discover_error.is_some() {
        body = body.child(
            col().key("d-error").items_center().gap(9.6).py(48.0).px(16.0)
                .child(text(tr("servers.discoverError")).font(theme.sans(14.4, 400.0)).color(theme.danger))
                .child(Btn::new(tr("common.retry")).ghost().sm().key("d-retry").on_press(discover_search).build(&theme)),
        );
    } else if state.discover_loading {
        let cards = (0..8).map(|i| card(&theme, format!("sk{i}")).child(ui::skeleton(&theme, None, 96.0))).collect();
        body = body.child(grid(width, 260.0, 14.4, cards));
    } else if state.discover_hits.is_empty() {
        body = body.child(
            el().key("d-empty").w_full().center().py(48.0).px(16.0)
                .child(text(tr("servers.discoverEmpty")).font(theme.sans(14.4, 400.0)).color(theme.text_muted)),
        );
    } else {
        let cards = state.discover_hits.iter().enumerate().map(|(i, server)| discover_card(app, layer, i, server)).collect();
        body = body.child(grid(width, 260.0, 14.4, cards));
        if state.discover_has_more {
            let label = if state.discover_loading_more { tr("common.loading") } else { tr("servers.loadMore") };
            body = body.child(
                row().w_full().justify_center().mt(24.0)
                    .child(Btn::new(label).ghost().key("d-more").disabled(state.discover_loading_more).on_press(discover_more).build(&theme)),
            );
        }
    }
    body.child(
        row().key("powered").w_full().justify_center().gap(4.0).mt(24.0).center_y()
            .child(text(tr("servers.poweredBy")).font(theme.sans(11.5, 400.0)).color(theme.text_muted))
            .child(link(&theme, "powered-link", "minecraft-list.info".into(), theme.text_secondary, |_| {
                let _ = open::that("https://minecraft-list.info");
            })),
    )
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let width = cx.layout_size("servers-body").map_or(852.0, |[w, _]| w);

    let mut head = row().key("head").justify_between().gap(16.0).mb(16.0).w_full().child(
        col().shrink().min_w(0.0)
            .child(ui::heading(&theme, tr("nav.servers"), 24.0))
            .child(text(tr("servers.subtitle")).font(theme.sans(14.4, 400.0)).color(theme.text_muted).wrap().mt(4.0)),
    );
    if app.servers.view == View::Mine {
        head = head.child(
            row().gap(6.4).items_start()
                .child(Btn::new(tr("servers.reset")).ghost().sm().key("reset").help(tr("servers.restoreDefaultList")).on_press(store_reset).build(&theme))
                .child(Btn::new(tr("servers.refresh")).icon(&icons::REFRESH).ghost().sm().key("refresh").help(tr("servers.refreshStatus")).on_press(refresh).build(&theme))
                .child(Btn::new(tr("servers.addServer")).icon(&icons::PLUS).primary().sm().key("add-toggle")
                    .on_press(|app| app.servers.show_add = !app.servers.show_add).build(&theme)),
        );
    }

    let body = col().key("servers-body").w_full().child(head).child(view_tabs(&theme, app.servers.view));
    let body = match app.servers.view {
        View::Mine => mine(app, width, layer, body),
        View::Discover => discover(app, width, layer, body),
    };
    let body = body.child(
        text(tr("servers.disclaimer")).font(rune::tok::TextStyle { line_height: 10.9 * 1.5, ..theme.sans(10.9, 400.0) })
            .color(theme.text_muted).wrap().align(rune::Align::Center).mt(32.0).w_full(),
    );

    col().key("servers").w_full().h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).items_center()
        .child(col().w_full().max_w(900.0).pt(32.0).px(24.0).pb(48.0).child(body))
}
