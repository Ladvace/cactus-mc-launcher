//! `Dock.svelte`: navigation, pinned instances and folders, downloads, new instance, settings and
//! the account, magnifying under the pointer. Sits on any window edge (`settings.dock_position`).

use rune::{Color, Cursor, Cx, El, PointerEvent, Spring, Tx, col, el, row, text};

use crate::app::{App, Route, to_percent};
use crate::core::i18n::{t, tf};
use crate::icons;
use crate::instance::{Instance, InstanceKind};
use crate::theme::{DECOR_PEEKS, Theme};
use crate::ui::{self, colored_icon};

#[derive(Default)]
pub struct State {
    /// The pointer along the dock's main axis, in dock points, while it's inside.
    pointer: Option<f32>,
    overflow_open: bool,
}

const ITEM: f32 = 48.0;
const SEP: f32 = 6.0;
const GAP: f32 = 8.0;
const PAD: f32 = 14.0;
const MAX: f32 = 1.28;
const RANGE: f32 = 100.0;

enum Entry<'a> {
    Instance(&'a Instance),
    Folder(String, Vec<&'a Instance>),
}

enum Item<'a> {
    Nav(Route, &'static rune::Icon, String),
    Sep,
    Entry(Entry<'a>),
    Overflow(usize),
    Downloads { pct: Option<u32>, target: Option<String>, label: String },
    Add,
    Settings,
    Account,
}

impl Item<'_> {
    fn width(&self) -> f32 {
        if matches!(self, Item::Sep) { SEP } else { ITEM }
    }
}

fn entries(app: &App) -> Vec<Entry<'_>> {
    let mut list = Vec::new();
    let mut groups: std::collections::BTreeMap<String, Vec<&Instance>> = Default::default();
    for inst in &app.instances {
        match &inst.group {
            Some(g) if !g.is_empty() => groups.entry(g.clone()).or_default().push(inst),
            _ => list.push(Entry::Instance(inst)),
        }
    }
    list.extend(groups.into_iter().map(|(name, insts)| Entry::Folder(name, insts)));
    list
}

