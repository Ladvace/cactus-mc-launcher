//! Achievements: `src/routes/achievements/+page.svelte`.

use std::sync::Mutex;

use rune::tok::TextStyle;
use rune::{Color, Cursor, Cx, El, Icon, ScrollSpec, Tx, col, el, icon, row, text};
use rune_kit::Select;

use crate::achievements::{AchievementsPayload, AdvancementView, StatEntry};
use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Achievements,
    Stats,
}

#[derive(Default)]
pub struct State {
    data: Option<AchievementsPayload>,
    loading: bool,
    error: Option<String>,
    tab: Tab,
    /// `None` is "all".
    category: Option<String>,
    hide_locked: bool,
    category_open: bool,
}

const CATEGORIES: [(&str, &str, &str); 6] = [
    ("story", "achievements.catStory", "book"),
    ("nether", "achievements.catNether", "fire"),
    ("end", "achievements.catEnd", "moon"),
    ("adventure", "achievements.catAdventure", "flag"),
    ("husbandry", "achievements.catHusbandry", "heart"),
    ("other", "achievements.catOther", "trophy"),
];

fn category_label(key: &str) -> String {
    CATEGORIES.iter().find(|entry| entry.0 == key).map_or_else(|| key.to_string(), |entry| tr(entry.1))
}

fn category_icon(key: &str) -> &'static Icon {
    icons::by_name(CATEGORIES.iter().find(|entry| entry.0 == key).map_or("trophy", |entry| entry.2))
}

fn load(app: &mut App) {
    app.achievements.loading = true;
    app.achievements.error = None;
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::get_achievements(host).await }, |app, result| {
        let state = &mut app.achievements;
        match result {
            Ok(payload) => state.data = Some(payload),
            Err(e) => state.error = Some(e.to_string()),
        }
        state.loading = false;
    });
}

pub fn on_enter(app: &mut App) {
    app.achievements.category_open = false;
    load(app);
}

pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

// Formatting.

