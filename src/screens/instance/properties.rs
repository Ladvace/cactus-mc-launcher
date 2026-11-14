//! `ServerProperties.svelte`: a form over the common `server.properties` keys that keeps every
//! other line (comments, unknown keys) as it was.

use std::collections::{HashMap, HashSet};

use rune::{Cursor, Cx, El, TextEdit, Tx, col, rich, row, text};

use super::{boxed, t_, text_field};
use crate::app::{App, Ignore};
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Number,
    Bool,
    Select(&'static [&'static str]),
}

struct Field {
    key: &'static str,
    label: &'static str,
    kind: Kind,
    default: &'static str,
    hint: Option<&'static str>,
}

const fn plain_field(key: &'static str, label: &'static str, kind: Kind, default: &'static str) -> Field {
    Field { key, label, kind, default, hint: None }
}

const FIELDS: [Field; 17] = [
    plain_field("motd", "server.fieldMotd", Kind::Text, "A Minecraft Server"),
    plain_field("max-players", "server.fieldMaxPlayers", Kind::Number, "20"),
    plain_field("difficulty", "server.fieldDifficulty", Kind::Select(&["peaceful", "easy", "normal", "hard"]), "easy"),
    plain_field("gamemode", "server.fieldGamemode", Kind::Select(&["survival", "creative", "adventure", "spectator"]), "survival"),
    plain_field("hardcore", "server.fieldHardcore", Kind::Bool, "false"),
    plain_field("pvp", "server.fieldPvp", Kind::Bool, "true"),
    Field { key: "online-mode", label: "server.fieldOnlineMode", kind: Kind::Bool, default: "true", hint: Some("server.fieldOnlineModeHint") },
    plain_field("white-list", "server.fieldWhitelist", Kind::Bool, "false"),
    plain_field("level-name", "server.fieldWorldName", Kind::Text, "world"),
    plain_field("level-seed", "server.fieldWorldSeed", Kind::Text, ""),
    plain_field("server-port", "server.fieldServerPort", Kind::Number, "25565"),
    plain_field("view-distance", "server.fieldViewDistance", Kind::Number, "10"),
    plain_field("simulation-distance", "server.fieldSimulationDistance", Kind::Number, "10"),
    plain_field("spawn-protection", "server.fieldSpawnProtection", Kind::Number, "16"),
    plain_field("allow-nether", "server.fieldAllowNether", Kind::Bool, "true"),
    plain_field("allow-flight", "server.fieldAllowFlight", Kind::Bool, "false"),
    plain_field("enable-command-block", "server.fieldCommandBlocks", Kind::Bool, "false"),
];

enum Line {
    Kv { key: String, raw: String },
    Raw(String),
}

#[derive(Default)]
pub struct State {
    lines: Vec<Line>,
    /// Bool and select values.
    values: HashMap<&'static str, String>,
    /// Text and number fields.
    edits: HashMap<&'static str, TextEdit>,
    pub open_select: Option<&'static str>,
    pub loading: bool,
    pub saving: bool,
    pub loaded: bool,
}

impl State {
    fn value(&self, key: &str) -> String {
        match self.edits.get(key) {
            Some(edit) => edit.text.clone(),
            None => self.values.get(key).cloned().unwrap_or_default(),
        }
    }
}

fn parse(contents: &str) -> Vec<Line> {
    contents.lines()
        .map(|raw| {
            let trimmed = raw.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return Line::Raw(raw.to_string());
            }
            match raw.find('=') {
                Some(eq) => Line::Kv { key: raw[..eq].trim().to_string(), raw: raw.to_string() },
                None => Line::Raw(raw.to_string()),
            }
        })
        .collect()
}

