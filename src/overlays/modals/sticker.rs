//! `StickerPicker.svelte`: an image for an instance or a group cover, from the Cactus decor
//! sprites, Giphy stickers or emoji.

use rune::{Color, Cursor, El, TextEdit, Tx, col, el, row, text};

use super::image::{emoji_to_data_uri, sprite_to_square_icon};
use super::widgets::boxed;
use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::stickers::Sticker;
use crate::theme::Theme;
use crate::ui::{self, Btn};

/// The backend's Giphy page size.
const LIMIT: usize = 30;
const COLS: usize = 3;
/// The body's inner width (480 less border and padding).
const BODY_W: f32 = 440.0;

/// `DECOR_IDS` in decor.ts.
const DECOR_IDS: [u32; 31] = [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 26, 30, 31, 32, 33, 34, 56, 57, 64];

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Decor,
    Stickers,
    Emoji,
}

#[derive(Default)]
pub struct State {
    tab: Tab,
    key_draft: TextEdit,
    saving_key: bool,
    editing_key: bool,
    query: TextEdit,
    /// The query the debounce timer last started for, and the one whose results are shown.
    debouncing: Option<String>,
    searched: Option<String>,
    stickers: Vec<Sticker>,
    loading: bool,
    loading_more: bool,
    has_more: bool,
    offset: usize,
    error: Option<String>,
    applying: bool,
    req: u64,
}

fn close(app: &mut App) {
    app.ui.sticker_picker = None;
    app.modals.sticker = State::default();
}

fn enabled(app: &App) -> bool {
    !app.settings.giphy_api_key.trim().is_empty()
}

/// Hand the data URI to whoever opened the picker, then close it.
fn pick(app: &mut App, uri: String) {
    if let Some((_, on_pick)) = app.ui.sticker_picker.clone() {
        on_pick(app, uri);
    }
    close(app);
}

fn choose_decor(app: &mut App, url: String) {
    if app.modals.sticker.applying {
        return;
    }
    app.modals.sticker.applying = true;
    app.fx.blocking(move |_| sprite_to_square_icon(&url, 128), |app, result| {
        app.modals.sticker.applying = false;
        match result {
            Ok(uri) => pick(app, uri),
            Err(e) => app.modals.sticker.error = Some(e),
        }
    });
}

fn choose_emoji(app: &mut App, emoji: &'static str) {
    if app.modals.sticker.applying {
        return;
    }
    app.modals.sticker.applying = true;
    app.fx.blocking(move |_| emoji_to_data_uri(emoji, 128), |app, result| {
        app.modals.sticker.applying = false;
        match result {
            Ok(uri) => pick(app, uri),
            Err(e) => {
                app.toasts.error(e);
            }
        }
    });
}

fn choose_sticker(app: &mut App, full: String) {
    if app.modals.sticker.applying {
        return;
    }
    app.modals.sticker.applying = true;
    app.fx.spawn(async move { crate::commands::download_image(full).await }, |app, result| {
        app.modals.sticker.applying = false;
        match result {
            Ok(uri) => pick(app, uri),
            Err(e) => app.modals.sticker.error = Some(e.to_string()),
        }
    });
}

fn save_key(app: &mut App) {
    let key = app.modals.sticker.key_draft.text.trim().to_string();
    if key.is_empty() {
        return;
    }
    app.modals.sticker.saving_key = true;
    app.update_settings(|settings| settings.giphy_api_key = key);
    let picker = &mut app.modals.sticker;
    picker.saving_key = false;
    picker.key_draft = TextEdit::default();
    picker.editing_key = false;
    picker.error = None;
    picker.searched = None;
    picker.tab = Tab::Stickers;
}

/// The 350 ms debounce on the search field: the search runs once the text has held still.
fn debounce(app: &mut App) {
    let query = app.modals.sticker.query.text.clone();
    if app.modals.sticker.debouncing.as_deref() == Some(&query) {
        return;
    }
    app.modals.sticker.debouncing = Some(query.clone());
    app.fx.after(std::time::Duration::from_millis(350), move |app| {
        if app.ui.sticker_picker.is_some() && app.modals.sticker.query.text == query && app.modals.sticker.searched.as_deref() != Some(&query) {
            reset_and_load(app, query);
        }
    });
}

