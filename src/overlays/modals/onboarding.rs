//! `Onboarding.svelte`: the five first-run steps over everything, with a language picker.

use std::rc::Rc;

use rune::{Color, Cursor, Cx, El, TextEdit, Transition, Tx, col, el, row, text};

use super::widgets::{Opt, Select};
use crate::app::App;
use crate::core::i18n::{LOCALES, t as tr, tf};
use crate::icons;
use crate::theme::{EMBER_STOPS, NEBULA_STOPS, THEME_PRESETS, TIDE_STOPS, Theme, parse_color};
use crate::ui::{self, Btn};

const STEPS: usize = 5;

#[derive(Default)]
pub struct State {
    pub step: usize,
    /// -1 going back, 1 going forward: the side the next step slides in from.
    dir: i8,
    username: TextEdit,
    lang_open: bool,
}

pub fn on_start(app: &mut App) {
    let name = if app.settings.offline_username.is_empty() { "Player".to_string() } else { app.settings.offline_username.clone() };
    app.modals.onboarding.username = TextEdit::new(name);
    app.modals.onboarding.dir = 1;
}

/// Enter on the welcome step is "Get started".
pub fn on_enter_key(app: &mut App) -> bool {
    if app.modals.onboarding.step == 0 {
        next(app);
        return true;
    }
    false
}

fn next(app: &mut App) {
    let onboarding = &mut app.modals.onboarding;
    if onboarding.step < STEPS - 1 {
        onboarding.dir = 1;
        onboarding.step += 1;
    }
}

fn back(app: &mut App) {
    let onboarding = &mut app.modals.onboarding;
    if onboarding.step > 0 {
        onboarding.dir = -1;
        onboarding.step -= 1;
    }
}

fn finish(app: &mut App, create_instance: bool) {
    let name = app.modals.onboarding.username.text.trim().to_string();
    app.update_settings(|settings| settings.offline_username = if name.is_empty() { "Player".into() } else { name });
    app.set_onboarded();
    if create_instance {
        app.ui.create_instance_open = true;
    }
}

fn username(app: &App) -> String {
    app.modals.onboarding.username.text.trim().to_string()
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    let theme = app.theme();
    let onboarding = &app.modals.onboarding;
    let step = match onboarding.step {
        0 => welcome(app, &theme),
        1 => sign_in(app, &theme),
        2 => pick_look(app, &theme),
        3 => no_ads(app, &theme),
        _ => all_set(app, &theme),
    };
    let step = col()
        .key(format!("step-{}", onboarding.step))
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .center()
        .gap(14.0)
        .transition(Transition::opacity().combined(Transition::offset(f32::from(onboarding.dir) * 32.0, 0.0)))
        .child(step);
    let mut dots = row().key("dots").gap(8.0).mt(14.0);
    for i in 0..STEPS {
        let (w, color) = if i == onboarding.step {
            (22.0, theme.accent)
        } else if i < onboarding.step {
            (8.0, theme.accent.alpha(0.55))
        } else {
            (8.0, theme.border)
        };
        dots = dots.child(el().key(format!("dot{i}")).size(w, 8.0).radius(4.0).fill(color).motion(rune::Spring::new(0.2, 0.0)));
    }
    let card = col()
        .key("card")
        .w(560.0)
        // At least 500 so the steps don't jump in size; taller only if a step needs it.
        .min_h(500.0)
        .clip_corners([0.0; 4])
        .items_center()
        .pt(32.0)
        .px(40.0)
        .pb(22.0)
        .fill(theme.bg_card)
        .border(2.0, theme.border)
        .shadow(24.0, 60.0, Color::BLACK.alpha(0.45))
        .child(step)
        .child(dots);
    let card = ui::bevel(card, 2.0, 0.04, 0.3);

    let skip = el()
        .key("skip")
        .role("button")
        .label(tr("common.skip"))
        .cursor(Cursor::Pointer)
        .hover_group()
        .on_press(|app: &mut App| finish(app, false))
        .child(text(tr("common.skip")).font(theme.sans(13.0, 400.0)).color(theme.text_muted).group_color(theme.text));

    let mut lang_layer = Vec::new();
    let lang = language_select(app, &theme, &mut lang_layer);

    let mut root = el()
        .key("onboarding")
        .w_full()
        .h_full()
        .absolute(0.0, 0.0)
        .center()
        .padding(24.0)
        .clip_corners([0.0; 4])
        .fill(theme.bg_app)
        .radial_gradient([0.5, 0.0], [1400.0, 900.0], [(0.0, theme.accent.alpha(0.22)), (0.6, theme.accent.alpha(0.0))])
        .modal(false, |_: &mut App| {})
        .transition(Transition::opacity().combined(Transition::offset(0.0, 12.0)));
    // The chosen background, live, so picking a theme shows it (the screen is opaque over the
    // app). Without one, the accent wash above is the onboarding's own look.
    if !app.settings.background.trim().is_empty() {
        root = root.child(crate::overlays::extras::background(app, cx).key("onboarding-bg").absolute(0.0, 0.0));
    }
    let root = root
        .child(
            // `.glow`, at the start of its drift (translate(-10%, -8%)).
            el().key("glow-box").absolute(0.0, 0.0).w_full().h_full().center().child(
                el().key("glow")
                    .size(640.0, 640.0)
                    .ml(-128.0)
                    .mt(-102.0)
                    .radius(320.0)
                    .radial_gradient([0.5, 0.5], 320.0, [(0.0, theme.accent.alpha(0.3)), (0.7, theme.accent.alpha(0.0))])
                    .blur(40.0)
                    .opacity(0.6),
            ),
        )
        // Below the macOS traffic lights (y 6–20), which the full-size titlebar draws over the content.
        .child(el().key("lang-corner").absolute(24.0, 34.0).child(lang))
        .child(row().key("skip-box").absolute(0.0, 0.0).w_full().justify_end().pt(22.0).pr(24.0).child(skip))
        .child(card);
    layer.push(root);
    layer.extend(lang_layer);
}