/// `toLocaleString()`: 12,345.
fn group_thousands(n: i64) -> String {
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

fn hours(ticks: i64) -> String {
    let played_hours = ticks as f64 / 20.0 / 3600.0;
    if played_hours >= 100.0 { format!("{} h", played_hours.round()) } else { format!("{played_hours:.1} h") }
}

fn distance(cm: i64) -> String {
    let km = cm as f64 / 100.0 / 1000.0;
    if km >= 1.0 { format!("{km:.1} km") } else { format!("{} m", (cm as f64 / 100.0).round()) }
}

/// "minecraft:oak_log" → "Oak Log".
fn title_case_key(key: &str) -> String {
    let name = key.rsplit(':').next().unwrap_or(key).replace('_', " ");
    let mut out = String::with_capacity(name.len());
    let mut start = true;
    for ch in name.chars() {
        let word = ch.is_alphanumeric() || ch == '_';
        if start && word {
            out.extend(ch.to_uppercase());
        } else {
            out.push(ch);
        }
        start = !word;
    }
    out
}

fn display_name(adv: &AdvancementView) -> String {
    if !adv.done && adv.hidden { "???".into() } else { adv.name.clone() }
}

fn display_desc(adv: &AdvancementView) -> String {
    if adv.done {
        adv.earned_in.clone().unwrap_or_else(|| tr("achievements.earned"))
    } else if adv.hidden {
        tr("achievements.hiddenUnlock")
    } else {
        tr("achievements.locked")
    }
}

// Pieces.

fn style(theme: &Theme, size: f32, weight: f32) -> TextStyle {
    theme.sans(size, weight)
}

fn styled_text(theme: &Theme, content: impl Into<String>, size: f32, color: Color) -> El {
    text(content.into()).font(style(theme, size, 400.0)).color(color)
}

fn strong(theme: &Theme, content: impl Into<String>, size: f32) -> El {
    text(content.into()).font(style(theme, size, 700.0)).color(theme.text)
}

/// `.track` / `.fill`: a flat bar, `frac` of it filled.
fn track(theme: &Theme, frac: f32, h: f32) -> El {
    row().w_full().h(h).fill(theme.border).clip_corners([0.0; 4])
        .child(el().h_full().w_frac(frac.clamp(0.0, 1.0)).fill(theme.accent))
}

/// Inset bevel used by the hero, tiles and stats.
fn tile_bevel(tile: El, light: f32, dark: f32) -> El {
    ui::bevel(tile, 2.0, light, dark)
}

/// `repeat(auto-fill, minmax(min, 1fr))`.
fn grid(width: f32, min: f32, gap_x: f32, gap_y: f32, items: Vec<El>) -> El {
    let columns = (((width + gap_x) / (min + gap_x)).floor() as usize).max(1);
    let cell = ((width - gap_x * (columns as f32 - 1.0)) / columns as f32).floor();
    let mut lines = col().w_full().gap(gap_y);
    let mut items = items.into_iter().peekable();
    let mut line_index = 0;
    while items.peek().is_some() {
        let mut line = row().key(format!("r{line_index}")).gap(gap_x).items_stretch();
        for item in items.by_ref().take(columns) {
            line = line.child(item.w(cell));
        }
        lines = lines.child(line);
        line_index += 1;
    }
    lines
}

/// The conic-gradient ring: a full track and an arc of `pct`, as stroked SVG icons.
fn ring_icon(pct: u32) -> &'static Icon {
    static CACHE: Mutex<Vec<Option<&'static Icon>>> = Mutex::new(Vec::new());
    let pct = pct.min(100) as usize;
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() < 101 {
        cache.resize(101, None);
    }
    if let Some(cached) = cache[pct] {
        return cached;
    }
    // closest-side 79%: the ring runs from r 42.7 to 54 of the 108 box.
    let (center, radius, stroke) = (54.0_f32, 48.35_f32, 11.3_f32);
    let body: String = if pct == 100 {
        format!(r#"<circle cx="{center}" cy="{center}" r="{radius}" stroke="currentColor" stroke-width="{stroke}" fill="none"/>"#)
    } else if pct == 0 {
        String::new()
    } else {
        let angle = pct as f32 / 100.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let (x, y) = (center + radius * angle.cos(), center + radius * angle.sin());
        let large = if pct > 50 { 1 } else { 0 };
        format!(r#"<path d="M{center} {:.3}A{radius} {radius} 0 {large} 1 {x:.3} {y:.3}" stroke="currentColor" stroke-width="{stroke}" fill="none"/>"#, center - radius)
    };
    let ring: &'static Icon = Box::leak(Box::new(Icon { name: "ach-ring", view: 108.0, body: Box::leak(body.into_boxed_str()) }));
    cache[pct] = Some(ring);
    ring
}

fn hero_ring(theme: &Theme, pct: u32) -> El {
    let track = ring_icon(100);
    el().key("hero-ring").size(108.0, 108.0)
        .child(icon(track, 108.0).absolute(0.0, 0.0).color(theme.border))
        .child(icon(ring_icon(pct), 108.0).absolute(0.0, 0.0).color(theme.accent))
        .child(
            col().size(108.0, 108.0).absolute(0.0, 0.0).center().gap(2.0)
                .child(text(format!("{pct}%")).font(style(theme, 26.0, 700.0)).color(theme.accent))
                .child(
                    text(tr("achievements.complete").to_uppercase())
                        .font(TextStyle { tracking: 11.0 * 0.06, ..style(theme, 11.0, 400.0) })
                        .color(theme.text_muted),
                ),
        )
}

fn h2(theme: &Theme, content: impl Into<String>) -> El {
    text(content.into()).font(theme.pixel(15.0)).color(theme.text)
}

fn hero(app: &App, cx: &mut Cx<App>, payload: &AchievementsPayload) -> El {
    let theme = app.theme();
    let earned_custom = payload.custom.iter().filter(|custom| custom.earned).count();
    let worlds = payload.player.worlds_scanned;
    let line1 = row().center_y().mb(4.0)
        .child(strong(&theme, payload.completion.earned.to_string(), 15.0))
        .child(styled_text(&theme, format!(" / {} {}", payload.completion.total, tr("achievements.advancementsLabel")), 15.0, theme.text));
    let line2 = row().center_y().mb(14.0).flex_wrap()
        .child(strong(&theme, earned_custom.to_string(), 13.0))
        .child(styled_text(&theme, format!(" / {} {} · ", payload.custom.len(), tr("achievements.challengesLabel")), 13.0, theme.text_muted))
        .child(strong(&theme, worlds.to_string(), 13.0))
        .child(styled_text(
            &theme,
            format!(" {} {}", if worlds == 1 { tr("achievements.world") } else { tr("achievements.worlds") }, tr("achievements.scanned")),
            13.0,
            theme.text_muted,
        ));
    let categories_w = cx.layout_size("ach-cats").map_or(600.0, |[w, _]| w);
    let bars = payload
        .categories
        .iter()
        .map(|category| {
            let frac = if category.total > 0 { category.earned as f32 / category.total as f32 } else { 0.0 };
            col().key(format!("cat-{}", category.key))
                .child(
                    row().gap(6.0).center_y().mb(4.0)
                        .child(ui::colored_icon(category_icon(&category.key), 13.0, theme.text_muted))
                        .child(styled_text(&theme, category_label(&category.key), 12.0, theme.text_muted).truncate().shrink().min_w(0.0))
                        .child(el().grow(1.0))
                        .child(text(format!("{}/{}", category.earned, category.total)).font(theme.mono(11.0)).color(theme.text)),
                )
                .child(track(&theme, frac, 6.0))
        })
        .collect();
    let meta = col().key("hero-meta").grow(1.0).shrink().min_w(0.0)
        .child(line1)
        .child(line2)
        .child(col().key("ach-cats").w_full().child(grid(categories_w, 150.0, 18.0, 10.0, bars)));
    tile_bevel(row().key("hero").center_y().gap(24.0).fill(theme.bg_card).border(2.0, theme.border).py(20.0).px(24.0).mb(20.0), 0.04, 0.28)
        .child(hero_ring(&theme, payload.completion.percent))
        .child(meta)
}

fn tabs(theme: &Theme, tab: Tab) -> El {
    let tab_button = |key: &str, label: String, target: Tab| {
        let active = tab == target;
        col().key(key.to_string()).role("tab").checked(active).focusable(true).cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| app.achievements.tab = target)
            .child(styled_text(theme, label, 14.0, if active { theme.text } else { theme.text_muted }).px(14.0).py(8.0))
            .child(el().h(2.0).w_full().fill(if active { theme.accent } else { Color::TRANSPARENT }))
    };
    // The 2 pt rule under the tabs, with the active tab's accent drawn over it (margin-bottom: -2px).
    el().key("tabs").w_full().mb(18.0)
        .child(el().h(2.0).fill(theme.border).pin(Some(0.0), None, Some(0.0), Some(0.0)))
        .child(
            row().gap(4.0)
                .child(tab_button("tab-achievements", tr("achievements.title"), Tab::Achievements))
                .child(tab_button("tab-stats", tr("achievements.tabLifetimeStats"), Tab::Stats)),
        )
}

fn achievements_tab(app: &App, layer: &mut Vec<El>, payload: &AchievementsPayload, width: f32) -> El {
    let theme = app.theme();
    let state = &app.achievements;

    let badge = text(tr("achievements.launcherExclusive").to_uppercase())
        .font(TextStyle { tracking: 10.0 * 0.05, ..style(&theme, 10.0, 400.0) })
        .color(theme.accent);
    let customs = payload
        .custom
        .iter()
        .map(|custom| {
            let (bg, ink) = if custom.earned { (theme.accent_soft, theme.accent) } else { (theme.border, theme.text_muted) };
            let mut name = row().gap(5.0).center_y()
                .child(text(custom.name.clone()).font(style(&theme, 13.0, 600.0)).color(if custom.earned { theme.text } else { theme.text_muted }));
            if custom.earned {
                name = name.child(ui::colored_icon(&icons::CHECK, 13.0, theme.accent));
            }
            let mut body = col().grow(1.0).shrink().min_w(0.0)
                .child(name)
                .child(text(custom.description.clone()).font(TextStyle { line_height: 11.5 * 1.4, ..style(&theme, 11.5, 400.0) }).color(theme.text_muted).wrap().mt(2.0));
            if !custom.earned && custom.progress > 0 {
                body = body.child(track(&theme, custom.progress as f32 / 100.0, 4.0).mt(6.0));
            }
            tile_bevel(row().key(format!("custom-{}", custom.id)).gap(12.0).fill(theme.bg_card).border(2.0, theme.border).py(12.0).px(14.0), 0.03, 0.25)
                .child(el().size(38.0, 38.0).center().fill(bg).child(ui::colored_icon(icons::by_name(&custom.icon), 20.0, ink)))
                .child(body)
        })
        .collect();
    let challenges = col().key("challenges").mb(26.0)
        .child(row().gap(8.0).center_y().mb(12.0).child(h2(&theme, tr("achievements.cactusChallenges"))).child(el().py(2.0).px(7.0).fill(theme.accent_soft).child(badge)))
        .child(grid(width, 240.0, 10.0, 10.0, customs));

    // Category filter: the Select shows labels; map them back to keys.
    let mut labels = vec![tr("achievements.allCategories")];
    labels.extend(payload.categories.iter().map(|category| category_label(&category.key)));
    let current = match &state.category {
        None => labels[0].clone(),
        Some(key) => category_label(key),
    };
    let select = Select::new("ach-category", &labels)
        .width(180.0)
        .label(tr("achievements.allCategories"))
        .current(Some(&current))
        .open(state.category_open)
        .on_toggle(|app: &mut App| app.achievements.category_open = !app.achievements.category_open)
        .on_close(|app: &mut App| app.achievements.category_open = false)
        .on_choose(|app: &mut App, label: &str, _: &mut Tx| {
            let state = &mut app.achievements;
            state.category_open = false;
            state.category = state.data.as_ref().and_then(|data| data.categories.iter().find(|category| category_label(&category.key) == label).map(|category| category.key.clone()));
        })
        .build(&theme.kit(), layer);
    let show_locked = !state.hide_locked;
    let show_locked_toggle = row().key("show-locked").gap(6.0).center_y().role("checkbox").checked(show_locked).label(tr("achievements.showLocked"))
        .focusable(true).cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.achievements.hide_locked = !app.achievements.hide_locked)
        .child(ui::checkbox(&theme, show_locked).a11y_hidden())
        .child(styled_text(&theme, tr("achievements.showLocked"), 12.0, theme.text_muted).a11y_hidden());
    let head = row().key("adv-head").justify_between().center_y().gap(12.0).mb(12.0).flex_wrap()
        .child(h2(&theme, tr("achievements.advancementsHeading")))
        .child(row().gap(14.0).center_y().child(show_locked_toggle).child(select));

    let visible: Vec<&AdvancementView> = payload
        .advancements
        .iter()
        .filter(|adv| state.category.as_ref().is_none_or(|category| *category == adv.category) && (show_locked || adv.done))
        .collect();
    let date_format = app.settings.date_format.clone();
    let tiles = visible
        .iter()
        .map(|adv| {
            let (bg, ink) = if adv.done { (theme.accent_soft, theme.accent) } else { (theme.border, theme.text_muted) };
            let mut tile = row().key(format!("adv-{}", adv.id)).center_y().gap(10.0).fill(theme.bg_card).border(1.0, theme.border).py(9.0).px(11.0)
                .help(adv.name.clone())
                .child(el().size(30.0, 30.0).center().fill(bg).child(ui::colored_icon(category_icon(&adv.category), 18.0, ink)))
                .child(
                    col().grow(1.0).shrink().min_w(0.0)
                        .child(text(display_name(adv)).font(style(&theme, 12.5, 600.0)).color(if adv.done { theme.text } else { theme.text_muted }).truncate())
                        .child(styled_text(&theme, display_desc(adv), 10.5, theme.text_muted).truncate()),
                );
            if adv.done {
                if let Some(at) = &adv.earned_at {
                    let date = ui::fmt::date_str(at, &date_format);
                    if !date.is_empty() {
                        tile = tile.child(text(date).font(theme.mono(10.0)).color(theme.text_muted));
                    }
                }
            }
            tile
        })
        .collect();
    let mut advancements = col().key("advancements").mb(26.0).child(head).child(grid(width, 210.0, 8.0, 8.0, tiles));
    if visible.is_empty() {
        advancements = advancements.child(styled_text(&theme, tr("achievements.noFilterResults"), 12.0, theme.text_muted).py(8.0));
    }
    col().w_full().child(challenges).child(advancements)
}

