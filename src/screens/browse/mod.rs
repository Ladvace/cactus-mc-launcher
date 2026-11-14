//! Browse: `src/routes/browse/+page.svelte`, ProjectModal and InstancePicker.

mod picker;
mod project;
mod select;

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use rune::tok::TextStyle;
use rune::{Color, Cursor, Cx, El, ScrollSpec, col, el, row, text};

use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::instance::ModLoader;
use crate::modrinth::{Category, SearchHit, SearchParams};
use crate::sources::Source;
use crate::ui::{self, Btn, fmt};

use select::{Opt, hover_ring, select, sunken};

const TABS: [(&str, &str); 5] = [
    ("browse.tabModpacks", "modpack"),
    ("browse.tabMods", "mod"),
    ("browse.tabResourcePacks", "resourcepack"),
    ("browse.tabShaders", "shader"),
    ("browse.tabDatapacks", "datapack"),
];

const SORTS: [(&str, &str); 5] = [
    ("relevance", "browse.sortRelevance"),
    ("downloads", "browse.sortDownloads"),
    ("follows", "browse.sortFollows"),
    ("newest", "browse.sortNewest"),
    ("updated", "browse.sortUpdated"),
];

const LOADERS: [&str; 5] = ["", "fabric", "quilt", "forge", "neoforge"];

/// `SOURCES` in types.ts.
const SOURCES: [(Source, &str, &str); 2] = [(Source::Modrinth, "modrinth", "Modrinth"), (Source::CurseForge, "curseforge", "CurseForge")];

const LIMIT: u64 = 20;
const DEBOUNCE: Duration = Duration::from_millis(350);

/// The parts of a `SearchHit` the page and the modal show.
#[derive(Clone, Debug)]
pub struct Hit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub project_type: String,
    pub source: String,
}

impl From<SearchHit> for Hit {
    fn from(hit: SearchHit) -> Hit {
        Hit {
            project_id: hit.project_id,
            title: hit.title,
            description: hit.description,
            author: hit.author,
            downloads: hit.downloads,
            icon_url: hit.icon_url,
            project_type: hit.project_type,
            source: hit.source,
        }
    }
}

pub struct State {
    source: Source,
    active_type: &'static str,
    query: rune::TextEdit,
    debounced: String,
    query_gen: u64,
    game_version: String,
    loader: String,
    sort: &'static str,
    show_filters: bool,
    all_categories: Vec<Category>,
    selected_categories: Vec<String>,
    environment: &'static str,
    open_source: bool,
    source_enabled: BTreeMap<String, bool>,
    game_versions: Vec<String>,
    hits: Vec<Hit>,
    total_hits: u64,
    offset: u64,
    loading: bool,
    loading_more: bool,
    error: Option<String>,
    search_gen: u64,
    list_view: bool,
    /// `ui.browseInstanceId` as the page found it on mount.
    from_instance: Option<String>,
    open_select: Option<&'static str>,
    select_query: rune::TextEdit,
    /// Instances whose first `modpack-progress` has arrived (installStore's start tokens).
    progress_seen: HashSet<String>,
    project: project::State,
    picker: picker::State,
}

impl Default for State {
    fn default() -> State {
        State {
            source: Source::Modrinth,
            active_type: "mod",
            query: rune::TextEdit::default(),
            debounced: String::new(),
            query_gen: 0,
            game_version: String::new(),
            loader: String::new(),
            sort: "relevance",
            show_filters: false,
            all_categories: Vec::new(),
            selected_categories: Vec::new(),
            environment: "",
            open_source: false,
            source_enabled: BTreeMap::from([("modrinth".into(), true), ("curseforge".into(), false)]),
            game_versions: Vec::new(),
            hits: Vec::new(),
            total_hits: 0,
            offset: 0,
            loading: false,
            loading_more: false,
            error: None,
            search_gen: 0,
            list_view: false,
            from_instance: None,
            open_select: None,
            select_query: rune::TextEdit::default(),
            progress_seen: HashSet::new(),
            project: project::State::default(),
            picker: picker::State::default(),
        }
    }
}

pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// Mount: a fresh page (the Svelte page remounts), preset from the instance it was opened for.
pub fn on_enter(app: &mut App) {
    let from_id = app.ui.browse_instance_id.take();
    let from_inst = from_id.as_deref().and_then(|id| app.instance(id)).cloned();
    let previous = std::mem::take(&mut app.browse);
    let browse = &mut app.browse;
    // Lists that don't depend on the page's state survive a remount.
    browse.all_categories = previous.all_categories;
    browse.game_versions = previous.game_versions;
    browse.progress_seen = previous.progress_seen;
    browse.list_view = app.prefs.read("cactus:browseView", "grid".to_string()) == "list";
    if let Some(inst) = &from_inst {
        let vanilla = inst.loader == ModLoader::Vanilla;
        browse.active_type = if vanilla { "resourcepack" } else { "mod" };
        browse.game_version = inst.mc_version.clone();
        browse.loader = if vanilla { String::new() } else { inst.loader.id().to_string() };
        browse.from_instance = Some(inst.id.clone());
    }

    let sources = crate::commands::list_sources();
    app.browse.source_enabled = sources.into_iter().map(|source| (source.id, source.enabled)).collect();
    if app.browse.all_categories.is_empty() {
        app.fx.spawn(async { crate::commands::get_content_categories().await }, |app, result| {
            if let Ok(list) = result {
                app.browse.all_categories = list;
            }
        });
    }
    if app.browse.game_versions.is_empty() {
        app.fx.spawn(async { crate::commands::get_minecraft_versions().await }, |app, result| {
            if let Ok(list) = result {
                app.browse.game_versions = list.versions.into_iter().filter(|version| version.kind == "release").map(|version| version.id).collect();
            }
        });
    }
    search(app);
}

pub fn on_event(app: &mut App, name: &str, payload: &serde_json::Value) {
    if name == "modpack-progress" {
        if let Some(id) = payload.get("instanceId").and_then(|value| value.as_str()) {
            app.browse.progress_seen.insert(id.to_string());
        }
    }
    project::on_event(app, name, payload);
}

fn show_loader(browse: &State) -> bool {
    browse.active_type == "mod" || browse.active_type == "modpack"
}

fn active_filter_count(browse: &State) -> usize {
    browse.selected_categories.len() + usize::from(!browse.environment.is_empty()) + usize::from(browse.open_source)
}

fn params(browse: &State, offset: u64) -> SearchParams {
    let modrinth = browse.source == Source::Modrinth;
    let show_loader = show_loader(browse);
    SearchParams {
        query: browse.debounced.clone(),
        project_type: browse.active_type.to_string(),
        game_version: Some(browse.game_version.clone()).filter(|version| !version.is_empty()),
        loader: Some(browse.loader.clone()).filter(|loader| show_loader && !loader.is_empty()),
        sort: Some(browse.sort.to_string()),
        categories: if modrinth { browse.selected_categories.clone() } else { Vec::new() },
        environment: Some(browse.environment.to_string()).filter(|environment| modrinth && show_loader && !environment.is_empty()),
        open_source: modrinth && browse.open_source,
        offset,
        limit: LIMIT,
    }
}

/// The `$effect` on every filter: a fresh first page. Answers to older searches are dropped.
fn search(app: &mut App) {
    let browse = &mut app.browse;
    browse.loading = true;
    browse.error = None;
    browse.offset = 0;
    browse.search_gen += 1;
    let (generation, source, search_params) = (browse.search_gen, browse.source, params(browse, 0));
    app.fx.spawn(async move { crate::commands::search_content(source, search_params).await }, move |app, result| {
        let browse = &mut app.browse;
        if browse.search_gen != generation {
            return;
        }
        match result {
            Ok(page) => {
                browse.hits = page.hits.into_iter().map(Hit::from).collect();
                browse.total_hits = page.total_hits;
            }
            Err(e) => {
                browse.error = Some(e.to_string());
                browse.hits.clear();
            }
        }
        browse.loading = false;
    });
}

fn load_more(app: &mut App) {
    let browse = &mut app.browse;
    browse.loading_more = true;
    let next = browse.offset + LIMIT;
    let (generation, source, search_params) = (browse.search_gen, browse.source, params(browse, next));
    app.fx.spawn(async move { crate::commands::search_content(source, search_params).await }, move |app, result| {
        let browse = &mut app.browse;
        if browse.search_gen != generation {
            return;
        }
        match result {
            Ok(page) => {
                browse.hits.extend(page.hits.into_iter().map(Hit::from));
                browse.offset = next;
            }
            Err(e) => browse.error = Some(e.to_string()),
        }
        browse.loading_more = false;
    });
}

