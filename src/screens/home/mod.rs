//! Home: `src/routes/+page.svelte`, HomeGrid, InstanceCard, InstanceCardSkeleton, NewsSection,
//! GroupContextMenu.

mod card;
mod grid;
pub use grid::save_covers;
mod group_menu;
mod news;

use std::collections::BTreeMap;

use rune::{Cursor, Cx, El, FileDialogOptions, PointerPress, ScrollSpec, Tx, col, el, row, text};
use rune_kit::{Menu, MenuItem, Select, SelectAppearance};

use crate::app::{App, Ignore, Route};
use crate::core::i18n::t as tr;
use crate::icons;
use crate::instance::{Instance, InstanceKind, ModLoader};
use crate::theme::Theme;
use crate::ui::{self, Btn, Variant};

/// Used by the folder overlay.
#[allow(unused_imports)]
pub use card::instance_card;
use grid::{Cell, DragState, Entry, ResizeState};


#[derive(Default)]
pub struct State {
    pub query: rune::TextEdit,
    /// None is "all".
    pub loader: Option<ModLoader>,
    pub loader_open: bool,
    pub arranging: bool,
    /// The page's right-click / Import menu, at a window point.
    pub menu: Option<(f32, f32)>,
    pub code_open: bool,
    pub code: rune::TextEdit,
    pub importing: bool,
    pub layout: BTreeMap<String, Cell>,
    pub covers: BTreeMap<String, String>,
    pub drag: Option<DragState>,
    pub resize: Option<ResizeState>,
    /// A drag just ended on a tile: the click that follows it doesn't open anything.
    pub suppress_click: bool,
    pub news: news::State,
}

pub fn on_enter(app: &mut App) {
    app.home.layout = grid::load_layout(&app.prefs);
    app.home.covers = app.prefs.read(grid::COVERS_KEY, BTreeMap::new());
    app.home.drag = None;
    app.home.resize = None;
    if app.settings.show_news && !app.home.news.started {
        news::load(app, false);
    }
}

pub fn on_event(_app: &mut App, _name: &str, _payload: &serde_json::Value) {}

fn filters_active(app: &App) -> bool {
    !app.home.query.text.trim().is_empty() || app.home.loader.is_some()
}

fn arranging(app: &App) -> bool {
    app.home.arranging && !filters_active(app)
}

fn online() -> bool {
    !crate::commands::backend_base().is_empty()
}

fn filtered(app: &App) -> Vec<&Instance> {
    let query = app.home.query.text.trim().to_string();
    let query_lower = query.to_lowercase();
    app.instances
        .iter()
        .filter(|inst| query.is_empty() || inst.name.to_lowercase().contains(&query_lower) || inst.mc_version.contains(&query))
        .filter(|inst| app.home.loader.is_none_or(|loader| inst.loader == loader))
        .collect()
}

fn entries<'a>(list: &[&'a Instance]) -> Vec<Entry<'a>> {
    let mut groups: BTreeMap<String, Vec<&'a Instance>> = BTreeMap::new();
    let mut out = Vec::new();
    for &inst in list {
        match inst.group.as_deref().filter(|group| !group.is_empty()) {
            Some(group) => groups.entry(group.to_string()).or_default().push(inst),
            None => out.push(Entry::Instance(inst)),
        }
    }
    out.extend(groups.into_iter().map(|(name, instances)| Entry::Folder { name, instances }));
    out
}

fn play_random(app: &mut App) {
    let clients: Vec<&Instance> = app.instances.iter().filter(|inst| inst.kind != InstanceKind::Server).collect();
    if clients.is_empty() {
        return;
    }
    let idle: Vec<&Instance> = clients.iter().copied().filter(|inst| !app.is_running(&inst.id) && !app.is_busy(&inst.id)).collect();
    let pool = if idle.is_empty() { &clients } else { &idle };
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.subsec_nanos());
    let id = pool[nanos as usize % pool.len()].id.clone();
    app.go(Route::Instance(id.clone()));
    app.launch_instance(&id);
}

fn import_setup_file(_app: &mut App, tx: &mut Tx) {
    let opts = FileDialogOptions::default().filter("Setup", &["cactuspack", "drakepack", "mrpack", "zip"]);
    tx.open_file(opts, |app: &mut App, result, _| {
        let Ok(Some(path)) = result else { return };
        app.home.importing = true;
        let host = app.fx.host.clone();
        app.fx.spawn(
            async move {
                let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
                crate::commands::import_setup(host, bytes).await.map_err(|e| e.to_string())
            },
            |app, result| {
                app.home.importing = false;
                match result {
                    Ok(imported) => {
                        app.refresh_instances();
                        app.go(Route::Instance(imported.instance.id));
                    }
                    Err(e) => app.toasts.error(e).ignore(),
                }
            },
        );
    });
}

