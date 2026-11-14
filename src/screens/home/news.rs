//! `NewsSection.svelte`: the news strip from `get_news`, with paging, the one-or-three layout
//! toggle, refresh and hide.

use rune::{Color, Cursor, El, SurfaceFit, Tx, col, el, row, text};

use crate::app::App;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::news::NewsItem;
use crate::theme::Theme;
use crate::ui::{self, fmt};

#[derive(Default)]
pub struct State {
    pub items: Vec<NewsItem>,
    pub loading: bool,
    pub failed: bool,
    pub page: usize,
    pub started: bool,
}

pub fn load(app: &mut App, force: bool) {
    app.home.news.loading = true;
    app.home.news.failed = false;
    app.home.news.started = true;
    app.fx.spawn(async move { crate::commands::get_news(force).await }, |app, result| {
        let news = &mut app.home.news;
        news.loading = false;
        match result {
            Ok(items) => {
                news.items = items;
                news.page = 0;
            }
            Err(_) => news.failed = true,
        }
    });
}

fn per_page(app: &App) -> usize {
    if app.settings.news_single { 1 } else { 3 }
}

fn pages(app: &App) -> Vec<&[NewsItem]> {
    app.home.news.items.chunks(per_page(app)).collect()
}

fn turn_page(app: &mut App, delta: isize) {
    let page_count = app.home.news.items.len().div_ceil(per_page(app));
    if page_count == 0 {
        return;
    }
    app.home.news.page = ((app.home.news.page as isize + delta).rem_euclid(page_count as isize)) as usize;
}

fn nav(theme: &Theme, key: &str, icon: &'static rune::Icon, size: f32, help: String, disabled: bool) -> El {
    let button = el()
        .key(key.to_string())
        .size(26.0, 26.0)
        .center()
        .fill(theme.bg_card)
        .border(1.0, theme.border)
        .hover_group()
        .role("button")
        .label(help.clone())
        .help(help)
        .child(ui::colored_icon(icon, size, theme.text_muted).group_color(theme.accent));
    if disabled { button.opacity(0.4).disabled(true) } else { button.focusable(true).cursor(Cursor::Pointer) }
}

fn open_link(tx: &mut Tx, item: &NewsItem) {
    if let Some(link) = item.link.as_deref().filter(|link| !link.is_empty()) {
        let _ = tx.open_url(link);
    }
}