/// A plain language picker over `LOCALES` (LanguageSelect's own look is the settings page's).
fn language_select(app: &App, theme: &Theme, layer: &mut Vec<El>) -> El {
    let opts: Vec<Opt> = LOCALES.iter().map(|(code, name)| Opt::new(*code, *name)).collect();
    Select {
        key: "onboarding-lang",
        options: &opts,
        value: crate::core::i18n::normalize(&app.settings.language),
        open: app.modals.onboarding.lang_open,
        width: 160.0,
        toggle: |app| app.modals.onboarding.lang_open = !app.modals.onboarding.lang_open,
        close: |app| app.modals.onboarding.lang_open = false,
        choose: Rc::new(|app: &mut App, code: &str| {
            let code = code.to_string();
            app.update_settings(|settings| settings.language = code);
        }),
        search: None,
        header: None,
    }
    .build(theme, layer)
}

/// An image `w` wide at its own aspect (`fallback_h` tall until its size is known).
fn picture(app: &App, src: &str, w: f32, fallback_h: f32) -> El {
    match app.images.get(&app.fx, src) {
        Some(img) => match img.size() {
            Some([iw, ih]) => el().size(w, w * ih as f32 / iw.max(1) as f32).pixels(img.pixels()),
            None => el().size(w, fallback_h).pixels(img.pixels()),
        },
        None => el().size(w, fallback_h),
    }
}

fn h1(theme: &Theme, label: String) -> El {
    text(label).font(theme.pixel(24.0)).color(theme.text).wrap().align(rune::Align::Center)
}

fn lead(theme: &Theme, label: String) -> El {
    text(label)
        .font(rune::tok::TextStyle { line_height: 22.4, ..theme.sans(14.0, 400.0) })
        .color(theme.text_secondary)
        .wrap()
        .align(rune::Align::Center)
        .max_w(380.0)
}

fn eyebrow(theme: &Theme, label: String) -> El {
    text(label.to_uppercase()).font(rune::tok::TextStyle { tracking: 1.2, ..theme.pixel(10.0) }).color(theme.accent)
}

