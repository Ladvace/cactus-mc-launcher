//! The Appearance card: accent, theme presets, the custom background and the Giphy key.

use std::rc::Rc;

use base64::Engine;
use rune::{Color, Cursor, El, FileDialogOptions, Shadow, col, el, row, text};

use super::bg::{self, DEFAULT_COLOR, Kind, PATTERNS};
use super::widgets::{card_with_head, chip, h3, label, range, seg, setting};
use super::{edit, field, save_one};
use crate::app::App;
use crate::core::i18n::t;
use crate::icons;
use crate::settings::Settings;
use crate::theme::{ACCENT_PRESETS, THEME_PRESETS, Theme, parse_color};
use crate::ui::Btn;

/// `DECOR_THEMES[..].placements[0].sprite`.
fn decor_sprite(id: &str) -> Option<&'static str> {
    match id {
        "desert" => Some("/decor/sprites/00.png"),
        "bloom" => Some("/decor/sprites/01.png"),
        "mystic" => Some("/decor/sprites/22.png"),
        _ => None,
    }
}

/// The colour the `colorField` of the current background kind shows.
pub fn current_color(settings: &Settings) -> String {
    let background = &settings.background;
    let color = match bg::kind(background) {
        Kind::Color => Some(background[6..].to_string()).filter(|hex| bg::is_hex(hex)),
        Kind::Pattern => bg::parse_pattern(background).1,
        Kind::Image => bg::parse_image(background).1,
        _ => None,
    };
    color.unwrap_or_else(|| DEFAULT_COLOR.to_string())
}

/// `setColor` / `setPatternColor` / `setImageColor`, by the current kind.
pub fn set_color(settings: &mut Settings, color: &str) {
    match bg::kind(&settings.background) {
        Kind::Color => settings.background = format!("color:{color}"),
        Kind::Pattern => {
            let (name, _) = bg::parse_pattern(&settings.background);
            settings.background = format!("pattern:{}|{color}", if name.is_empty() { "dots" } else { &name });
        }
        Kind::Image => {
            let (uri, _) = bg::parse_image(&settings.background);
            if !uri.is_empty() {
                settings.background = format!("image:{color}|{uri}");
            }
        }
        _ => {}
    }
}

fn set_pattern(settings: &mut Settings, name: &str) {
    let (_, color) = if bg::kind(&settings.background) == Kind::Pattern { bg::parse_pattern(&settings.background) } else { (String::new(), None) };
    settings.background = match color {
        Some(color) => format!("pattern:{name}|{color}"),
        None => format!("pattern:{name}"),
    };
}

fn pick_sticker(app: &mut App) {
    app.open_sticker_picker(t("settings.appBackground"), |app, uri| edit(app, |draft| draft.background = format!("image:{uri}")));
}

/// `onBgFile`: the chosen image as a data URI.
fn upload(_: &mut App, tx: &mut rune::Tx) {
    let options = FileDialogOptions { title: t("settings.upload"), ..Default::default() }.filter("Images", &["png", "jpg", "jpeg", "gif", "webp"]);
    tx.open_file(options, |app: &mut App, result, _| {
        let Ok(Some(path)) = result else { return };
        app.fx.blocking(
            move |_| {
                let bytes = std::fs::read(&path).ok()?;
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
                let mime = match ext.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "image/png",
                };
                Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
            },
            |app, uri| match uri {
                Some(uri) => edit(app, |draft| draft.background = format!("image:{uri}")),
                None => eprintln!("background upload failed"),
            },
        );
    });
}

