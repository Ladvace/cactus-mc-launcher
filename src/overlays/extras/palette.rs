//! `CommandPalette.svelte`: Cmd/Ctrl-K, a search over pages, actions and instances.

use rune::{Color, Cursor, Cx, El, Icon, Key, KeyEvent, ScrollSpec, Transition, Tx, col, el, row, text};

use crate::app::{App, Route};
use crate::core::i18n;
use crate::icons;
use crate::ui;

#[derive(Default)]
pub struct State {
    pub query: rune::TextEdit,
    selected: usize,
    /// The query `selected` was chosen under; a new query selects the first result again.
    selected_for: String,
}

#[derive(Clone)]
enum Run {
    Go(Route),
    CreateInstance,
    Accounts,
    CheckUpdates,
    Changelog,
    Crafting,
}

struct Cmd {
    id: String,
    label: String,
    hint: String,
    icon: &'static Icon,
    instance: Option<usize>,
    run: Run,
}

fn base() -> Vec<Cmd> {
    let go = i18n::t("palette.hintGoTo");
    let act = i18n::t("palette.hintAction");
    let cmd = |id: &str, label: String, hint: &str, icon: &'static Icon, run: Run| Cmd {
        id: id.into(),
        label,
        hint: hint.into(),
        icon,
        instance: None,
        run,
    };
    vec![
        cmd("nav-home", i18n::t("nav.home"), &go, &icons::HOME, Run::Go(Route::Home)),
        cmd("nav-browse", i18n::t("palette.browseMods"), &go, &icons::COMPASS, Run::Go(Route::Browse)),
        cmd("nav-servers", i18n::t("nav.servers"), &go, &icons::GLOBE, Run::Go(Route::Servers)),
        cmd("nav-achievements", i18n::t("palette.achievements"), &go, &icons::TROPHY, Run::Go(Route::Achievements)),
        cmd("nav-community", i18n::t("nav.community"), &go, &icons::USERS, Run::Go(Route::Community)),
        cmd("nav-settings", i18n::t("nav.settings"), &go, &icons::SETTINGS, Run::Go(Route::Settings)),
        cmd("act-create", i18n::t("palette.createInstance"), &act, &icons::PLUS, Run::CreateInstance),
        cmd("act-accounts", i18n::t("palette.accounts"), &act, &icons::USER, Run::Accounts),
        cmd("act-update", i18n::t("palette.checkUpdates"), &act, &icons::REFRESH, Run::CheckUpdates),
        cmd("act-changelog", i18n::t("palette.whatsNew"), &act, &icons::CLOCK, Run::Changelog),
        cmd("act-crafting", i18n::t("palette.crafting"), &act, &icons::GRID, Run::Crafting),
    ]
}

/// `results`: everything when the query is empty, else the labels containing it, earliest match first.
fn results(app: &App) -> Vec<Cmd> {
    let mut all = base();
    for (i, inst) in app.instances.iter().enumerate() {
        let loader = serde_json::to_value(inst.loader).ok().and_then(|value| value.as_str().map(String::from)).unwrap_or_default();
        all.push(Cmd {
            id: format!("inst-{}", inst.id),
            label: inst.name.clone(),
            hint: format!("{loader} {}", inst.mc_version),
            icon: &icons::GRID,
            instance: Some(i),
            run: Run::Go(Route::Instance(inst.id.clone())),
        });
    }
    let query = app.extras.palette.query.text.trim().to_lowercase();
    if query.is_empty() {
        return all;
    }
    let mut scored: Vec<(usize, Cmd)> =
        all.into_iter().filter_map(|cmd| cmd.label.to_lowercase().find(&query).map(|pos| (pos, cmd))).collect();
    scored.sort_by_key(|(pos, _)| *pos);
    scored.into_iter().map(|(_, cmd)| cmd).collect()
}

fn selected(app: &App) -> usize {
    let palette = &app.extras.palette;
    if palette.selected_for == palette.query.text { palette.selected } else { 0 }
}

fn select(app: &mut App, i: usize) {
    let palette = &mut app.extras.palette;
    palette.selected = i;
    palette.selected_for = palette.query.text.clone();
}

pub fn close(app: &mut App) {
    app.ui.command_palette_open = false;
    let palette = &mut app.extras.palette;
    palette.query.clear();
    palette.selected = 0;
    palette.selected_for.clear();
}

fn run(app: &mut App, action: Run) {
    close(app);
    match action {
        Run::Go(route) => app.go(route),
        Run::CreateInstance => app.ui.create_instance_open = true,
        Run::Accounts => app.ui.accounts_open = true,
        Run::CheckUpdates => super::updater::check(app, true),
        Run::Changelog => app.ui.changelog_open = true,
        Run::Crafting => app.ui.crafting_open = true,
    }
}