/// The section, or None when it's hidden, failed or has nothing to show.
pub fn view(app: &App, width: f32) -> Option<El> {
    let news = &app.home.news;
    if !app.settings.show_news || news.failed || (!news.loading && news.items.is_empty()) {
        return None;
    }
    let theme = app.theme();
    let single = app.settings.news_single;
    let pages = pages(app);
    let page = news.page.min(pages.len().saturating_sub(1));

    let mut actions = row().gap(6.0).center_y();
    if pages.len() > 1 {
        actions = actions
            .child(nav(&theme, "news-prev", &icons::CHEVRON_LEFT, 14.0, tr("news.previous"), false).on_press(|app: &mut App| turn_page(app, -1)))
            .child(nav(&theme, "news-next", &icons::CHEVRON_RIGHT, 14.0, tr("news.next"), false).on_press(|app: &mut App| turn_page(app, 1)));
    }
    actions = actions
        .child(
            nav(&theme, "news-layout", if single { &icons::GRID } else { &icons::EXPAND }, 14.0,
                if single { tr("news.layoutSingle") } else { tr("news.layoutMulti") }, false)
                .on_press(|app: &mut App| app.update_settings(|settings| settings.news_single = !settings.news_single)),
        )
        .child({
            let refresh = nav(&theme, "news-refresh", &icons::REFRESH, 13.0, tr("news.refresh"), news.loading);
            if news.loading { refresh } else { refresh.on_press(|app: &mut App| load(app, true)) }
        })
        .child(nav(&theme, "news-hide", &icons::CLOSE, 14.0, tr("news.hide"), false).on_press(|app: &mut App| app.update_settings(|settings| settings.show_news = false)));

    let head = row()
        .key("news-head")
        .justify_between()
        .center_y()
        .gap(10.0)
        .mb(12.0)
        .child(ui::heading(&theme, tr("news.title"), 15.0))
        .child(actions);

    const NEWS_H: f32 = 233.0;
    let stacked = width < 760.0 - 64.0;
    let inner = width - 12.0;
    let (feature_w, column_w) = if single || stacked { (width, width) } else { (inner * 0.6, inner * 0.4) };

    let mut section = col().key("news").w_full().mt(10.0).mb(30.0).child(head);
    if news.loading && news.items.is_empty() {
        let mut strip = row().key("strip").gap(12.0).child(
            el().size(feature_w, NEWS_H).border(2.0, theme.border).child(shimmer(&theme).w_full().h_full()),
        );
        if !stacked {
            let mut placeholders = col().w(column_w).h(NEWS_H).gap(12.0);
            for i in 0..2 {
                placeholders = placeholders.child(
                    row().key(format!("sk{i}")).w_full().h((NEWS_H - 12.0) / 2.0).gap(10.0).padding(8.0)
                        .fill(theme.bg_card).border(2.0, theme.border)
                        .child(shimmer(&theme).w(96.0).h_full())
                        .child(col().grow(1.0).gap(6.0).py(4.0)
                            .child(el().w_full().h(10.0).fill(theme.border).radius(2.0))
                            .child(el().w_frac(0.55).h(10.0).fill(theme.border).radius(2.0))),
                );
            }
            strip = strip.child(placeholders);
        }
        return Some(section.child(strip));
    }
    let Some(current) = pages.get(page) else { return Some(section) };
    let Some(featured) = current.first() else { return Some(section) };

    let mut strip = if stacked && !single { col() } else { row() }.key(format!("strip-{page}")).gap(12.0).transition(rune::Transition::opacity());
    strip = strip.child(feature(app, &theme, featured, feature_w, NEWS_H));
    if !single {
        let mut minis = col().key("minis").w(column_w).h(NEWS_H).gap(12.0);
        for item in &current[1..] {
            minis = minis.child(mini(app, &theme, item, column_w, (NEWS_H - 12.0) / 2.0));
        }
        strip = strip.child(minis);
    }
    section = section.child(strip);

    if pages.len() > 1 {
        let mut dots = row().key("dots").role("tablist").label(tr("news.pages")).justify_center().gap(7.0).mt(12.0);
        for i in 0..pages.len() {
            let active = i == page;
            dots = dots.child(
                el().key(format!("dot{i}"))
                    .size(if active { 22.0 } else { 9.0 }, 9.0)
                    .fill(if active { theme.accent } else { theme.border })
                    .border(1.0, if active { theme.accent } else { theme.border })
                    .motion(rune::Spring::new(0.15, 0.0))
                    .role("tab")
                    .checked(active)
                    .label(tf("news.pageN", &[("n", &(i + 1).to_string())]))
                    .focusable(true)
                    .cursor(Cursor::Pointer)
                    .on_press(move |app: &mut App| app.home.news.page = i),
            );
        }
        section = section.child(dots);
    }
    Some(section)
}

fn shimmer(theme: &Theme) -> El {
    el().linear_gradient(std::f32::consts::FRAC_PI_2, [(0.0, theme.bg_app), (0.5, theme.bg_card), (1.0, theme.bg_app)])
        .looping(rune_motion::LoopSpec::opacity(1.3, 0.0, 0.6, 1.0))
}

fn date(app: &App, iso: &str) -> String {
    let formatted = fmt::date_str(iso, &app.settings.date_format);
    if formatted.is_empty() { iso.to_string() } else { formatted }
}