pub fn load(app: &mut App) {
    app.instance_page.props.loading = true;
    let id = app.instance_page.id.clone();
    app.fx.blocking(move |host| (crate::commands::read_server_properties(host.clone(), id.clone()), id), |app, (result, id)| {
        if app.instance_page.id != id {
            return;
        }
        let props = &mut app.instance_page.props;
        props.loading = false;
        match result {
            Ok(contents) => {
                let found: HashMap<String, String> = contents
                    .lines()
                    .filter(|line| !line.trim_start().is_empty() && !line.trim_start().starts_with('#'))
                    .filter_map(|line| line.find('=').map(|eq| (line[..eq].trim().to_string(), line[eq + 1..].to_string())))
                    .collect();
                props.lines = parse(&contents);
                props.values.clear();
                props.edits.clear();
                for field in &FIELDS {
                    let value = found.get(field.key).cloned().unwrap_or_else(|| field.default.to_string());
                    match field.kind {
                        Kind::Text | Kind::Number => {
                            props.edits.insert(field.key, TextEdit::new(value));
                        }
                        _ => {
                            props.values.insert(field.key, value);
                        }
                    }
                }
                props.loaded = true;
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn save(app: &mut App) {
    let props = &app.instance_page.props;
    let managed: HashSet<&str> = FIELDS.iter().map(|field| field.key).collect();
    let mut seen = HashSet::new();
    let mut out: Vec<String> = props
        .lines
        .iter()
        .map(|line| match line {
            Line::Kv { key, .. } if managed.contains(key.as_str()) => {
                seen.insert(key.clone());
                format!("{key}={}", props.value(key))
            }
            Line::Kv { raw, .. } | Line::Raw(raw) => raw.clone(),
        })
        .collect();
    for field in &FIELDS {
        let value = props.value(field.key);
        if !seen.contains(field.key) && !value.is_empty() {
            out.push(format!("{}={value}", field.key));
        }
    }
    let mut contents = out.join("\n");
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    app.instance_page.props.saving = true;
    let id = app.instance_page.id.clone();
    app.fx.blocking(
        move |host| crate::commands::write_server_properties(host.clone(), host.state(), id, contents.clone()).map(|()| contents),
        |app, result| {
            app.instance_page.props.saving = false;
            match result {
                Ok(contents) => {
                    app.instance_page.props.lines = parse(&contents);
                    app.toasts.success(t_("server.propsSaved"));
                }
                Err(e) => app.toasts.error(e.to_string()).ignore(),
            }
        },
    );
}

pub fn view(app: &App, theme: &Theme, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let props = &app.instance_page.props;
    let running = app.is_running(&app.instance_page.id);

    let (prefix, code, suffix) = (t_("server.propsLeadPrefix"), "server.properties", t_("server.propsLeadSuffix"));
    let mut lead = format!("{prefix}{code}{suffix}");
    let mut spans = vec![rune::Span::new(prefix.len(), prefix.len() + code.len()).color(theme.accent)];
    if running {
        let start = lead.len() + 1;
        let hint = t_("server.propsRestartHint");
        lead = format!("{lead} {hint}");
        spans.push(rune::Span::new(start, start + hint.len()).weight(700.0));
    }
    let head = row()
        .w_full()
        .justify_between()
        .gap(12.0)
        .mb(16.0)
        .child(rich(lead, spans).font(theme.sans(12.5, 400.0)).color(theme.text_muted).wrap().max_w(520.0).grow(1.0).shrink())
        .child(row().gap(8.0)
            .child(Btn::new(t_("server.reload")).ghost().sm().key("props-reload").disabled(props.loading || props.saving).on_press(load).build(theme))
            .child(Btn::new(if props.saving { t_("server.saving") } else { t_("common.save") }).primary().sm().key("props-save")
                .disabled(props.loading || props.saving || !props.loaded).on_press(save).build(theme)));

    if props.loading && !props.loaded {
        return col().w_full().child(head).child(ui::muted(theme, t_("common.loading"), 12.5));
    }

    // `repeat(auto-fill, minmax(220px, 1fr))` with a 16 pt column gap; text fields span the row.
    let width = cx.layout_size("props-grid").map_or(1040.0, |size| size[0]);
    let cols = (((width + 16.0) / (220.0 + 16.0)).floor() as usize).max(1);
    let col_w = ((width - 16.0 * (cols as f32 - 1.0)) / cols as f32).max(120.0);
    let mut grid = col().key("props-grid").w_full().gap(14.0);
    let mut line: Vec<El> = Vec::new();
    let flush = |grid: El, line: &mut Vec<El>| {
        if line.is_empty() {
            return grid;
        }
        let mut grid_row = row().gap(16.0).equal_columns(cols);
        for cell in line.drain(..) {
            grid_row = grid_row.child(cell);
        }
        grid.child(grid_row)
    };
    let kit = theme.kit();
    for field in &FIELDS {
        let cell = field_view(app, theme, &kit, field, col_w, layer);
        if field.kind == Kind::Text {
            grid = flush(grid, &mut line);
            grid = grid.child(cell);
        } else {
            line.push(cell);
            if line.len() == cols {
                grid = flush(grid, &mut line);
            }
        }
    }
    grid = flush(grid, &mut line);
    col().w_full().child(head).child(grid)
}

fn field_view(app: &App, theme: &Theme, kit: &rune_kit::Kit, field: &'static Field, col_w: f32, layer: &mut Vec<El>) -> El {
    let props = &app.instance_page.props;
    let key = field.key;
    let mut cell = col().key(format!("prop-{key}")).gap(6.0);
    match field.kind {
        Kind::Bool => {
            let checked = props.values.get(key).is_some_and(|value| value == "true");
            cell = cell.child(
                ui::check_row(theme, &format!("check-{key}"), t_(field.label), checked, move |app| {
                    let toggled = if checked { "false" } else { "true" };
                    app.instance_page.props.values.insert(key, toggled.to_string());
                })
                .pt(4.0)
                .cursor(Cursor::Pointer),
            );
        }
        Kind::Select(options) => {
            let current = props.values.get(key).cloned().unwrap_or_default();
            cell = cell.child(ui::field_label(theme, t_(field.label)).mb(0.0)).child(
                rune_kit::Select::new(format!("select-{key}"), options)
                    .appearance(rune_kit::SelectAppearance::Field)
                    .width(col_w)
                    .current(Some(&current))
                    .open(props.open_select == Some(key))
                    .on_toggle(move |app: &mut App| {
                        let open_select = &mut app.instance_page.props.open_select;
                        *open_select = if *open_select == Some(key) { None } else { Some(key) };
                    })
                    .on_close(|app: &mut App| app.instance_page.props.open_select = None)
                    .on_choose(move |app: &mut App, choice: &str, _: &mut Tx| {
                        app.instance_page.props.values.insert(key, choice.to_string());
                        app.instance_page.props.open_select = None;
                    })
                    .build(kit, layer),
            );
        }
        Kind::Text | Kind::Number => {
            let edit = props.edits.get(key).cloned().unwrap_or_default();
            let field_el = text_field(theme, &edit, "", rune::edit_access(move |app: &mut App| app.instance_page.props.edits.entry(key).or_default())).key(format!("input-{key}"));
            cell = cell.child(ui::field_label(theme, t_(field.label)).mb(0.0)).child(boxed(theme, &edit, field_el));
        }
    }
    if let Some(hint) = field.hint {
        cell = cell.child(text(t_(hint)).font(theme.sans(11.5, 400.0)).color(theme.text_muted).wrap());
    }
    cell
}