pub fn view(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let draft = &page.draft;
    let mut rows = Vec::new();

    let mut accents = row().key("accents").gap(8.0);
    for accent in &ACCENT_PRESETS {
        let on = draft.accent == accent.id;
        let dot = parse_color(accent.swatch).unwrap_or(Color::BLACK);
        let id = accent.id;
        let mut button = el()
            .key(format!("accent-{}", if id.is_empty() { "gold" } else { id }))
            .size(26.0, 26.0)
            .fill(dot)
            .border(2.0, if on { theme.text } else { theme.border })
            .role("button")
            .checked(on)
            .label(t(accent.label_key))
            .help(t(accent.label_key))
            .focusable(true)
            .cursor(Cursor::Pointer)
            .press_scale(0.94)
            .on_press(move |app: &mut App| save_one(app, |settings| settings.accent = id.into()));
        if on {
            button = button.outline(2.0, 0.0, dot);
        }
        accents = accents.child(button);
    }
    rows.push(setting(theme, "accent", true, label(theme, t("settings.accent"), Some(t("settings.accentDesc"))), accents));

    rows.push(label(theme, t("settings.themePresets"), Some(t("settings.themePresetsDesc"))).mb(10.0));
    let tab = |animated: bool| -> Rc<dyn Fn(&mut App)> { Rc::new(move |app: &mut App| app.settings_page.animated_tab = animated) };
    rows.push(
        row().mb(12.0).child(seg(theme, "theme-tabs", vec![
            (t("settings.themesStatic"), !page.animated_tab, tab(false)),
            (t("settings.themesAnimated"), page.animated_tab, tab(true)),
        ])),
    );
    rows.push(themes(app, theme));

    rows.push(setting(
        theme,
        "background",
        false,
        label(theme, t("settings.background"), Some(t("settings.backgroundDesc"))),
        bg::framed(app, theme, &draft.background, theme.border)
            .key("bg-preview")
            .size(108.0, 60.0)
            .box_shadow(Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.28), inset: true }),
    ));

    let kind = bg::kind(&draft.background);
    let color_value = current_color(draft);
    rows.push(
        row()
            .key("bg-kinds")
            .w_full()
            .flex_wrap()
            .gap(8.0)
            .pt(14.0)
            .border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle)
            .child(chip(theme, "bg-default", t("settings.bgDefault"), kind == Kind::Default, |app| edit(app, |draft| draft.background.clear())))
            .child(chip(theme, "bg-color", t("settings.bgSolidColor"), kind == Kind::Color, move |app| {
                let color = current_color(&app.settings_page.draft);
                edit(app, |draft| draft.background = format!("color:{color}"));
            }))
            .child(chip(theme, "bg-pattern", t("settings.bgPattern"), kind == Kind::Pattern, |app| {
                edit(app, |draft| {
                    let name = if bg::kind(&draft.background) == Kind::Pattern { bg::parse_pattern(&draft.background).0 } else { String::new() };
                    set_pattern(draft, if name.is_empty() { "dots" } else { &name });
                })
            }))
            .child(chip(theme, "bg-image", t("settings.bgImageGif"), kind == Kind::Image, pick_sticker)),
    );

    let detail = || row().w_full().center_y().gap(12.0).pt(14.0);
    match kind {
        Kind::Color => rows.push(detail().key("bg-detail").child(color_field(app, theme, &color_value))),
        Kind::Pattern => {
            let (active, _) = bg::parse_pattern(&draft.background);
            let mut swatches = detail().key("patterns").flex_wrap();
            for name in PATTERNS {
                let on = active == name;
                let mut swatch = bg::framed(app, theme, &format!("pattern:{name}|{color_value}"), if on { theme.accent } else { theme.border })
                    .key(format!("pattern-{name}"))
                    .size(56.0, 40.0)
                    .role("button")
                    .checked(on)
                    .label(name)
                    .help(name)
                    .focusable(true)
                    .cursor(Cursor::Pointer)
                    .press_scale(0.96)
                    .on_press(move |app: &mut App| edit(app, |draft| set_pattern(draft, name)));
                if on {
                    swatch = swatch.outline(2.0, 0.0, theme.accent_soft);
                }
                swatches = swatches.child(swatch);
            }
            rows.push(swatches);
            rows.push(
                detail()
                    .key("bg-color")
                    .child(text(t("settings.baseColor")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary))
                    .child(color_field(app, theme, &color_value)),
            );
        }
        Kind::Image => {
            rows.push(
                detail()
                    .key("bg-detail")
                    .child(Btn::new(t("settings.upload")).key("bg-upload").ghost().icon(&icons::EDIT).on_press_tx(upload).build(theme))
                    .child(Btn::new(t("settings.stickersGifs")).key("bg-stickers").ghost().icon(&icons::SPARKLES).on_press(pick_sticker).build(theme)),
            );
            rows.push(
                detail()
                    .key("bg-color")
                    .child(text(t("settings.tint")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary))
                    .child(color_field(app, theme, &color_value)),
            );
        }
        Kind::Texture => {
            let (_, _, opacity) = bg::parse_texture(&draft.background);
            rows.push(
                detail()
                    .key("bg-color")
                    .child(text(t("settings.textureOpacity")).font(theme.sans(13.0, 400.0)).color(theme.text_secondary))
                    .child(range(theme, "texture-opacity", 360.0, (opacity - 0.1) / 0.9, 18, 0, false, |app, value| {
                        let new_opacity = ((0.1 + value * 0.9) * 100.0).round() / 100.0;
                        edit(app, |draft| {
                            let (uri, color, _) = bg::parse_texture(&draft.background);
                            let prefix = color.map(|hex| format!("{hex}|")).unwrap_or_default();
                            draft.background = format!("texture:{new_opacity}|{prefix}{uri}");
                        });
                    }))
                    .child(text(format!("{}%", (opacity * 100.0).round())).font(theme.pixel(14.0)).color(theme.text_secondary)),
            );
        }
        _ => {}
    }

    rows.push(super::general::giphy(app, theme).mt(14.0));
    card_with_head(theme, "appearance", h3(theme, t("settings.appearance")), rows)
}