fn nav(theme: &Theme) -> El {
    row()
        .key("nav")
        .gap(10.0)
        .mt(10.0)
        .child(Btn::new(tr("common.back")).ghost().key("back").on_press(back).build(theme))
        .child(Btn::new(tr("common.continue")).primary().key("continue").on_press(next).build(theme))
}

fn welcome(app: &App, theme: &Theme) -> El {
    let title = row()
        .flex_wrap()
        .center()
        .gap(8.0)
        .child(text(tr("onboarding.welcome")).font(theme.pixel(24.0)).color(theme.text))
        .child(text("Cactus\u{a0}Launcher").font(theme.pixel(24.0)).color(theme.accent));
    col()
        .items_center()
        .gap(14.0)
        .child(picture(app, "/empty-cactus.png", 150.0, 90.0))
        .child(title)
        .child(lead(theme, tr("onboarding.welcomeLead")))
        .child(row().mt(8.0).child(Btn::new(tr("onboarding.getStarted")).primary().big().icon(&icons::PLAY).key("get-started").on_press(next).build(theme)))
}

fn sign_in(app: &App, theme: &Theme) -> El {
    let mut body = col().items_center().gap(14.0).child(eyebrow(theme, tr("onboarding.yourPlayer"))).child(h1(theme, tr("onboarding.signInToPlay")));
    if let Some(account) = app.active_account() {
        body = body.child(super::widgets::strong(
            theme,
            &format!("{} ", tr("onboarding.signedInAs")),
            &account.username,
            &format!(". {}", tr("onboarding.readyOnline")),
            14.0,
            theme.text_secondary,
            theme.text,
        ));
    } else if let Some(code) = &app.accounts.device_code {
        let user_code = code.user_code.clone();
        let uri = code.verification_uri.clone();
        body = body
            .child(lead(theme, tr("onboarding.enterCode")))
            .child(
                row()
                    .key("code-row")
                    .gap(10.0)
                    .center_y()
                    .child(
                        el().key("code")
                            .px(18.0)
                            .py(10.0)
                            .fill(theme.bg_input)
                            .border(2.0, theme.border)
                            .role("button")
                            .help(tr("onboarding.copy"))
                            .cursor(Cursor::Pointer)
                            .on_press_tx(move |app: &mut App, tx: &mut Tx| {
                                tx.copy(user_code.clone());
                                app.toasts.success(tr("onboarding.codeCopied"));
                            })
                            .child(text(code.user_code.clone()).font(rune::tok::TextStyle { tracking: 2.9, ..theme.pixel(24.0) }).color(theme.accent)),
                    )
                    .child(
                        Btn::new(tr("onboarding.openLink"))
                            .ghost()
                            .sm()
                            .key("open-link")
                            .on_press(move |_| {
                                let _ = open::that(&uri);
                            })
                            .build(theme),
                    ),
            )
            .child(
                row()
                    .gap(8.0)
                    .center_y()
                    .child(ui::spinner(theme, 13.0))
                    .child(
                        text(if code.authorizing { tr("onboarding.signingIn") } else { tr("onboarding.waitingAuth") })
                            .font(theme.sans(12.5, 400.0))
                            .color(theme.text_secondary),
                    ),
            );
        if !code.authorizing {
            body = body.child(row().child(Btn::new(tr("common.cancel")).ghost().sm().key("cancel-login").on_press(|app| app.cancel_login()).build(theme)));
        }
    } else {
        if app.accounts.microsoft_configured {
            body = body
                .child(lead(theme, tr("onboarding.msLead")))
                .child(row().child(
                    Btn::new(tr("onboarding.signInMicrosoft"))
                        .primary()
                        .big()
                        .icon(&icons::USER)
                        .key("ms-login")
                        .on_press(|app| app.login_microsoft())
                        .build(theme),
                ))
                .child(
                    row()
                        .key("divider")
                        .w(320.0)
                        .gap(10.0)
                        .center_y()
                        .child(el().h(1.0).grow(1.0).fill(theme.border))
                        .child(text(tr("onboarding.orPlayOffline")).font(theme.sans(11.5, 400.0)).color(theme.text_muted))
                        .child(el().h(1.0).grow(1.0).fill(theme.border)),
                );
        } else {
            body = body.child(lead(theme, tr("onboarding.offlineLead")));
        }
        let onboarding = &app.modals.onboarding;
        let input = rune::text_input(&onboarding.username, theme.pixel(18.0))
            .single_line()
            .label(tr("onboarding.yourPlayer"))
            .placeholder("Player", theme.text_muted)
            .color(theme.text)
            .align(rune::Align::Center)
            .selection_color(theme.accent.alpha(0.6))
            .on_edit(|app: &mut App| {
                let edit = &mut app.modals.onboarding.username;
                if edit.text.chars().count() > 24 {
                    let cut: String = edit.text.chars().take(24).collect();
                    edit.replace_text(cut);
                }
                edit
            })
            .on_submit(|app: &mut App, _: &mut Tx| next(app))
            .w_full();
        body = body.child(
            el().key("name-input")
                .w(320.0)
                .mt(4.0)
                .px(14.0)
                .py(12.0)
                .fill(theme.bg_input)
                .border(2.0, if onboarding.username.focused { theme.accent } else { theme.border })
                .box_shadow(rune::Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.3), inset: true })
                .child(input),
        );
    }
    body.child(nav(theme))
}