fn feature(app: &App, theme: &Theme, item: &NewsItem, w: f32, h: f32) -> El {
    let has_link = item.link.as_deref().is_some_and(|link| !link.is_empty());
    let img = item.image.as_deref().and_then(|src| app.images.get(&app.fx, src));
    let mut card = ui::bevel(el().key("feature").size(w, h).border(2.0, theme.border).fill(theme.bg_app), 2.0, 0.03, 0.25)
        .role("button")
        .label(item.title.clone());
    card = match (&img, item.image.is_some()) {
        (Some(img), _) => card.child(el().size(w - 4.0, h - 4.0).absolute(0.0, 0.0).pixels_fit(img.pixels(), SurfaceFit::Fill)),
        (None, false) => card.linear_gradient(135_f32.to_radians(), [(0.0, theme.bg_card), (1.0, theme.bg_app)]),
        (None, true) => card,
    };
    let mut scrim = col()
        .key("scrim")
        .w(w - 4.0)
        .absolute(0.0, 0.0)
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .gap(7.0)
        .pt(16.0)
        .px(18.0)
        .pb(15.0)
        .linear_gradient(0.0, [(0.12, Color::BLACK.alpha(0.9)), (0.55, Color::BLACK.alpha(0.55)), (1.0, Color::BLACK.alpha(0.0))]);
    if !item.category.is_empty() {
        scrim = scrim.child(text(item.category.to_uppercase()).font(rune::tok::TextStyle { tracking: 0.6, ..theme.sans(10.0, 600.0) }).color(theme.accent));
    }
    scrim = scrim.child(text(item.title.clone()).font(rune::tok::TextStyle { line_height: 19.0 * 1.2, ..theme.pixel(19.0) }).color(Color::WHITE).wrap().max_lines(2));
    if !item.summary.is_empty() {
        scrim = scrim.child(
            text(item.summary.clone())
                .font(rune::tok::TextStyle { line_height: 12.5 * 1.45, ..theme.sans(12.5, 400.0) })
                .color(Color::WHITE.alpha(0.82))
                .wrap()
                .max_lines(2),
        );
    }
    let mut meta = row().justify_between().center_y().gap(10.0).mt(2.0).child(text(date(app, &item.date)).font(theme.sans(11.0, 400.0)).color(Color::WHITE.alpha(0.65)));
    if has_link {
        meta = meta.child(text(tr("news.readMore")).font(theme.sans(11.0, 400.0)).color(theme.accent));
    }
    card = card.child(scrim.child(meta));
    if has_link {
        let item = item.clone();
        card = card
            .hover_group()
            .child(el().size(w, h).absolute(-2.0, -2.0).border(2.0, theme.accent).group_opacity(0.0, 1.0).a11y_hidden())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press_tx(move |_: &mut App, tx: &mut Tx| open_link(tx, &item));
    }
    card
}

fn mini(app: &App, theme: &Theme, item: &NewsItem, w: f32, h: f32) -> El {
    let has_link = item.link.as_deref().is_some_and(|link| !link.is_empty());
    let thumb = match item.image.as_deref() {
        Some(src) => {
            let frame = el().w(96.0).h_full().fill(theme.bg_app).border(1.0, theme.border).clip_corners([0.0; 4]);
            match app.images.get(&app.fx, src) {
                Some(img) => frame.child(el().w(94.0).h(h - 4.0 - 16.0 - 2.0).pixels_fit(img.pixels(), SurfaceFit::Fill)),
                None => frame,
            }
        }
        None => el().w(96.0).h_full().center().fill(theme.bg_app).border(1.0, theme.border).child(ui::colored_icon(&icons::GLOBE, 18.0, theme.text_muted)),
    };
    let mut body = col().grow(1.0).min_w(0.0).gap(4.0).py(2.0).pr(2.0);
    if !item.category.is_empty() {
        body = body.child(text(item.category.to_uppercase()).font(rune::tok::TextStyle { tracking: 0.6, ..theme.sans(10.0, 600.0) }).color(theme.accent));
    }
    body = body
        .child(text(item.title.clone()).font(rune::tok::TextStyle { line_height: 12.5 * 1.3, ..theme.sans(12.5, 600.0) }).color(theme.text).wrap().max_lines(3))
        .child(el().grow(1.0));
    let mut meta = row().justify_between().center_y().gap(8.0).child(text(date(app, &item.date)).font(theme.sans(10.5, 400.0)).color(theme.text_muted));
    if has_link {
        meta = meta.child(text(tr("news.read")).font(theme.sans(10.5, 400.0)).color(theme.accent));
    }
    body = body.child(meta);
    let mut card = ui::bevel(row().key(format!("mini-{}", item.id)).size(w, h).gap(10.0).padding(8.0).fill(theme.bg_card).border(2.0, theme.border), 2.0, 0.03, 0.22)
        .role("button")
        .label(item.title.clone())
        .child(thumb)
        .child(body);
    if has_link {
        let item = item.clone();
        card = card
            .hover_group()
            .child(el().size(w, h).absolute(-2.0, -2.0).border(2.0, theme.accent).group_opacity(0.0, 1.0).a11y_hidden())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press_tx(move |_: &mut App, tx: &mut Tx| open_link(tx, &item));
    }
    card
}