/// `.themes`: `repeat(auto-fill, minmax(88px, 1fr))` at this width is six columns.
fn themes(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let draft = &page.draft;
    let shown: Vec<_> = THEME_PRESETS.iter().filter(|preset| preset.bg.starts_with("anim:") == page.animated_tab).collect();
    let mut grid = col().key("themes").w_full().gap(8.0).mb(18.0);
    for chunk in shown.chunks(6) {
        let mut swatch_row = row().w_full().gap(8.0);
        for preset in chunk {
            let on = draft.background == preset.bg && draft.decor_theme == preset.decor.unwrap_or("");
            let mut swatch = bg::framed(app, theme, preset.bg, if on { theme.accent } else { theme.border }).w_full().h(44.0);
            if on {
                swatch = swatch.outline(2.0, 0.0, theme.accent_soft);
            }
            if let Some(src) = preset.decor.and_then(decor_sprite) {
                let sprite = match app.images.get(&app.fx, src) {
                    Some(img) => el().w(26.0).pixels(img.pixels()),
                    None => el().size(26.0, 26.0),
                };
                swatch = swatch.child(row().w_full().h_full().absolute(0.0, 0.0).justify_end().items_end().pr(3.0).pb(2.0).child(sprite));
            }
            let (background, decor) = (preset.bg, preset.decor.unwrap_or(""));
            swatch_row = swatch_row.child(
                col()
                    .key(format!("theme-{}", preset.name.to_lowercase()))
                    .grow(1.0)
                    .w(0.0)
                    .gap(4.0)
                    .role("button")
                    .checked(on)
                    .label(preset.name)
                    .help(preset.name)
                    .focusable(true)
                    .cursor(Cursor::Pointer)
                    .on_press(move |app: &mut App| {
                        edit(app, |draft| {
                            draft.background = background.into();
                            draft.decor_theme = decor.into();
                        })
                    })
                    .child(swatch)
                    .child(text(preset.name).font(theme.sans(11.5, 400.0)).color(if on { theme.accent } else { theme.text_secondary })),
            );
        }
        for _ in chunk.len()..6 {
            swatch_row = swatch_row.child(el().grow(1.0).w(0.0));
        }
        grid = grid.child(swatch_row);
    }
    grid
}

/// The `colorField` snippet: a colour well and its hex, which can be typed.
fn color_field(app: &App, theme: &Theme, value: &str) -> El {
    let page = &app.settings_page;
    let well = el().size(52.0, 40.0).fill(theme.bg_input).border(2.0, theme.border).padding(4.0).child(
        el().w_full().h_full().fill(parse_color(value).unwrap_or(Color::BLACK)),
    );
    let hex = rune::text_input(&page.hex, theme.pixel(14.0))
        .key("hex-input")
        .label(t("settings.background"))
        .single_line()
        .color(theme.text_secondary)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(|app| &mut field(app).settings_page.hex)
        .on_submit(|_: &mut App, tx| tx.blur())
        .w(90.0);
    row().center_y().gap(12.0).child(well).child(hex)
}