/// The query's 350 ms debounce: only the last keystroke's timer searches.
fn on_query_change(app: &mut App) {
    app.browse.query_gen += 1;
    let generation = app.browse.query_gen;
    app.fx.spawn(async { tokio::time::sleep(DEBOUNCE).await }, move |app, ()| {
        let browse = &mut app.browse;
        if browse.query_gen == generation && browse.query.text != browse.debounced {
            browse.debounced = browse.query.text.clone();
            search(app);
        }
    });
}

fn clear_filters(browse: &mut State) {
    browse.selected_categories.clear();
    browse.environment = "";
    browse.open_source = false;
}

fn set_view(app: &mut App, list: bool) {
    app.browse.list_view = list;
    app.prefs.write("cactus:browseView", if list { "list" } else { "grid" });
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let browse = &app.browse;

    let mut tabs = row().key("tabs").gap(4.0);
    for (label, ty) in TABS {
        let active = browse.active_type == ty;
        tabs = tabs.child(
            col()
                .key(format!("tab-{ty}"))
                .role("tab")
                .checked(active)
                .label(tr(label))
                .focusable(true)
                .cursor(Cursor::Pointer)
                .hover_group()
                .on_press(move |app: &mut App| {
                    app.browse.active_type = ty;
                    clear_filters(&mut app.browse);
                    search(app);
                })
                .child(
                    text(tr(label))
                        .font(TextStyle { weight: 500.0, ..theme.pixel(14.0) })
                        .color(if active { theme.accent } else { theme.text_secondary })
                        .group_color(if active { theme.accent } else { theme.text })
                        .px(14.0)
                        .py(10.0),
                )
                .child(el().h(3.0).w_full().fill(if active { theme.accent } else { Color::TRANSPARENT })),
        );
    }
    let tabs = col()
        .key("tabbar")
        .mb(20.0)
        .child(tabs)
        .child(el().key("tab-rule").h(1.0).w_full().mt(-1.0).fill(theme.border_subtle));

    let page = col()
        .key("browse-page")
        .w_full()
        .max_w(1200.0)
        .self_center()
        .px(32.0)
        .py(28.0)
        .child(ui::heading(&theme, tr("nav.browse"), 24.0).mb(18.0))
        .child(tabs)
        .child(toolbar(app, layer));
    let page = if browse.source == Source::Modrinth && browse.show_filters { page.child(filters_panel(app)) } else { page };
    let page = page.child(results(app, cx));

    project::view(app, cx, layer);

    col()
        .key("browse")
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(page)
}