fn import_from_code(app: &mut App) {
    let code = app.home.code.text.trim().to_string();
    if code.is_empty() || app.home.importing {
        return;
    }
    app.home.importing = true;
    crate::screens::share::import_from_code(app, code, |app, result| {
        app.home.importing = false;
        match result {
            Ok(()) => {
                app.home.code_open = false;
                app.home.code = rune::TextEdit::default();
            }
            Err(e) => app.toasts.error(e).ignore(),
        }
    });
}

fn open_menu_at(app: &mut App, press: PointerPress) {
    app.home.menu = Some((press.x, press.y));
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let width = cx.layout_size("home-page").map_or(1180.0 - 64.0, |[w, _]| w - 64.0);
    let arranging = arranging(app);

    let mut page = col().key("home-page").w_full().max_w(1200.0).px(32.0).py(28.0);
    page = page.child(hero(&theme));
    if let Some(news_section) = news::view(app, width) {
        page = page.child(news_section);
    }

    let has_instances = !app.instances.is_empty();
    if has_instances {
        page = page.child(toolbar(app, &theme, layer, arranging));
        if arranging {
            page = page.child(arrange_hint(&theme));
        }
    }

    if !app.instances_loaded {
        let mut skeletons = row().key("skeletons").flex_wrap().gaps(16.0, 16.0);
        let cols = (((width + 16.0) / (160.0 + 16.0)).floor() as usize).max(1);
        let w = (width - 16.0 * (cols as f32 - 1.0)) / cols as f32;
        for i in 0..6 {
            skeletons = skeletons.child(card::skeleton_card(app, i, w, 160.0));
        }
        page = page.child(skeletons);
    } else if !has_instances {
        page = page.child(empty(app, &theme));
    } else {
        let list = filtered(app);
        if list.is_empty() {
            page = page.child(ui::muted(&theme, tr("home.noMatchFilters"), 14.0).key("no-match"));
        } else {
            page = page.child(grid::view(app, &entries(&list), width, arranging));
        }
    }

    home_menu(app, layer);
    group_menu::view(app, layer);
    code_modal(app, &theme, layer);
    if app.home.importing {
        layer.push(
            row().key("importing-toast").w_full().absolute(0.0, 0.0).pin(Some(0.0), None, Some(0.0), Some(100.0)).justify_center().child(
                el().px(16.0).py(10.0).fill(theme.bg_raised).border(2.0, theme.accent).shadow(4.0, 12.0, theme.shadow).role("status")
                    .child(text(tr("common.importing")).font(theme.sans(13.0, 400.0)).color(theme.accent)),
            ),
        );
    }

    col().key("home")
        .items_center()
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .on_secondary_press(|app: &mut App, press: PointerPress, _: &mut Tx| open_menu_at(app, press))
        .child(page)
}