fn reset_and_load(app: &mut App, query: String) {
    let picker = &mut app.modals.sticker;
    picker.searched = Some(query.clone());
    picker.loading = true;
    picker.error = None;
    picker.offset = 0;
    picker.has_more = true;
    picker.req += 1;
    let req = picker.req;
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::search_stickers(host.state(), query, 0).await }, move |app, result| {
        let picker = &mut app.modals.sticker;
        if picker.req != req {
            return;
        }
        picker.loading = false;
        match result {
            Ok(list) => {
                picker.offset = list.len();
                picker.has_more = list.len() >= LIMIT;
                picker.stickers = list;
            }
            Err(e) => {
                picker.error = Some(e.to_string());
                picker.stickers.clear();
            }
        }
    });
}

fn load_more(app: &mut App) {
    let picker = &mut app.modals.sticker;
    if picker.loading || picker.loading_more || !picker.has_more {
        return;
    }
    picker.loading_more = true;
    let (req, offset, query) = (picker.req, picker.offset, picker.searched.clone().unwrap_or_default());
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::search_stickers(host.state(), query, offset as u32).await }, move |app, result| {
        let picker = &mut app.modals.sticker;
        if picker.req != req {
            return;
        }
        picker.loading_more = false;
        match result {
            Ok(list) => {
                picker.offset += list.len();
                picker.has_more = list.len() >= LIMIT;
                for sticker in list {
                    if !picker.stickers.iter().any(|existing| existing.id == sticker.id) {
                        picker.stickers.push(sticker);
                    }
                }
            }
            Err(_) => picker.has_more = false,
        }
    });
}

pub fn view(app: &App, layer: &mut Vec<El>) {
    let Some((title, _)) = &app.ui.sticker_picker else { return };
    let theme = app.theme();
    let picker = &app.modals.sticker;
    if picker.tab == Tab::Stickers && enabled(app) && !picker.editing_key && picker.debouncing.as_deref() != Some(picker.query.text.as_str()) {
        app.fx.post(debounce);
    }
    let tab = |tab_kind: Tab, key: &str, label: String, icon: Option<&'static rune::Icon>| {
        let on = picker.tab == tab_kind;
        let ink = if on { theme.accent } else { theme.text_secondary };
        let mut face = row()
            .key(key.to_string())
            .role("tab")
            .label(label.clone())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .gap(6.0)
            .center_y()
            .px(12.0)
            .pt(8.0)
            .pb(8.0)
            .border_sides([0.0, 0.0, 3.0, 0.0], if on { theme.accent } else { Color::TRANSPARENT })
            .hover_group()
            .on_press(move |app: &mut App| app.modals.sticker.tab = tab_kind);
        if let Some(icon) = icon {
            face = face.child(if on { ui::colored_icon(icon, 14.0, ink) } else { ui::colored_icon(icon, 14.0, ink).group_color(theme.text) });
        }
        let label = text(label).font(rune::tok::TextStyle { weight: 500.0, ..theme.pixel(13.0) }).color(ink);
        face.child(if on { label } else { label.group_color(theme.text) })
    };
    let tabs = row()
        .key("tabs")
        .role("tablist")
        .gap(4.0)
        .mb(16.0)
        .border_sides([0.0, 0.0, 1.0, 0.0], theme.border_subtle)
        .child(tab(Tab::Decor, "tab-decor", "Cactus".into(), None))
        .child(tab(Tab::Stickers, "tab-stickers", tr("sticker.stickers"), Some(&icons::SPARKLES)))
        .child(tab(Tab::Emoji, "tab-emoji", tr("sticker.emoji"), None));
    let content = match picker.tab {
        Tab::Decor => decor_grid(app, &theme, picker.applying),
        Tab::Stickers if !enabled(app) || picker.editing_key => key_notice(&theme, picker),
        Tab::Stickers => stickers(app, &theme, picker),
        Tab::Emoji => emoji(&theme),
    };
    let body = col().key("sticker-picker").w(BODY_W).child(tabs).child(content);
    layer.push(ui::modal(&theme, "sticker-modal", title.clone(), 480.0, body, None, close));
}