/// `backgroundCss` for a swatch: a colour, a linear gradient, a pattern's or texture's base
/// colour, or an animated theme's first frame.
fn swatch(theme: &Theme, bg: &str) -> El {
    let tile = el().size(52.0, 52.0).border(2.0, theme.border);
    let base = |value: &str| parse_color(value).unwrap_or(theme.bg_app);
    if let Some(name) = bg.strip_prefix("anim:") {
        let stops: &[&str] = match name {
            "nebula" => &NEBULA_STOPS[..4],
            "ember" => &EMBER_STOPS[..4],
            "tide" => &TIDE_STOPS[..4],
            "starfield" => &["#080a12", "#080a12"],
            "beams" => &["#0a0f18", "#16243a"],
            "circuit" => &["#0c1016", "#14202c"],
            _ => &["#0a0c12", "#141a26"],
        };
        let n = stops.len() as f32 - 1.0;
        let stops: Vec<(f32, Color)> = stops.iter().enumerate().map(|(i, hex)| (i as f32 / n, base(hex))).collect();
        return tile.fill(stops[0].1).linear_gradient(130f32.to_radians(), stops);
    }
    if let Some(css) = bg.strip_prefix("color:") {
        if let Some(inner) = css.strip_prefix("linear-gradient(").and_then(|tail| tail.strip_suffix(')')) {
            let mut parts = inner.split(',').map(str::trim);
            let angle = parts.next().and_then(|token| token.strip_suffix("deg")).and_then(|token| token.parse::<f32>().ok()).unwrap_or(180.0);
            let stops: Vec<(f32, Color)> = parts
                .filter_map(|part| {
                    let (color, pos) = part.split_once(' ').unwrap_or((part, ""));
                    Some((pos.trim_end_matches('%').parse::<f32>().map(|pct| pct / 100.0).unwrap_or(0.0), parse_color(color)?))
                })
                .collect();
            let first = stops.first().map_or(theme.bg_app, |stop| stop.1);
            return tile.fill(first).linear_gradient(angle.to_radians(), stops);
        }
        return tile.fill(base(css));
    }
    if let Some(rest) = bg.strip_prefix("pattern:").or_else(|| bg.strip_prefix("texture:")) {
        let color = rest.split('|').find(|part| part.starts_with('#')).unwrap_or("");
        return tile.fill(base(color));
    }
    tile.fill(theme.bg_app)
}

