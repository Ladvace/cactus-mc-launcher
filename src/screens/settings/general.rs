//! Account, General (with LanguageSelect and Flag), Sound, Home news, Dock, Accessibility,
//! Game window and Servers.

use std::rc::Rc;

use rune::{Color, El, Shadow, TextEdit, row, text};

use super::widgets::{card, dropdown, label, linked_small, range, seg, setting, switch, ticks};
use super::{edit, field, save_one};
use crate::app::App;
use crate::core::i18n::{LOCALES, t, tf};
use crate::icons;
use crate::settings::Settings;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const GIPHY_DEV: &str = "https://developers.giphy.com";
const NGROK_AUTHTOKEN: &str = "https://dashboard.ngrok.com/get-started/your-authtoken";

/// A `.input` bound to `access`; `secure` for `type="password"`, `width` for `.narrow`/`.tiny`.
pub(super) fn input(
    theme: &Theme,
    edit: &TextEdit,
    key: &str,
    name: String,
    placeholder: impl Into<String>,
    width: Option<f32>,
    secure: bool,
    access: impl for<'a> Fn(&'a mut App) -> &'a mut TextEdit + 'static,
) -> El {
    let mut text_field = rune::text_input(edit, theme.sans(13.0, 400.0))
        .key(key.to_string())
        .label(name)
        .single_line()
        .placeholder(placeholder.into(), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(access)
        .on_submit(|_: &mut App, tx| tx.blur())
        .grow(1.0);
    if secure {
        text_field = text_field.secure();
    }
    let frame = ui::input_frame(theme, edit.focused).child(text_field);
    match width {
        Some(w) => frame.w(w),
        None => frame.w_full(),
    }
}

/// `Flag.svelte`: the 3:2 pixel flag with its 1 pt dark outline.
pub(super) fn flag(app: &App, code: &str, width: f32) -> El {
    ui::image(app, &format!("/flags/{code}.png"), width, width * 2.0 / 3.0).box_shadow(Shadow {
        x: 0.0,
        y: 0.0,
        blur: 0.0,
        spread: 1.0,
        color: Color::BLACK.alpha(0.35),
        inset: false,
    })
}

pub fn account(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    card(theme, "account", t("settings.account"), vec![setting(
        theme,
        "offline-username",
        true,
        label(theme, t("settings.offlineUsername"), Some(t("settings.offlineUsernameDesc"))),
        input(theme, &page.username, "username-input", t("settings.offlineUsername"), t("settings.offlineUsernamePlaceholder"), Some(240.0), false, |app| {
            &mut field(app).settings_page.username
        }),
    )])
}

pub fn general(app: &App, theme: &Theme, layer: &mut Vec<El>) -> El {
    let page = &app.settings_page;
    let draft = &page.draft;
    let langs: Vec<(String, String)> = LOCALES.iter().map(|(code, name)| (code.to_string(), name.to_string())).collect();
    let lead = |code: &str| flag(app, code, 22.0);
    let lang = dropdown(theme, layer, "lang", 172.0, &t("settings.language"), page.open == Some("lang"), &app.settings.language, langs, Some(&lead), |app, code| {
        if code != app.settings.language {
            let code = code.to_string();
            save_one(app, move |settings| settings.language = code.clone());
        }
    });
    let sample = ui::fmt::date(chrono::Utc::now(), &draft.date_format);
    let dates = vec![
        ("system".to_string(), t("settings.dateSystem")),
        ("iso".to_string(), "ISO (2026-07-20)".to_string()),
        ("us".to_string(), "US (07/20/2026)".to_string()),
        ("eu".to_string(), "EU (20/07/2026)".to_string()),
    ];
    let date = dropdown(theme, layer, "date", 220.0, &t("settings.dateFormat"), page.open == Some("date"), &draft.date_format, dates, None, |app, value| {
        let value = value.to_string();
        save_one(app, move |settings| settings.date_format = value.clone());
    });
    let loaders = [("vanilla", "Vanilla"), ("fabric", "Fabric"), ("quilt", "Quilt"), ("forge", "Forge"), ("neoforge", "NeoForge")]
        .iter()
        .map(|(value, name)| (value.to_string(), name.to_string()))
        .collect();
    let loader = dropdown(theme, layer, "loader", 220.0, &t("settings.defaultLoader"), page.open == Some("loader"), &draft.default_loader, loaders, None, |app, value| {
        let value = value.to_string();
        save_one(app, move |settings| settings.default_loader = value.clone());
    });
    card(theme, "general", t("settings.general"), vec![
        setting(theme, "language", true, label(theme, t("settings.language"), Some(t("settings.languageDesc"))), lang),
        setting(theme, "date-format", false, label(theme, t("settings.dateFormat"), Some(tf("settings.dateFormatDesc", &[("sample", &sample)]))), date),
        setting(theme, "default-loader", false, label(theme, t("settings.defaultLoader"), Some(t("settings.defaultLoaderDesc"))), loader),
        setting(
            theme,
            "open-on-install",
            false,
            label(theme, t("settings.openInstanceOnInstall"), Some(t("settings.openInstanceOnInstallDesc"))),
            switch(theme, "open-on-install-switch", &t("settings.openInstanceOnInstall"), draft.open_instance_on_install, |app| {
                let enabled = !app.settings_page.draft.open_instance_on_install;
                save_one(app, move |settings| settings.open_instance_on_install = enabled);
            }),
        ),
    ])
}

pub fn sound(app: &App, theme: &Theme) -> El {
    let draft = &app.settings_page.draft;
    card(theme, "sound", t("settings.sound"), vec![
        setting(
            theme,
            "button-sounds",
            true,
            label(theme, t("settings.buttonSounds"), Some(t("settings.buttonSoundsDesc"))),
            switch(theme, "button-sounds-switch", &t("settings.buttonSounds"), draft.ui_sounds, |app| edit(app, |draft| draft.ui_sounds = !draft.ui_sounds)),
        ),
        setting(
            theme,
            "sound-volume",
            false,
            label(theme, t("settings.soundVolume"), Some(format!("{}%", draft.sound_volume))),
            range(theme, "volume-range", 240.0, draft.sound_volume as f32 / 100.0, 20, 0, !draft.ui_sounds, |app, fraction| {
                let volume = (fraction * 100.0).round() as u32;
                if volume != app.settings_page.draft.sound_volume {
                    save_one(app, move |settings| settings.sound_volume = volume);
                }
            }),
        ),
    ])
}

pub fn news(app: &App, theme: &Theme) -> El {
    let draft = &app.settings_page.draft;
    let mut rows = vec![setting(
        theme,
        "news-on-home",
        true,
        label(theme, t("settings.newsOnHome"), Some(t("settings.newsOnHomeDesc"))),
        switch(theme, "news-switch", &t("settings.newsOnHome"), draft.show_news, |app| {
            let enabled = !app.settings_page.draft.show_news;
            save_one(app, move |settings| settings.show_news = enabled);
        }),
    )];
    if draft.show_news {
        rows.push(setting(
            theme,
            "one-story",
            false,
            label(theme, t("settings.oneStoryPerPage"), Some(t("settings.oneStoryPerPageDesc"))),
            switch(theme, "one-story-switch", &t("settings.oneStoryPerPage"), draft.news_single, |app| {
                let enabled = !app.settings_page.draft.news_single;
                save_one(app, move |settings| settings.news_single = enabled);
            }),
        ));
    }
    card(theme, "news", t("settings.homeNews"), rows)
}

pub fn dock(app: &App, theme: &Theme) -> El {
    let draft = &app.settings_page.draft;
    let positions = [("bottom", "settings.dockBottom"), ("top", "settings.dockTop"), ("left", "settings.dockLeft"), ("right", "settings.dockRight")];
    let items = positions
        .iter()
        .map(|(value, label_key)| {
            let value = *value;
            let f: Rc<dyn Fn(&mut App)> = Rc::new(move |app: &mut App| edit(app, |draft| draft.dock_position = value.into()));
            (t(label_key), draft.dock_position == value, f)
        })
        .collect();
    card(theme, "dock", t("settings.dock"), vec![
        setting(theme, "dock-position", true, label(theme, t("settings.dockPosition"), Some(t("settings.dockPositionDesc"))), seg(theme, "dock-seg", items)),
        setting(
            theme,
            "magnify",
            false,
            label(theme, t("settings.magnifyDock"), Some(t("settings.magnifyDockDesc"))),
            switch(theme, "magnify-switch", &t("settings.magnifyDock"), draft.dock_magnify, |app| edit(app, |draft| draft.dock_magnify = !draft.dock_magnify)),
        ),
    ])
}

const SCALE_MIN: u32 = 70;
const SCALE_MAX: u32 = 130;

pub fn accessibility(app: &App, theme: &Theme) -> El {
    let draft = &app.settings_page.draft;
    let span = (SCALE_MAX - SCALE_MIN) as f32;
    let scale = rune::col()
        .w(240.0)
        .child(range(theme, "scale-range", 240.0, (draft.ui_scale.clamp(SCALE_MIN, SCALE_MAX) - SCALE_MIN) as f32 / span, 12, 0, false, move |app, fraction| {
            let scale = SCALE_MIN + (fraction * span).round() as u32;
            if scale != app.settings_page.draft.ui_scale {
                edit(app, |draft| draft.ui_scale = scale);
            }
        }))
        .child(ticks(theme, 240.0, [SCALE_MIN, 100, SCALE_MAX].iter().map(|percent| ((percent - SCALE_MIN) as f32 / span, format!("{percent}%"), None)).collect()));
    let mut rows = vec![setting(theme, "ui-scale", true, label(theme, t("settings.uiScale"), Some(format!("{}%", draft.ui_scale))), scale)];
    type Get = fn(&Settings) -> bool;
    type Set = fn(&mut Settings, bool);
    let toggles: [(&str, &str, &str, Get, Set); 5] = [
        ("reduce-motion", "settings.reduceMotion", "settings.reduceMotionDesc", |settings| settings.reduce_motion, |settings, on| settings.reduce_motion = on),
        ("readable-font", "settings.readableFont", "settings.readableFontDesc", |settings| settings.readable_font, |settings, on| settings.readable_font = on),
        ("high-contrast", "settings.highContrast", "settings.highContrastDesc", |settings| settings.high_contrast, |settings, on| settings.high_contrast = on),
        ("reduce-transparency", "settings.reduceTransparency", "settings.reduceTransparencyDesc", |settings| settings.reduce_transparency, |settings, on| settings.reduce_transparency = on),
        ("always-show-focus", "settings.alwaysShowFocus", "settings.alwaysShowFocusDesc", |settings| settings.always_show_focus, |settings, on| settings.always_show_focus = on),
    ];
    for (key, name, desc, get, set) in toggles {
        rows.push(setting(
            theme,
            key,
            false,
            label(theme, t(name), Some(t(desc))),
            switch(theme, &format!("{key}-switch"), &t(name), get(draft), move |app| {
                let enabled = !get(&app.settings_page.draft);
                save_one(app, move |settings| set(settings, enabled));
            }),
        ));
    }
    card(theme, "accessibility", t("settings.accessibility"), rows)
}

pub fn game_window(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let draft = &page.draft;
    let default = draft.game_width == 854 && draft.game_height == 480;
    let resolution = row()
        .center_y()
        .gap(8.0)
        .child(input(theme, &page.game_width, "width-input", "Width".into(), "", Some(90.0), false, |app| &mut field(app).settings_page.game_width))
        .child(text("×").font(theme.sans(14.0, 400.0)).color(theme.text_muted))
        .child(input(theme, &page.game_height, "height-input", "Height".into(), "", Some(90.0), false, |app| &mut field(app).settings_page.game_height))
        .child(
            Btn::new(t("settings.reset"))
                .key("reset-resolution")
                .ghost()
                .icon(&icons::REFRESH)
                .disabled(default)
                .help(t("settings.resetResolutionTitle"))
                .on_press(|app| {
                    app.settings_page.game_width.replace_text("854");
                    app.settings_page.game_height.replace_text("480");
                    edit(app, |draft| {
                        draft.game_width = 854;
                        draft.game_height = 480;
                    });
                })
                .build(theme),
        );
    card(theme, "game-window", t("settings.gameWindow"), vec![
        setting(theme, "resolution", true, label(theme, t("settings.defaultResolution"), None), resolution),
        setting(
            theme,
            "menu-branding",
            false,
            label(theme, t("settings.menuBranding"), Some(t("settings.menuBrandingDesc"))),
            switch(theme, "menu-branding-switch", &t("settings.menuBranding"), draft.menu_branding, |app| {
                let enabled = !app.settings_page.draft.menu_branding;
                save_one(app, move |settings| settings.menu_branding = enabled);
            }),
        ),
    ])
}

pub fn servers(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let desc_before = t("settings.ngrokDesc1");
    let desc_after = t("settings.ngrokDesc2");
    let desc = linked_small(theme, "ngrok-desc", &[(&format!("{desc_before} "), None), ("ngrok.com", Some(NGROK_AUTHTOKEN)), (&desc_after, None)]);
    let heading = rune::col().gap(2.0).grow(1.0).shrink().child(text(t("settings.ngrokAuthtoken")).font(theme.sans(13.5, 500.0)).color(theme.text)).child(desc);
    card(theme, "servers", t("settings.servers"), vec![setting(
        theme,
        "ngrok",
        true,
        heading,
        input(theme, &page.ngrok, "ngrok-input", t("settings.ngrokAuthtoken"), t("settings.ngrokPlaceholder"), Some(240.0), true, |app| &mut field(app).settings_page.ngrok),
    )])
}

/// The Giphy key row of Appearance.
pub fn giphy(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let desc = linked_small(theme, "giphy-desc", &[
        ("Animated stickers are off by default. Paste a free key from ", None),
        ("developers.giphy.com", Some(GIPHY_DEV)),
        (" to enable the Stickers tab. The emoji picker always works.", None),
    ]);
    let heading = rune::col().gap(2.0).grow(1.0).shrink().child(text(t("settings.giphyKey")).font(theme.sans(13.5, 500.0)).color(theme.text)).child(desc);
    setting(theme, "giphy", false, heading, input(theme, &page.giphy, "giphy-input", t("settings.giphyKey"), "Paste key to enable", Some(240.0), true, |app| &mut field(app).settings_page.giphy))
}