fn decor_grid(app: &App, theme: &Theme, busy: bool) -> El {
    let cell = (BODY_W - 8.0 * 4.0) / 5.0;
    let mut grid = col().key("decor").gap(8.0);
    for chunk in DECOR_IDS.chunks(5) {
        let mut line = row().gap(8.0);
        for id in chunk {
            let url = format!("/decor/sprites/{id:02}.png");
            let img = sprite(app, &url, cell - 16.0);
            let face = el()
                .key(format!("decor-{id}"))
                .size(cell, cell)
                .center()
                .fill(theme.bg_input)
                .role("button")
                .label(tr("sticker.decorationAlt"))
                .focusable(true)
                .cursor(Cursor::Pointer)
                .press_scale(0.96)
                .on_press(move |app: &mut App| choose_decor(app, url.clone()))
                .child(img);
            line = line.child(boxed(theme, face, false));
        }
        grid = grid.child(line);
    }
    let scroller = grid.max_h(360.0).shrink().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false });
    if busy { scroller.opacity(0.5).disabled(true) } else { scroller }
}

/// `object-fit: contain` in a `max`-point square.
fn sprite(app: &App, src: &str, max: f32) -> El {
    let Some(img) = app.images.get(&app.fx, src) else { return el().size(max, max) };
    match img.size() {
        Some([w, h]) => {
            let fit = (max / w as f32).min(max / h as f32);
            el().size(w as f32 * fit, h as f32 * fit).pixels(img.pixels())
        }
        None => el().size(max, max).pixels(img.pixels()),
    }
}

fn notice(theme: &Theme, error: bool) -> El {
    col().padding(14.0).gap(6.0).fill(theme.bg_input).border(2.0, if error { theme.danger } else { theme.border })
}

fn key_notice(theme: &Theme, picker: &State) -> El {
    let prompt = row()
        .flex_wrap()
        .child(text(format!("{} ", tr("sticker.giphyPromptBefore"))).font(theme.sans(13.0, 400.0)).color(theme.text_secondary))
        .child(
            rune::rich("developers.giphy.com", vec![rune::Span::new(0, 20).underline(theme.accent)])
                .font(theme.sans(13.0, 400.0))
                .color(theme.accent)
                .key("giphy-link")
                .role("link")
                .cursor(Cursor::Pointer)
                .on_press(|_: &mut App| {
                    let _ = open::that("https://developers.giphy.com");
                }),
        )
        .child(text(tr("sticker.giphyPromptAfter")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary));
    let input = rune::text_input(&picker.key_draft, theme.sans(13.0, 400.0))
        .single_line()
        .secure()
        .placeholder(tr("sticker.keyPlaceholder"), theme.text_muted)
        .color(theme.text)
        .on_edit(|app: &mut App| &mut app.modals.sticker.key_draft)
        .on_submit(|app: &mut App, _: &mut Tx| save_key(app))
        .grow(1.0);
    let mut key_row = row()
        .key("key-row")
        .gap(8.0)
        .mt(4.0)
        .mb(4.0)
        .center_y()
        .child(
            row()
                .grow(1.0)
                .min_w(0.0)
                .px(10.0)
                .py(8.0)
                .fill(rune::color::hex("#17161a").unwrap_or(theme.bg_input))
                .border(2.0, if picker.key_draft.focused { theme.accent } else { theme.border })
                .child(input),
        )
        .child(
            Btn::new(if picker.saving_key { tr("sticker.saving") } else { tr("common.save") })
                .primary()
                .sm()
                .key("save-key")
                .disabled(picker.saving_key || picker.key_draft.text.trim().is_empty())
                .on_press(save_key)
                .build(theme),
        );
    if picker.editing_key {
        key_row = key_row.child(Btn::new(tr("common.cancel")).ghost().sm().key("cancel-key").on_press(|app| app.modals.sticker.editing_key = false).build(theme));
    }
    notice(theme, false)
        .key("key-notice")
        .child(
            text(if picker.editing_key { tr("sticker.updateKey") } else { tr("sticker.stickersOff") })
                .font(theme.sans(13.0, 700.0))
                .color(theme.text_secondary),
        )
        .child(prompt)
        .child(key_row)
        .child(text(tr("sticker.changeLater")).font(theme.sans(11.5, 400.0)).color(theme.text_muted).wrap())
}