fn bars(theme: &Theme, key: &str, entries: &[StatEntry]) -> El {
    if entries.is_empty() {
        return styled_text(theme, tr("achievements.noData"), 12.0, theme.text_muted).py(8.0);
    }
    let max = entries[0].count.max(1) as f32;
    col().key(key.to_string()).gap(8.0).children(entries.iter().map(|entry| {
        row().key(entry.key.clone()).gap(10.0).center_y()
            .child(styled_text(theme, title_case_key(&entry.key), 12.0, theme.text).truncate().w(110.0))
            .child(el().grow(1.0).w(0.0).child(track(theme, entry.count as f32 / max, 6.0)))
            .child(text(group_thousands(entry.count)).font(theme.mono(11.0)).color(theme.text_muted).align(rune::Align::End).w(60.0))
    }))
}

fn stats_tab(app: &App, payload: &AchievementsPayload, width: f32) -> El {
    let theme = app.theme();
    let lifetime = &payload.stats;
    let stat = |value: String, label: String| {
        tile_bevel(col().gap(3.0).fill(theme.bg_card).border(2.0, theme.border).py(14.0).px(16.0), 0.03, 0.25)
            .child(text(value).font(style(&theme, 20.0, 700.0)).color(theme.accent))
            .child(styled_text(&theme, label, 11.5, theme.text_muted))
    };
    let stats = vec![
        stat(hours(lifetime.play_time_ticks), tr("achievements.playtime")),
        stat(group_thousands(lifetime.blocks_mined), tr("achievements.blocksMined")),
        stat(group_thousands(lifetime.mobs_killed), tr("achievements.mobsKilled")),
        stat(group_thousands(lifetime.deaths), tr("achievements.deaths")),
        stat(distance(lifetime.distance_cm), tr("achievements.distanceTraveled")),
        stat(group_thousands(lifetime.jumps), tr("achievements.jumps")),
        stat(group_thousands(lifetime.items_picked_up), tr("achievements.itemsPickedUp")),
        stat(group_thousands(lifetime.times_slept), tr("achievements.nightsSlept")),
    ];
    let col_w = ((width - 20.0) / 2.0).floor();
    let top_lists = if width > 640.0 - 64.0 {
        row().gap(20.0).items_start()
            .child(col().w(col_w).mb(26.0).child(h2(&theme, tr("achievements.topMinedBlocks")).mb(12.0)).child(bars(&theme, "top-mined", &lifetime.top_mined)))
            .child(col().w(col_w).mb(26.0).child(h2(&theme, tr("achievements.topMobsDefeated")).mb(12.0)).child(bars(&theme, "top-killed", &lifetime.top_killed)))
    } else {
        col().w_full()
            .child(col().mb(26.0).child(h2(&theme, tr("achievements.topMinedBlocks")).mb(12.0)).child(bars(&theme, "top-mined", &lifetime.top_mined)))
            .child(col().mb(26.0).child(h2(&theme, tr("achievements.topMobsDefeated")).mb(12.0)).child(bars(&theme, "top-killed", &lifetime.top_killed)))
    };

    // grid-template-columns: 2fr 1fr 1fr 1fr 1fr 1fr
    let cell = |content: El, fr: f32| content.w(0.0).grow(fr).min_w(0.0);
    let th_style = TextStyle { tracking: 11.0 * 0.04, ..style(&theme, 11.0, 400.0) };
    let th = |label: String, fr: f32| cell(text(label.to_uppercase()).font(th_style).color(theme.text_muted).truncate(), fr);
    let td = |value: String| cell(styled_text(&theme, value, 12.5, theme.text).truncate(), 1.0);
    let n = payload.instances.len();
    let mut table = col().key("by-instance").border(2.0, theme.border).child(
        row().key("th").gap(8.0).py(9.0).px(14.0).fill(Color::BLACK.alpha(0.15)).border_sides([0.0, 0.0, if n > 0 { 1.0 } else { 0.0 }, 0.0], theme.border)
            .child(th(tr("achievements.instance"), 2.0))
            .child(th(tr("achievements.headerWorlds"), 1.0))
            .child(th(tr("achievements.playtime"), 1.0))
            .child(th(tr("achievements.headerMined"), 1.0))
            .child(th(tr("achievements.headerKilled"), 1.0))
            .child(th(tr("achievements.deaths"), 1.0)),
    );
    for (i, inst) in payload.instances.iter().enumerate() {
        table = table.child(
            row().key(format!("tr-{}", inst.id)).gap(8.0).py(9.0).px(14.0).border_sides([0.0, 0.0, if i + 1 < n { 1.0 } else { 0.0 }, 0.0], theme.border)
                .child(cell(text(inst.name.clone()).font(style(&theme, 12.5, 600.0)).color(theme.text).truncate(), 2.0))
                .child(td(inst.worlds.to_string()))
                .child(td(hours(inst.play_time_ticks)))
                .child(td(group_thousands(inst.blocks_mined)))
                .child(td(group_thousands(inst.mobs_killed)))
                .child(td(group_thousands(inst.deaths))),
        );
    }

    col().w_full()
        .child(col().key("stats").mb(26.0).child(grid(width, 150.0, 10.0, 10.0, stats)))
        .child(top_lists)
        .child(col().key("instances").mb(26.0).child(h2(&theme, tr("achievements.byInstance")).mb(12.0)).child(table))
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let state = &app.achievements;
    let width = cx.layout_size("ach-body").map_or(856.0, |[w, _]| w);

    let subtitle = match state.data.as_ref().map(|payload| payload.player.name.as_str()).filter(|name| !name.is_empty()) {
        Some(name) => row().flex_wrap()
            .child(styled_text(&theme, tr("achievements.lifetimeProgressPrefix"), 13.0, theme.text_muted))
            .child(text(name.to_string()).font(style(&theme, 13.0, 700.0)).color(theme.text))
            .child(styled_text(&theme, tr("achievements.lifetimeProgressSuffix"), 13.0, theme.text_muted)),
        None => row().child(styled_text(&theme, tr("achievements.lifetimeMinecraftProgress"), 13.0, theme.text_muted)),
    };
    let head = row().key("head").justify_between().items_start().gap(16.0).mb(22.0)
        .child(col().shrink().min_w(0.0).child(ui::heading(&theme, tr("achievements.title"), 24.0)).child(subtitle.mt(4.0)))
        .child(
            Btn::new(tr("achievements.refresh")).icon(&icons::REFRESH).key("refresh").disabled(state.loading)
                .on_press(load).build(&theme),
        );

    let mut body = col().key("ach-body").w_full().child(head);
    if state.loading {
        body = body.child(el().key("state").w_full().center().py(60.0).px(20.0).child(styled_text(&theme, tr("achievements.scanning"), 14.0, theme.text_muted)));
    } else if let Some(error) = &state.error {
        body = body.child(
            el().key("state").w_full().center().py(60.0).px(20.0)
                .child(styled_text(&theme, tf("achievements.loadError", &[("error", error)]), 14.0, theme.warning).wrap().align(rune::Align::Center)),
        );
    } else if let Some(payload) = &state.data {
        if !payload.has_data {
            body = body.child(
                row().key("banner").center_y().gap(12.0).py(12.0).px(16.0).mb(18.0).fill(theme.bg_card).border(2.0, theme.border)
                    .child(ui::colored_icon(&icons::TROPHY, 18.0, theme.text_muted))
                    .child(
                        col().gap(2.0).shrink().min_w(0.0)
                            .child(text(tr("achievements.noWorldData")).font(style(&theme, 13.0, 700.0)).color(theme.text))
                            .child(text(tr("achievements.noWorldDataHelp")).font(TextStyle { line_height: 18.0, ..style(&theme, 12.0, 400.0) }).color(theme.text_muted).wrap()),
                    ),
            );
        }
        body = body.child(hero(app, cx, payload)).child(tabs(&theme, state.tab));
        body = body.child(match state.tab {
            Tab::Achievements => achievements_tab(app, layer, payload, width),
            Tab::Stats => stats_tab(app, payload, width),
        });
    }

    col().key("achievements").w_full().h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).items_center()
        .child(col().w_full().max_w(920.0).py(28.0).px(32.0).child(body))
}