fn pick_look(app: &App, theme: &Theme) -> El {
    let current = app.settings.background.as_str();
    // Seven to a row (7 × 62 + 6 × 6 = 470 of the card's 480): at five, the presets ran to
    // five rows and pushed the heading and buttons out of the card.
    let mut grid = col().key("themes").gap(6.0);
    for chunk in THEME_PRESETS.chunks(7) {
        let mut line = row().gap(6.0).justify_center();
        for preset in chunk {
            let on = current == preset.bg;
            let (bg, decor) = (preset.bg, preset.decor.unwrap_or(""));
            line = line.child(
                col()
                    .key(format!("theme-{}", preset.name))
                    .w(62.0)
                    .items_center()
                    .gap(5.0)
                    .padding(3.0)
                    .border(2.0, if on { theme.accent } else { Color::TRANSPARENT })
                    .role("button")
                    .label(preset.name)
                    .help(preset.name)
                    .cursor(Cursor::Pointer)
                    .on_press(move |app: &mut App| {
                        app.update_settings(|settings| {
                            settings.background = bg.to_string();
                            settings.decor_theme = decor.to_string();
                        })
                    })
                    .child(swatch(theme, preset.bg))
                    .child(text(preset.name).font(theme.sans(10.0, 400.0)).color(if on { theme.accent } else { theme.text_secondary })),
            );
        }
        grid = grid.child(line);
    }
    col()
        .items_center()
        .gap(14.0)
        .child(eyebrow(theme, tr("onboarding.makeItYours")))
        .child(h1(theme, tr("onboarding.pickLook")))
        .child(lead(theme, tr("onboarding.pickLookLead")))
        .child(grid)
        .child(nav(theme))
}

fn no_ads(app: &App, theme: &Theme) -> El {
    col()
        .items_center()
        .gap(14.0)
        .child(picture(app, "/no-ads.png", 230.0, 164.0))
        .child(h1(theme, tr("onboarding.noAdsTitle")))
        .child(lead(theme, tr("onboarding.noAdsLead")))
        .child(nav(theme))
}

fn all_set(app: &App, theme: &Theme) -> El {
    let name = username(app);
    let social = |key: &str, icon: Option<&'static rune::Icon>, label: String, url: &'static str| {
        let mut link = row()
            .key(key.to_string())
            .gap(6.0)
            .center_y()
            .px(14.0)
            .py(8.0)
            .fill(theme.bg_input)
            .role("link")
            .label(label.clone())
            .cursor(Cursor::Pointer)
            .on_press(move |_: &mut App| {
                let _ = open::that(url);
            });
        link = match icon {
            Some(icon) => link.child(ui::colored_icon(icon, 13.0, theme.text_secondary).group_color(theme.accent)),
            None => link.child(text("★").font(theme.sans(12.0, 400.0)).color(theme.text_secondary).group_color(theme.accent)),
        };
        let face = link.child(text(label).font(theme.sans(12.0, 400.0)).color(theme.text_secondary).group_color(theme.accent)).hover_group();
        // `.social`: a subtle border that turns accent on hover.
        face.child(el().absolute(0.0, 0.0).w_full().h_full().border(2.0, theme.border_subtle))
            .child(el().absolute(0.0, 0.0).w_full().h_full().border(2.0, theme.accent).group_opacity(0.0, 1.0))
    };
    col()
        .items_center()
        .gap(14.0)
        .child(picture(app, "/empty-cactus.png", 150.0, 90.0))
        .child(h1(theme, tf("onboarding.allSet", &[("name", &if name.is_empty() { String::new() } else { format!(", {name}") })])))
        .child(lead(theme, tr("onboarding.allSetLead")))
        .child(
            col()
                .key("finish")
                .items_center()
                .gap(10.0)
                .mt(10.0)
                .child(row().child(
                    Btn::new(tr("onboarding.createFirstInstance"))
                        .primary()
                        .big()
                        .icon(&icons::PLUS)
                        .key("create-first")
                        .on_press(|app| finish(app, true))
                        .build(theme),
                ))
                .child(row().child(Btn::new(tr("onboarding.exploreOwn")).ghost().key("explore").on_press(|app| finish(app, false)).build(theme))),
        )
        .child(
            row()
                .key("socials")
                .gap(10.0)
                .mt(6.0)
                .child(social("discord", Some(&icons::USERS), tr("onboarding.joinDiscord"), "https://discord.gg/bfdUaMf7Mg"))
                .child(social("github", None, tr("onboarding.starGithub"), "https://github.com/Ladvace/cactus-mc-launcher")),
        )
        .child(text(tr("onboarding.enableExtrasTip")).font(theme.sans(11.0, 400.0)).color(theme.text_muted).wrap().align(rune::Align::Center))
}