fn stickers(app: &App, theme: &Theme, picker: &State) -> El {
    let search = row()
        .key("search")
        .gap(10.0)
        .center_y()
        .px(12.0)
        .py(9.0)
        .mb(12.0)
        .fill(theme.bg_input)
        .border(2.0, if picker.query.focused { theme.accent } else { theme.border })
        .child(ui::colored_icon(&icons::SEARCH, 16.0, theme.text_muted))
        .child(
            rune::text_input(&picker.query, theme.sans(13.0, 400.0))
                .single_line()
                .placeholder(tr("sticker.searchPlaceholder"), theme.text_muted)
                .color(theme.text)
                .on_edit(|app: &mut App| &mut app.modals.sticker.query)
                .grow(1.0),
        );
    let col_w = (BODY_W - 8.0 * (COLS as f32 - 1.0)) / COLS as f32;
    let skeleton_h = |i: usize| match i % 3 {
        2 => 128.0,
        1 => 72.0,
        _ => 96.0,
    };
    let grid = if picker.loading || picker.searched.is_none() {
        let mut skeletons = row().key("grid").gap(8.0).items_start();
        for c in 0..COLS {
            let mut column = col().gap(8.0).w(col_w);
            for r in 0..4 {
                column = column.child(ui::skeleton(theme, Some(col_w), skeleton_h(r + c + 1)));
            }
            skeletons = skeletons.child(column);
        }
        skeletons
    } else if let Some(error) = &picker.error {
        notice(theme, true).key("error").child(text(error.clone()).font(theme.sans(13.0, 400.0)).color(theme.danger).wrap()).child(
            row().mt(8.0).child(
                Btn::new(tr("sticker.changeKey"))
                    .ghost()
                    .sm()
                    .key("change-key")
                    .on_press(|app| {
                        app.modals.sticker.key_draft = TextEdit::default();
                        app.modals.sticker.editing_key = true;
                    })
                    .build(theme),
            ),
        )
    } else if picker.stickers.is_empty() {
        el().key("none").padding(24.0).center().child(text(tr("sticker.noStickers")).font(theme.body()).color(theme.text_muted))
    } else {
        let cols = (0..COLS).map(|column| {
            let cells = picker.stickers.iter().skip(column).step_by(COLS).map(|sticker| sticker_cell(app, theme, sticker, col_w));
            col().gap(8.0).w(col_w).children(cells).children(picker.loading_more.then(|| ui::skeleton(theme, Some(col_w), 96.0)))
        });
        let mut scroller = col()
            .key("grid")
            .child(row().gap(8.0).items_start().children(cols))
            .max_h(340.0)
            .shrink()
            .min_h(0.0)
            .scroll(rune::ScrollSpec { stick_bottom: false });
        if picker.has_more && !picker.loading_more {
            scroller = scroller.on_far_from_bottom(140.0, |app: &mut App, far| {
                if !far {
                    load_more(app);
                }
            });
        }
        if picker.applying { scroller.opacity(0.5).disabled(true) } else { scroller }
    };
    col().key("stickers").child(search).child(grid).child(giphy_mark(theme))
}

fn sticker_cell(app: &App, theme: &Theme, sticker: &Sticker, w: f32) -> El {
    let inner = w - 4.0;
    let img = match app.images.get(&app.fx, &sticker.preview) {
        Some(img) => match img.size() {
            Some([iw, ih]) => el().size(inner, inner * ih as f32 / iw.max(1) as f32).pixels(img.pixels()),
            None => el().size(inner, 96.0).pixels(img.pixels()),
        },
        None => ui::skeleton(theme, Some(inner), 96.0),
    };
    let full = sticker.full.clone();
    let face = el()
        .key(format!("s-{}", sticker.id))
        .padding(2.0)
        .fill(theme.bg_input)
        .role("button")
        .label(tr("sticker.stickerAlt"))
        .help(tr("sticker.useSticker"))
        .cursor(Cursor::Pointer)
        .press_scale(0.97)
        .on_press(move |app: &mut App| choose_sticker(app, full.clone()))
        .child(img);
    boxed(theme, face, false)
}