/// `onKey`: arrows move the selection, Enter runs it, Escape closes.
pub fn on_key(app: &mut App, key: &KeyEvent, tx: &mut Tx) -> bool {
    if !app.ui.command_palette_open {
        return false;
    }
    let list = results(app);
    let current = selected(app);
    match key.key {
        Key::Down => {
            let i = (current + 1).min(list.len().saturating_sub(1));
            select(app, i);
            if let Some(cmd) = list.get(i) {
                tx.scroll_into_view(format!("pal-{}", cmd.id));
            }
        }
        Key::Up => {
            let i = current.saturating_sub(1);
            select(app, i);
            if let Some(cmd) = list.get(i) {
                tx.scroll_into_view(format!("pal-{}", cmd.id));
            }
        }
        Key::Enter => {
            if let Some(cmd) = list.into_iter().nth(current) {
                run(app, cmd.run);
            }
        }
        Key::Escape => close(app),
        _ => return false,
    }
    true
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    if !app.ui.command_palette_open {
        return;
    }
    let theme = app.theme();
    let [win_w, win_h] = super::window_size(cx);
    let list = results(app);
    let selected_index = selected(app);

    let field = rune::text_input(&app.extras.palette.query, theme.sans(15.2, 400.0))
        .single_line()
        .placeholder(i18n::t("palette.placeholder"), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(|app: &mut App| &mut app.extras.palette.query)
        .on_input_key(on_key)
        .key("palette-input")
        .label(i18n::t("palette.placeholder"))
        .grow(1.0);
    let search = row()
        .key("palette-search")
        .center_y()
        .gap(8.8)
        .px(14.4)
        .py(12.0)
        .border_sides([0.0, 0.0, 1.0, 0.0], theme.border)
        .child(ui::colored_icon(&icons::SEARCH, 16.0, theme.text_muted))
        .child(field);

    let mut rows = col().key("palette-list").role("listbox").padding(5.6).min_h(0.0).shrink().scroll(ScrollSpec { stick_bottom: false });
    for (i, cmd) in list.iter().enumerate() {
        let icon = match cmd.instance.and_then(|index| app.instances.get(index)) {
            Some(inst) => ui::instance_icon(app, inst, 22.0),
            None => ui::colored_icon(cmd.icon, 16.0, theme.text_muted),
        };
        let action = cmd.run.clone();
        let mut line = row()
            .key(format!("pal-{}", cmd.id))
            .role("option")
            .label(cmd.label.clone())
            .checked(i == selected_index)
            .center_y()
            .gap(9.6)
            .w_full()
            .px(9.6)
            .py(8.0)
            .radius(4.0)
            .cursor(Cursor::Pointer)
            .on_hover(move |app: &mut App| {
                if selected(app) != i {
                    select(app, i);
                }
            })
            .on_press(move |app: &mut App| run(app, action.clone()))
            .child(el().w(22.0).h(22.0).center().child(icon))
            .child(text(cmd.label.clone()).font(theme.sans(14.0, 400.0)).color(theme.text).truncate().grow(1.0).shrink());
        if i == selected_index {
            line = line.fill(theme.accent_soft);
        }
        if !cmd.hint.is_empty() {
            line = line.child(text(cmd.hint.clone()).font(theme.mono(11.5)).color(theme.text_muted));
        }
        rows = rows.child(line);
    }
    if list.is_empty() {
        rows = rows.child(text(i18n::t("palette.noMatches")).font(theme.sans(14.0, 400.0)).color(theme.text_muted).px(9.6).py(8.0));
    }

    let palette = col()
        .key("palette")
        .role("dialog")
        .label(i18n::t("palette.placeholder"))
        .w((win_w - 32.0).min(560.0))
        .max_h(win_h * 0.6)
        .fill(theme.bg_card)
        .border(2.0, theme.border)
        .shadow(12.0, 30.0, Color::BLACK.alpha(0.5))
        .clip_corners([0.0; 4])
        .transition(Transition::scale(0.98).combined(Transition::offset(0.0, -6.0)))
        .on_press(|_: &mut App| {})
        .child(search)
        .child(rows);
    layer.push(
        col()
            .key("palette-overlay")
            .w_full()
            .h_full()
            .absolute(0.0, 0.0)
            .items_center()
            .pt(win_h * 0.12)
            .fill(Color::BLACK.alpha(0.45))
            .transition(Transition::opacity())
            .modal(true, close)
            .child(palette),
    );
}