fn entry_has(entry: &Entry, id: &str) -> bool {
    match entry {
        Entry::Instance(i) => i.id == id,
        Entry::Folder(_, insts) => insts.iter().any(|i| i.id == id),
    }
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let pos = app.settings.dock_position.as_str();
    let horizontal = !matches!(pos, "left" | "right");
    let magnify = app.settings.dock_magnify && !app.settings.reduce_motion;

    let all = entries(app);
    let cap = if horizontal {
        7
    } else {
        let h = cx.layout_size("content").map_or(760.0, |[_, h]| h);
        let slots = (((h - 52.0 - (6.0 * 56.0 + 28.0)) / 56.0).floor() as i32).clamp(1, 7) as usize;
        if all.len() <= slots { slots } else { slots.saturating_sub(1).max(1) }
    };
    let overflow_count = all.len().saturating_sub(cap);
    let (pinned, rest) = if all.len() > cap { let mut head = all; let tail = head.split_off(cap); (head, tail) } else { (all, Vec::new()) };

    let unpinned_active: Vec<&String> = app.install.progress.keys().filter(|id| !pinned.iter().any(|entry| entry_has(entry, id))).collect();
    let dl_id = unpinned_active.first().map(|id| id.to_string());
    let show_downloads = app.install.starting > 0 || !unpinned_active.is_empty();
    let pending = app.install.pending.as_ref();
    let dl_pct = match &dl_id {
        Some(id) => app.install.percent(id),
        None => pending.and_then(|p| to_percent(p.current, p.total)),
    };
    let dl_label = match &dl_id {
        Some(id) => app.install.progress.get(id).map_or_else(|| app.install.overall_message(), |p| p.message.clone()),
        None => pending.map_or_else(|| app.install.overall_message(), |p| p.message.clone()),
    };

    let mut items = vec![
        Item::Nav(Route::Home, &icons::HOME, t("nav.home")),
        Item::Nav(Route::Browse, &icons::COMPASS, t("nav.browse")),
        Item::Nav(Route::Servers, &icons::GLOBE, t("nav.servers")),
        Item::Nav(Route::Community, &icons::USERS, t("nav.community")),
        Item::Sep,
    ];
    items.extend(pinned.into_iter().map(Item::Entry));
    if overflow_count > 0 {
        items.push(Item::Overflow(overflow_count));
    }
    if show_downloads {
        items.push(Item::Downloads { pct: dl_pct, target: dl_id, label: dl_label });
    }
    items.extend([Item::Add, Item::Sep, Item::Settings, Item::Account]);

    // Magnification: each item scales by its distance from the pointer along the dock.
    let mut centers = Vec::with_capacity(items.len());
    let mut at = PAD;
    for item in &items {
        centers.push(at + item.width() / 2.0);
        at += item.width() + GAP;
    }
    let scale_of = |i: usize| match (magnify, app.dock.pointer) {
        (true, Some(pointer)) => {
            let dist = (pointer - centers[i]).abs();
            if dist > RANGE { 1.0 } else { 1.0 + (MAX - 1.0) * (1.0 - dist / RANGE) }
        }
        _ => 1.0,
    };

    let mut dock = if horizontal { row().px(12.0).py(8.0) } else { col().px(8.0).py(12.0) };
    dock = dock
        .key("dock")
        .role("toolbar")
        .label(t("dock.ariaNav"))
        .gap(GAP)
        .fill(if theme.reduce_transparency { theme.bg_raised } else { theme.bg_raised.alpha(0.92) })
        .border(2.0, theme.border)
        .shadow(8.0, 30.0, Color::BLACK.alpha(0.5));
    dock = ui::bevel(dock, 2.0, 0.05, 0.3);
    dock = match pos {
        "top" => dock.items_start(),
        "left" => dock.items_start(),
        "right" => dock.items_end(),
        _ => dock.items_end(),
    };
    if !theme.reduce_transparency {
        dock = dock.backdrop_blur(8.0);
    }
    dock = dock
        .on_pointer_move_local(move |app: &mut App, p: PointerEvent, _tx: &mut Tx| {
            let along = if horizontal { p.x } else { p.y };
            if app.dock.pointer.is_none_or(|old| (old - along).abs() > 0.5) {
                app.dock.pointer = Some(along);
            }
        })
        .on_hover_change(|app: &mut App, inside, _tx: &mut Tx| {
            if !inside {
                app.dock.pointer = None;
            }
        });

    for (i, item) in items.iter().enumerate() {
        let child = match item {
            Item::Sep => {
                let sep = el().fill(theme.border);
                if horizontal { sep.w(2.0).self_stretch().my(4.0).mx(2.0) } else { sep.h(2.0).self_stretch().mx(4.0).my(2.0) }
            }
            _ => dock_item(app, &theme, i, item, scale_of(i), pos),
        };
        dock = dock.child(child);
    }

    if let Some(peek) = DECOR_PEEKS.iter().find(|(id, _)| *id == app.settings.decor_theme).map(|(_, p)| *p) {
        let img = ui::image(app, peek, 40.0, 40.0).a11y_hidden();
        dock = dock.child(match pos {
            "top" | "left" | "right" => img.absolute(10.0, -26.0),
            _ => img.absolute(at - 54.0 - 12.0, -26.0),
        });
    }

    if app.dock.overflow_open && !rest.is_empty() {
        layer.push(overflow_menu(app, &theme, &rest, pos));
    }

    place("dock-wrap", pos, dock, 14.0)
}

/// A full-window layer holding `child` centred on the `pos` edge, `inset` from it. The layer
/// itself has no handlers, so it isn'th hit and the page under it stays clickable.
fn place(key: &str, pos: &str, child: El, inset: f32) -> El {
    let wrap = match pos {
        "top" => col().items_center().pt(inset),
        "left" => row().items_center().pl(inset),
        "right" => row().items_center().justify_end().pr(inset),
        _ => col().items_center().justify_end().pb(inset),
    };
    wrap.key(key.to_string()).absolute(0.0, 0.0).w_full().h_full().child(child)
}