/// The required "Powered By GIPHY" mark; the wordmark's gradient is sampled per letter.
fn giphy_mark(theme: &Theme) -> El {
    let stops = ["#ff6666", "#cc4db3", "#9933ff", "#4d99cc", "#00ff99"];
    let mut word = row();
    for (ch, hex) in "GIPHY".chars().zip(stops) {
        word = word.child(
            text(ch.to_string())
                .font(rune::tok::TextStyle { weight: 900.0, tracking: 0.26, ..theme.sans(13.0, 900.0) })
                .color(rune::color::hex(hex).unwrap_or(theme.text)),
        );
    }
    row()
        .key("giphy-mark")
        .help("Powered By GIPHY")
        .self_start()
        .gap(6.0)
        .center_y()
        .mt(12.0)
        .px(9.0)
        .py(5.0)
        .fill(Color::BLACK)
        .radius(3.0)
        .child(text("POWERED BY").font(rune::tok::TextStyle { tracking: 0.72, ..theme.sans(9.0, 600.0) }).color(rune::color::hex("#b7b7b7").unwrap_or(theme.text_muted)))
        .child(word)
}

/// `GROUPS`: four rows of sixteen.
const GROUPS: [(&str, [&str; 16]); 4] = [
    ("sticker.groupBlocks", ["⛏️", "🗡️", "🪓", "⚒️", "🛡️", "🏹", "🧨", "💎", "💠", "🧱", "🗺️", "🧭", "🔥", "💧", "🌳", "🌸"]),
    ("sticker.groupMobs", ["🐷", "🐑", "🐮", "🐔", "🐺", "🐝", "🦊", "🐢", "🐼", "🐉", "👾", "🧟", "💀", "👻", "🕷️", "🦇"]),
    ("sticker.groupFaces", ["😀", "😎", "🤠", "🤖", "😈", "🥳", "🤩", "😴", "🤯", "🫠", "👽", "🎃", "🥸", "😤", "🤓", "🫡"]),
    ("sticker.groupSymbols", ["⭐", "🌈", "⚡", "❤️", "🏆", "🎮", "✨", "🔷", "🟢", "🟣", "🟡", "🔶", "☠️", "🌙", "☀️", "🎯"]),
];

fn emoji(theme: &Theme) -> El {
    let cell = (BODY_W - 6.0 * 7.0) / 8.0;
    let mut out = col().key("emoji").gap(18.0);
    for (gi, (name, list)) in GROUPS.iter().enumerate() {
        let mut grid = col().gap(6.0);
        for (ri, chunk) in list.chunks(8).enumerate() {
            let mut line = row().gap(6.0);
            for (ci, glyph) in chunk.iter().enumerate() {
                let glyph: &'static str = glyph;
                let face = el()
                    .key(format!("e{gi}-{}", ri * 8 + ci))
                    .size(cell, cell)
                    .center()
                    .fill(theme.bg_input)
                    .box_shadow(rune::Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.22), inset: true })
                    .hover(theme.bg_hover)
                    .role("button")
                    .label(tf("sticker.useEmoji", &[("emoji", glyph)]))
                    .help(tf("sticker.useEmoji", &[("emoji", glyph)]))
                    .focusable(true)
                    .cursor(Cursor::Pointer)
                    .press_scale(0.94)
                    .on_press(move |app: &mut App| choose_emoji(app, glyph))
                    .child(text(glyph).font(theme.sans(22.0, 400.0)));
                line = line.child(boxed(theme, face, false));
            }
            grid = grid.child(line);
        }
        out = out.child(
            col()
                .child(text(tr(name).to_uppercase()).font(rune::tok::TextStyle { tracking: 0.48, ..theme.pixel(12.0) }).color(theme.text_muted).mb(8.0))
                .child(grid),
        );
    }
    out
}