fn toolbar(app: &App, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let browse = &app.browse;

    let source_opts = SOURCES
        .iter()
        .map(|(_, id, label)| {
            let enabled = browse.source_enabled.get(*id).copied().unwrap_or(false);
            Opt {
                value: id.to_string(),
                label: format!("{label}{}", if enabled { String::new() } else { tr("browse.unavailableSuffix") }),
                disabled: !enabled,
            }
        })
        .collect();
    let source_id = SOURCES.iter().find(|entry| entry.0 == browse.source).map_or("modrinth", |entry| entry.1);
    let source_sel = select(app, layer, "source", 160.0, source_opts, source_id, false, Some(tr("browse.contentSource")), |app, value| {
        app.browse.source = Source::from_id(&value);
        search(app);
    });

    let field = rune::text_input(&browse.query, theme.sans(13.0, 400.0))
        .single_line()
        .key("search-input")
        .label(tr("browse.searchPlaceholder"))
        .placeholder(tr("browse.searchPlaceholder"), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(|app: &mut App| &mut app.browse.query)
        .on_input_change(|app: &mut App, _| on_query_change(app))
        .grow(1.0);
    let search_box = ui::input_frame(&theme, browse.query.focused)
        .key("search")
        .grow(1.0)
        .gap(8.0)
        .child(ui::colored_icon(&icons::SEARCH, 16.0, theme.text_muted))
        .child(field);

    let mut versions = vec![Opt::new("", tr("browse.anyVersion"))];
    versions.extend(browse.game_versions.iter().map(|version| Opt::new(version.clone(), version.clone())));
    let version_sel = select(app, layer, "version", 160.0, versions, &browse.game_version, true, None, |app, value| {
        app.browse.game_version = value;
        search(app);
    });

    let mut bar = row().key("toolbar").gap(10.0).mb(22.0).items_stretch().child(source_sel).child(search_box).child(version_sel);
    if show_loader(browse) {
        let loader_opts = LOADERS.iter().map(|loader| Opt::new(*loader, if loader.is_empty() { tr("browse.anyLoader") } else { loader.to_string() })).collect();
        bar = bar.child(select(app, layer, "loader", 160.0, loader_opts, &browse.loader, false, None, |app, value| {
            app.browse.loader = value;
            search(app);
        }));
    }
    let sorts = SORTS.iter().map(|(value, key)| Opt::new(*value, tr(key))).collect();
    bar = bar.child(select(app, layer, "sort", 160.0, sorts, browse.sort, false, None, |app, value| {
        app.browse.sort = SORTS.iter().find(|entry| entry.0 == value).map_or("relevance", |entry| entry.0);
        search(app);
    }));

    if browse.source == Source::Modrinth {
        let count = active_filter_count(browse);
        let highlighted = browse.show_filters || count > 0;
        let mut btn = sunken(
            row()
                .key("filters-btn")
                .role("button")
                .label(tr("browse.filters"))
                .focusable(true)
                .cursor(Cursor::Pointer)
                .hover_group()
                .gap(7.0)
                .center_y()
                .px(14.0)
                .fill(theme.bg_input)
                .border(2.0, if highlighted { theme.accent } else { theme.border })
                .on_press(|app: &mut App| app.browse.show_filters = !app.browse.show_filters),
        )
        .child(hover_ring(theme.accent))
        .child(ui::colored_icon(&icons::FILTER, 14.0, if highlighted { theme.text } else { theme.text_secondary }).group_color(theme.text))
        .child(text(tr("browse.filters")).font(theme.sans(13.0, 600.0)).color(if highlighted { theme.text } else { theme.text_secondary }).group_color(theme.text));
        if count > 0 {
            btn = btn.child(
                text(count.to_string())
                    .font(theme.sans(11.0, 700.0))
                    .color(theme.accent_contrast)
                    .fill(theme.accent)
                    .min_w(18.0)
                    .h(18.0)
                    .px(5.0)
                    .center(),
            );
        }
        bar = bar.child(btn);
    }

    let view_button = |key: &'static str, icon: &'static rune::Icon, help: String, active: bool, list: bool| {
        el().key(key)
            .w(38.0)
            .center()
            .role("button")
            .label(help.clone())
            .help(help)
            .checked(active)
            .focusable(true)
            .cursor(Cursor::Pointer)
            .hover_group()
            .fill(if active { theme.accent } else { theme.bg_input })
            .on_press(move |app: &mut App| set_view(app, list))
            .child(ui::colored_icon(icon, 14.0, if active { theme.accent_contrast } else { theme.text_muted }).group_color(if active { theme.accent_contrast } else { theme.text }))
    };
    let toggle = sunken(
        row()
            .key("view-toggle")
            .role("group")
            .label(tr("browse.viewLabel"))
            .border(2.0, theme.border)
            .child(view_button("view-grid", &icons::GRID, tr("browse.viewGrid"), !browse.list_view, false))
            .child(el().w(2.0).fill(theme.border))
            .child(view_button("view-list", &icons::LIST, tr("browse.viewList"), browse.list_view, true)),
    );
    bar.child(toggle)
}

fn chip(theme: &crate::theme::Theme, key: String, label: String, active: bool, press: impl Fn(&mut App) + 'static) -> El {
    row()
        .key(key)
        .role("button")
        .label(label.clone())
        .checked(active)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover_group()
        .px(11.0)
        .py(5.0)
        .fill(if active { theme.accent_soft } else { theme.bg_input })
        .border(2.0, if active { theme.accent } else { theme.border })
        .on_press(move |app: &mut App| press(app))
        .child(
            text(label)
                .font(theme.sans(12.5, 400.0))
                .color(if active { theme.accent } else { theme.text_secondary })
                .group_color(if active { theme.accent } else { theme.text })
                .a11y_hidden(),
        )
        .child(hover_ring(theme.accent))
}

fn filters_panel(app: &App) -> El {
    let theme = app.theme();
    let browse = &app.browse;
    let title = |label: String| text(label.to_uppercase()).font(TextStyle { tracking: 11.0 * 0.04, ..theme.pixel(11.0) }).color(theme.text_muted);
    let mut panel = col().key("filters-panel").gap(14.0).mt(-8.0).mb(22.0).padding(16.0).fill(theme.bg_card).border(2.0, theme.border);
    if show_loader(browse) {
        let mut chips = row().gap(6.0).flex_wrap();
        for (value, key) in [("", "browse.envAny"), ("client", "browse.envClient"), ("server", "browse.envServer")] {
            chips = chips.child(chip(&theme, format!("env-{value}"), tr(key), browse.environment == value, move |app| {
                app.browse.environment = value;
                search(app);
            }));
        }
        panel = panel.child(col().key("fg-env").gap(8.0).child(title(tr("browse.environment"))).child(chips));
    }
    let mut headers: Vec<&str> = Vec::new();
    for category in browse.all_categories.iter().filter(|category| category.project_type == browse.active_type) {
        if !headers.contains(&category.header.as_str()) {
            headers.push(&category.header);
        }
    }
    for header in headers {
        let mut chips = row().gap(6.0).flex_wrap();
        for category in browse.all_categories.iter().filter(|category| category.project_type == browse.active_type && category.header == header) {
            let name = category.name.clone();
            let selected = browse.selected_categories.contains(&name);
            chips = chips.child(chip(&theme, format!("cat-{name}"), capitalize(&name), selected, move |app| {
                let selected = &mut app.browse.selected_categories;
                match selected.iter().position(|chosen| *chosen == name) {
                    Some(i) => {
                        selected.remove(i);
                    }
                    None => selected.push(name.clone()),
                }
                search(app);
            }));
        }
        panel = panel.child(col().key(format!("fg-{header}")).gap(8.0).child(title(capitalize(header))).child(chips));
    }
    let open_source = row()
        .key("open-source")
        .gap(8.0)
        .center_y()
        .role("checkbox")
        .checked(browse.open_source)
        .label(tr("browse.openSourceOnly"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| {
            app.browse.open_source = !app.browse.open_source;
            search(app);
        })
        .child(ui::checkbox(&theme, browse.open_source).a11y_hidden())
        .child(text(tr("browse.openSourceOnly")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary).a11y_hidden());
    let mut footer = row()
        .key("fg-footer")
        .justify_between()
        .center_y()
        .pt(14.0)
        .border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle)
        .child(open_source);
    if active_filter_count(browse) > 0 {
        footer = footer.child(
            col()
                .key("clear-filters")
                .role("button")
                .label(tr("browse.clearFilters"))
                .focusable(true)
                .cursor(Cursor::Pointer)
                .hover_group()
                .on_press(|app: &mut App| {
                    clear_filters(&mut app.browse);
                    search(app);
                })
                .child(text(tr("browse.clearFilters")).font(theme.sans(12.5, 400.0)).color(theme.text_muted).group_color(theme.danger))
                .child(el().h(1.0).w_full().fill(theme.text_muted).group_opacity(1.0, 0.0))
                .child(el().h(1.0).w_full().mt(-1.0).fill(theme.danger).group_opacity(0.0, 1.0)),
        );
    }
    panel.child(footer)
}

fn results(app: &App, cx: &mut Cx<App>) -> El {
    let theme = app.theme();
    let browse = &app.browse;
    if let Some(error) = &browse.error {
        let source = SOURCES.iter().find(|entry| entry.0 == browse.source).map_or_else(|| tr("browse.contentSourceFallback"), |entry| entry.2.to_string());
        return col()
            .key("status-error")
            .padding(48.0)
            .gap(10.0)
            .items_center()
            .child(text(tf("browse.cantReach", &[("source", &source)])).font(theme.body()).color(theme.danger))
            .child(text(error.clone()).font(theme.sans(12.0, 400.0)).color(theme.text_muted).max_w(480.0).wrap_anywhere())
            .child(Btn::new(tr("common.retry")).key("retry").ghost().on_press(search).build(&theme));
    }
    if !browse.loading && browse.hits.is_empty() {
        return text(tr("browse.noResults")).key("status-empty").font(theme.body()).color(theme.text_muted).padding(48.0).self_center();
    }

    let width = cx.layout_size("results").map_or(1116.0, |[w, _]| w);
    let cols = if browse.list_view { 1 } else { (((width + 14.0) / (300.0 + 14.0)).floor() as usize).max(1) };
    let card_w = (width - 14.0 * (cols as f32 - 1.0)) / cols as f32;
    let cards: Vec<El> = if browse.loading {
        (0..8).map(|i| skeleton_card(&theme, i)).collect()
    } else {
        browse.hits.iter().map(|hit| card(app, hit, browse.list_view)).collect()
    };
    let mut grid = col().key("results").w_full().gap(14.0);
    let mut remaining = cards.into_iter().peekable();
    let mut line_index = 0;
    while remaining.peek().is_some() {
        let mut line = row().key(format!("row-{line_index}")).gap(14.0).items_stretch();
        for tile in remaining.by_ref().take(cols) {
            line = line.child(tile.w(card_w));
        }
        grid = grid.child(line);
        line_index += 1;
    }
    let mut out = col().key("results-wrap").w_full().child(grid);
    if !browse.loading && (browse.hits.len() as u64) < browse.total_hits {
        out = out.child(
            row().key("more").justify_center().my(24.0).child(
                Btn::new(if browse.loading_more { tr("common.loading") } else { tr("browse.loadMore") })
                    .key("load-more")
                    .ghost()
                    .disabled(browse.loading_more)
                    .on_press(load_more)
                    .build(&theme),
            ),
        );
    }
    out
}

fn card_frame(theme: &crate::theme::Theme, key: String) -> El {
    ui::bevel(row().key(key).gap(14.0).padding(14.0).fill(theme.bg_card).border(2.0, theme.border), 2.0, 0.04, 0.28)
}

fn card(app: &App, hit: &Hit, list: bool) -> El {
    let theme = app.theme();
    let icon = match hit.icon_url.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => ui::image(app, url, 52.0, 52.0).fill(theme.bg_input),
        None => el().size(52.0, 52.0).center().fill(theme.bg_input).child(ui::colored_icon(&icons::PACKAGE, 24.0, theme.text_muted)),
    };
    let icon = el().size(56.0, 56.0).border(2.0, Color::BLACK.alpha(0.3)).child(icon);
    let to_open = hit.clone();
    let body = col()
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(4.0)
        .child(
            row()
                .gap(8.0)
                .items_end()
                .min_w(0.0)
                .child(text(hit.title.clone()).font(theme.sans(14.0, 600.0)).color(theme.text).shrink().truncate())
                .child(text(tf("browse.byAuthor", &[("author", &hit.author)])).font(theme.sans(11.5, 400.0)).color(theme.text_muted).pb(1.0)),
        )
        .child(
            text(hit.description.clone())
                .font(TextStyle { line_height: 18.0, ..theme.sans(12.5, 400.0) })
                .color(theme.text_secondary)
                .max_lines(if list { 3 } else { 2 }),
        )
        .child(el().grow(1.0))
        .child(
            row()
                .gap(4.0)
                .center_y()
                .child(ui::colored_icon(&icons::PACKAGE, 12.0, theme.text_muted))
                .child(text(fmt::count(hit.downloads)).font(theme.sans(11.5, 400.0)).color(theme.text_muted)),
        );
    card_frame(&theme, format!("card-{}", hit.project_id))
        .role("button")
        .label(hit.title.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover_group()
        .hover(theme.bg_hover.alpha(0.6))
        .on_press(move |app: &mut App| project::open(app, to_open.clone()))
        .child(icon)
        .child(body)
        .child(hover_ring(theme.accent))
}

fn skeleton_card(theme: &crate::theme::Theme, i: usize) -> El {
    card_frame(theme, format!("skel-{i}"))
        .child(ui::skeleton(theme, Some(56.0), 56.0))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(4.0)
                .child(el().w_frac(0.55).child(ui::skeleton(theme, None, 14.0)))
                .child(ui::skeleton(theme, None, 11.0))
                .child(el().w_frac(0.75).child(ui::skeleton(theme, None, 11.0)))
                .child(el().grow(1.0))
                .child(ui::skeleton(theme, Some(44.0), 11.0)),
        )
}