fn dock_item(app: &App, theme: &Theme, i: usize, item: &Item, scale: f32, pos: &str) -> El {
    let section = app.route.section();
    let (label, active, running, preparing) = match item {
        Item::Nav(route, _, nav_label) => (nav_label.clone(), route.section() == section, false, false),
        Item::Settings => (t("nav.settings"), section == "/settings", false, false),
        Item::Entry(Entry::Instance(inst)) => (
            inst.name.clone(),
            app.route == Route::Instance(inst.id.clone()),
            app.is_running(&inst.id),
            app.is_busy(&inst.id) && !app.is_running(&inst.id),
        ),
        Item::Entry(Entry::Folder(name, _)) => (name.clone(), false, false, false),
        Item::Overflow(n) => (tf("dock.moreOnHome", &[("count", &n.to_string())]), false, false, false),
        Item::Downloads { label, .. } => (label.clone(), false, false, false),
        Item::Add => (t("dock.newInstance"), false, false, false),
        Item::Account => (app.active_name(), false, false, false),
        Item::Sep => (String::new(), false, false, false),
    };

    let glyph: El = match item {
        Item::Nav(_, icon, _) => colored_icon(icon, 24.0, if active { theme.accent } else { theme.text_secondary }),
        Item::Settings => colored_icon(&icons::SETTINGS, 24.0, if active { theme.accent } else { theme.text_secondary }),
        Item::Add => colored_icon(&icons::PLUS, 24.0, theme.text_secondary),
        Item::Overflow(n) => text(format!("+{n}")).font(rune::tok::TextStyle { weight: 700.0, ..theme.pixel(15.0) }).color(theme.text_secondary),
        Item::Downloads { pct, .. } => {
            let mut stack = col().center().gap(2.0).child(ui::spinner(theme, 14.0));
            if let Some(p) = pct {
                stack = stack.child(text(format!("{p}%")).font(theme.pixel(9.0)).color(theme.text));
            }
            stack
        }
        Item::Entry(Entry::Folder(name, insts)) => {
            let base = match app.home.covers.get(name) {
                Some(src) => ui::image(app, src, 44.0, 44.0),
                None => colored_icon(&icons::FOLDER, 24.0, theme.text_secondary),
            };
            el().size(44.0, 44.0).center().child(base).child(badge(theme, &insts.len().to_string()).absolute(44.0 - 14.0, 44.0 - 12.0))
        }
        Item::Entry(Entry::Instance(inst)) => {
            let mut tile = el().size(44.0, 44.0).child(ui::instance_icon(app, inst, 44.0));
            if inst.kind == InstanceKind::Server {
                tile = tile.child(badge(theme, "S").absolute(0.0, 0.0).help(t("dock.dedicatedServer")));
            }
            if running || preparing {
                let mut dot = el()
                    .size(10.0, 10.0)
                    .radius(5.0)
                    .absolute(44.0 - 12.0, 0.0)
                    .fill(if running { theme.success } else { theme.accent })
                    .border(2.0, theme.bg_card)
                    .help(if running { t("dock.running") } else { t("dock.preparing") });
                if preparing {
                    dot = dot.looping(rune_motion::LoopSpec::opacity(1.0, 0.0, 0.35, 1.0));
                }
                tile = tile.child(dot);
            }
            if app.install.is_installing(&inst.id) {
                let mut overlay = col().absolute(0.0, 0.0).size(44.0, 44.0).center().gap(2.0).fill(Color::BLACK.alpha(0.55)).child(ui::spinner(theme, 14.0));
                if let Some(p) = app.install.percent(&inst.id) {
                    overlay = overlay.child(text(format!("{p}%")).font(theme.pixel(9.0)).color(theme.text));
                }
                tile = tile.child(overlay);
            }
            tile
        }
        Item::Account => match app.active_account() {
            Some(account) => ui::image(app, &crate::overlays::accounts::skin_face(app, &account.uuid, 44), 44.0, 44.0),
            None => colored_icon(&icons::USER, 24.0, theme.text_secondary),
        },
        Item::Sep => el(),
    };

    let border = if running { theme.success } else if active { theme.border } else { Color::TRANSPARENT };
    let origin = match pos {
        "top" => (0.5, 0.0),
        "left" => (0.0, 0.5),
        "right" => (1.0, 0.5),
        _ => (0.5, 1.0),
    };
    let grow = (scale - 1.0) * ITEM / 2.0;
    let mut button = el()
        .key(format!("dock-{i}"))
        .size(ITEM, ITEM)
        .center()
        .fill(theme.bg_card)
        .border(2.0, border)
        .role("button")
        .label(label.clone())
        .help(label)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover(Color::TRANSPARENT)
        .scale(scale)
        .origin(origin.0, origin.1)
        .scale_motion(Spring::new(0.18, 0.0))
        .press_scale(0.94)
        .child(glyph);
    button = ui::bevel(button, 2.0, 0.05, 0.28);
    button = if matches!(pos, "left" | "right") { button.my(grow) } else { button.mx(grow) };
    if matches!(item, Item::Nav(Route::Community, ..)) {
        button = button.child(
            el().absolute(ITEM - 22.0, -5.0)
                .px(4.0)
                .py(1.0)
                .fill(theme.accent)
                .border(1.0, theme.bg_app)
                .child(text("beta").font(theme.pixel(8.0)).color(theme.accent_contrast)),
        );
    }
    if active {
        let dot = el().size(4.0, 4.0).fill(theme.accent);
        button = button.child(match pos {
            "top" => dot.absolute(ITEM / 2.0 - 4.0, -8.0),
            "left" => dot.absolute(-8.0, ITEM / 2.0 - 4.0),
            "right" => dot.absolute(ITEM + 2.0, ITEM / 2.0 - 4.0),
            _ => dot.absolute(ITEM / 2.0 - 4.0, ITEM + 2.0),
        });
    }

    match item {
        Item::Nav(route, ..) => {
            let route = route.clone();
            button.on_press(move |app: &mut App| app.go(route.clone()))
        }
        Item::Settings => button.on_press(|app: &mut App| app.go(Route::Settings)),
        Item::Entry(Entry::Instance(inst)) => {
            let id = inst.id.clone();
            let menu_id = inst.id.clone();
            button.on_press(move |app: &mut App| app.go(Route::Instance(id.clone())))
                .on_secondary_press(move |app: &mut App, p, _tx: &mut Tx| app.open_instance_menu(&menu_id, p.x, p.y))
        }
        Item::Entry(Entry::Folder(name, _)) => {
            let name = name.clone();
            button.on_press(move |app: &mut App| app.ui.folder_open = Some(name.clone()))
        }
        Item::Overflow(_) => button.on_press(|app: &mut App| app.dock.overflow_open = !app.dock.overflow_open),
        Item::Downloads { target, .. } => {
            let target = target.clone();
            button.on_press(move |app: &mut App| {
                if let Some(id) = &target {
                    app.go(Route::Instance(id.clone()));
                }
            })
        }
        Item::Add => button.on_press(|app: &mut App| app.ui.create_instance_open = true),
        Item::Account => button.on_press(|app: &mut App| app.ui.accounts_open = true),
        Item::Sep => button,
    }
}