fn hero(theme: &Theme) -> El {
    let cmdk = row()
        .key("cmdk")
        .gap(8.0)
        .center_y()
        .px(10.0)
        .py(7.0)
        .min_w(190.0)
        .fill(theme.bg_input)
        .border(2.0, theme.border)
        .hover_group()
        .role("button")
        .label(tr("home.commandPalette"))
        .help(tr("home.commandPalette"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.ui.command_palette_open = !app.ui.command_palette_open)
        .child(ui::colored_icon(&icons::SEARCH, 14.0, theme.text_muted).group_color(theme.text))
        .child(text(tr("home.search")).font(theme.sans(13.0, 400.0)).color(theme.text_muted).group_color(theme.text).grow(1.0))
        .child(
            el().px(6.0).py(2.0).fill(theme.bg_card).border(1.0, theme.border)
                .child(text(if cfg!(target_os = "macos") { "⌘K" } else { "Ctrl K" }).font(theme.pixel(10.5)).color(theme.text_secondary)),
        )
        .child(el().key("ring").absolute(-2.0, -2.0).pin(Some(-2.0), Some(-2.0), Some(-2.0), Some(-2.0)).border(2.0, theme.accent).group_opacity(0.0, 1.0).a11y_hidden());

    let import = Btn::new(tr("common.import"))
        .key("import")
        .ghost()
        .icon(&icons::DOWNLOAD)
        .help(tr("home.importShared"))
        .on_press_tx(|app: &mut App, _tx: &mut Tx| {
            let anchor = rune::memo::window_layout("import").map(|rect| (rect[0], rect[1] + rect[3] + 4.0));
            app.home.menu = Some(anchor.unwrap_or((900.0, 80.0)));
        })
        .build(theme);
    let new = Btn::new(tr("home.newInstance"))
        .key("new-instance")
        .primary()
        .icon(&icons::PLUS)
        .on_press(|app: &mut App| app.ui.create_instance_open = true)
        .build(theme);

    row()
        .key("hero")
        .justify_between()
        .items_start()
        .gap(16.0)
        .mb(24.0)
        .child(
            col().shrink().min_w(0.0)
                .child(ui::heading(theme, tr("home.welcomeBack"), 26.0))
                .child(text(tr("home.subtitle")).font(theme.body()).color(theme.text_secondary).mt(6.0).wrap()),
        )
        .child(row().gap(10.0).center_y().child(cmdk).child(import).child(new))
}

fn loader_value(loader: Option<ModLoader>) -> String {
    loader.map_or_else(|| tr("home.allLoaders"), |loader| loader.label().to_string())
}

fn toolbar(app: &App, theme: &Theme, layer: &mut Vec<El>, arranging: bool) -> El {
    let search = ui::input_frame(theme, app.home.query.focused)
        .key("search")
        .grow(1.0)
        .gap(8.0)
        .child(ui::colored_icon(&icons::SEARCH, 16.0, theme.text_muted))
        .child(
            rune::text_input(&app.home.query, theme.sans(13.0, 400.0))
                .key("search-input")
                .label(tr("home.searchInstances"))
                .single_line()
                .placeholder(tr("home.searchInstances"), theme.text_muted)
                .color(theme.text)
                .selection_color(theme.accent.alpha(0.6))
                .on_edit(|app: &mut App| &mut app.home.query)
                .grow(1.0),
        );

    let mut options = vec![tr("home.allLoaders")];
    options.extend(ModLoader::ALL.iter().map(|loader| loader.label().to_string()));
    let kit = theme.kit();
    let current = loader_value(app.home.loader);
    let select = Select::new("loader-filter", &options)
        .width(160.0)
        .appearance(SelectAppearance::Field)
        .label(tr("home.allLoaders"))
        .current(Some(&current))
        .open(app.home.loader_open)
        .on_toggle(|app: &mut App| app.home.loader_open = !app.home.loader_open)
        .on_close(|app: &mut App| app.home.loader_open = false)
        .on_choose(|app: &mut App, choice: &str, _: &mut Tx| {
            app.home.loader_open = false;
            app.home.loader = ModLoader::ALL.into_iter().find(|loader| loader.label() == choice);
        })
        .build(&kit, layer);

    let filters = filters_active(app);
    let arrange = Btn::new(if arranging { tr("common.done") } else { tr("home.arrange") })
        .key("arrange")
        .variant(if arranging { Variant::Primary } else { Variant::Ghost })
        .icon(if arranging { &icons::CHECK } else { &icons::EXPAND })
        .disabled(filters)
        .help(if filters { tr("home.clearFiltersToRearrange") } else { tr("home.dragResizeTiles") })
        .on_press(|app: &mut App| app.home.arranging = !app.home.arranging)
        .build(theme);

    row().key("toolbar").gap(12.0).center_y().mb(18.0).child(search).child(select).child(arrange)
}

fn arrange_hint(theme: &Theme) -> El {
    let small = |label: String| text(label).font(theme.sans(12.5, 400.0)).color(theme.text_muted);
    let reset = row()
        .key("reset-layout")
        .gap(5.0)
        .center_y()
        .px(10.0)
        .py(5.0)
        .border(2.0, theme.border)
        .hover_group()
        .role("button")
        .label(tr("home.resetLayout"))
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(grid::reset_layout)
        .child(ui::colored_icon(&icons::REFRESH, 12.0, theme.text_secondary).group_color(theme.danger))
        .child(text(tr("home.resetLayout")).font(theme.sans(12.0, 600.0)).color(theme.text_secondary).group_color(theme.danger))
        .child(el().key("ring").absolute(-2.0, -2.0).pin(Some(-2.0), Some(-2.0), Some(-2.0), Some(-2.0)).border(2.0, theme.danger).group_opacity(0.0, 1.0).a11y_hidden());
    row()
        .key("arrange-hint")
        .justify_between()
        .center_y()
        .gap(12.0)
        .mt(-6.0)
        .mb(16.0)
        .child(
            row().gap(5.0).center_y().shrink().min_w(0.0)
                .child(small(tr("home.arrangeHint")))
                .child(ui::colored_icon(&icons::EXPAND, 12.0, theme.text_muted))
                .child(small(tr("home.arrangeHintResize"))),
        )
        .child(reset)
}

fn empty(app: &App, theme: &Theme) -> El {
    col()
        .key("empty")
        .w_full()
        .items_center()
        .gap(12.0)
        .px(24.0)
        .py(72.0)
        .child(ui::image(app, "/empty-cactus.png", 240.0, 144.0).mb(4.0))
        .child(ui::heading(theme, tr("home.noInstancesYet"), 18.0))
        .child(text(tr("home.createFirst")).font(theme.body()).color(theme.text_secondary).wrap().align(rune::Align::Center).max_w(340.0).mb(8.0))
        .child(
            Btn::new(tr("home.createInstance"))
                .key("create-first")
                .primary()
                .icon(&icons::PLUS)
                .on_press(|app: &mut App| app.ui.create_instance_open = true)
                .build(theme),
        )
}

fn close_menu(app: &mut App) {
    app.home.menu = None;
}

fn home_menu(app: &App, layer: &mut Vec<El>) {
    let Some((x, y)) = app.home.menu else { return };
    let kit = app.theme().kit();
    let mut menu = Menu::new("home-menu").standard().min_width(260.0).trigger(el()).open(true).at(x, y).on_close(close_menu);
    if app.instances.iter().any(|inst| inst.kind != InstanceKind::Server) {
        menu = menu
            .item(MenuItem::new(tr("home.playRandom")).icon(&icons::SHUFFLE).on_select(|app: &mut App| {
                close_menu(app);
                play_random(app);
            }))
            .separator();
    }
    menu = menu
        .item(MenuItem::new(tr("home.newInstance")).icon(&icons::PLUS).on_select(|app: &mut App| {
            close_menu(app);
            app.ui.create_instance_open = true;
        }))
        .separator()
        .item(MenuItem::new(tr("home.importSetupFile")).icon(&icons::DOWNLOAD).on_select_tx(|app: &mut App, tx: &mut Tx| {
            close_menu(app);
            import_setup_file(app, tx);
        }))
        .item(MenuItem::new(tr("home.importFromLauncher")).icon(&icons::SHUFFLE).on_select(|app: &mut App| {
            close_menu(app);
            app.ui.launcher_import_open = true;
        }));
    if online() {
        menu = menu.item(MenuItem::new(tr("home.importFromCode")).icon(&icons::SHARE).on_select(|app: &mut App| {
            close_menu(app);
            app.home.code_open = true;
            app.home.code.focused = true;
        }));
    }
    let _ = menu.build(&kit, layer);
}

fn code_modal(app: &App, theme: &Theme, layer: &mut Vec<El>) {
    if !app.home.code_open {
        return;
    }
    let close = |app: &mut App| app.home.code_open = false;
    let field = ui::input_frame(theme, app.home.code.focused).key("code-field").child(
        rune::text_input(&app.home.code, theme.sans(13.0, 400.0))
            .key("code-input")
            .label(tr("home.pasteShareCode"))
            .single_line()
            .placeholder(tr("home.pasteShareCode"), theme.text_muted)
            .color(theme.text)
            .selection_color(theme.accent.alpha(0.6))
            .on_edit(|app: &mut App| &mut app.home.code)
            .on_submit(|app: &mut App, _: &mut Tx| import_from_code(app))
            .grow(1.0),
    );
    let busy = app.home.importing;
    let footer = ui::buttons([
        Btn::new(tr("common.cancel")).key("code-cancel").ghost().on_press(close).build(theme),
        Btn::new(if busy { tr("common.importing") } else { tr("common.import") })
            .key("code-import")
            .primary()
            .disabled(busy || app.home.code.text.trim().is_empty())
            .on_press(import_from_code)
            .build(theme),
    ]);
    layer.push(ui::modal(theme, "code-modal", tr("home.importFromCodeTitle"), 380.0, field, Some(footer), close));
}