fn badge(theme: &Theme, label: &str) -> El {
    el().px(3.0).py(1.0).fill(theme.accent).child(text(label.to_string()).font(theme.pixel(8.0)).color(theme.bg_app))
}

fn overflow_menu(app: &App, theme: &Theme, rest: &[Entry], pos: &str) -> El {
    let mut menu = col()
        .key("dock-overflow")
        .min_w(200.0)
        .max_h(320.0)
        .padding(6.0)
        .gap(2.0)
        .fill(theme.bg_raised)
        .border(2.0, theme.border)
        .shadow(12.0, 30.0, Color::BLACK.alpha(0.5))
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .transition(rune::Transition::opacity());
    for entry in rest {
        let row_el = match entry {
            Entry::Folder(name, insts) => {
                let folder_name = name.clone();
                row()
                    .key(format!("ov-folder-{name}"))
                    .gap(10.0)
                    .center_y()
                    .px(8.0)
                    .py(7.0)
                    .hover(theme.bg_hover)
                    .cursor(Cursor::Pointer)
                    .on_press(move |app: &mut App| {
                        app.dock.overflow_open = false;
                        app.ui.folder_open = Some(folder_name.clone());
                    })
                    .child(el().size(24.0, 24.0).center().child(colored_icon(&icons::FOLDER, 18.0, theme.text_muted)))
                    .child(text(name.clone()).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).truncate().grow(1.0))
                    .child(text(insts.len().to_string()).font(theme.sans(11.0, 400.0)).color(theme.text_muted).pl(8.0))
            }
            Entry::Instance(inst) => {
                let id = inst.id.clone();
                row()
                    .key(format!("ov-{}", inst.id))
                    .gap(10.0)
                    .center_y()
                    .px(8.0)
                    .py(7.0)
                    .hover(theme.bg_hover)
                    .cursor(Cursor::Pointer)
                    .on_press(move |app: &mut App| {
                        app.dock.overflow_open = false;
                        app.go(Route::Instance(id.clone()));
                    })
                    .child(ui::instance_icon(app, inst, 24.0))
                    .child(text(inst.name.clone()).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).truncate().grow(1.0))
            }
        };
        menu = menu.child(row_el);
    }
    // Above the dock, as the Svelte menu opens off the "+N" button; outside clicks close it.
    place("dock-overflow-wrap", pos, menu, 84.0).modal(true, |app: &mut App| app.dock.overflow_open = false)
}
