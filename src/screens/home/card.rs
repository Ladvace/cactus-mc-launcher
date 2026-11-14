//! `InstanceCard.svelte` and `InstanceCardSkeleton.svelte`.

use rune::tok::TextStyle;
use rune::{Color, Cursor, El, PointerPress, Shadow, SurfaceFit, col, el, row, text};

use crate::app::{App, Route};
use crate::core::i18n::t as tr;
use crate::icons;
use crate::instance::{Instance, InstanceKind};
use crate::theme::{Theme, parse_color};
use crate::ui;

use super::grid::{GAP, PITCH};

/// The `.card` look: card fill, 2 pt border, the soft inner bevel.
pub(super) fn card_face(theme: &Theme) -> El {
    ui::bevel(col().fill(theme.bg_card).border(2.0, theme.border), 2.0, 0.04, 0.28)
}

/// HomeGrid's `iconFor`, from the card's size in grid cells.
fn icon_size(w: f32, h: f32) -> f32 {
    let span = |size: f32| ((size + GAP) / PITCH).round().max(1.0);
    let (span_w, span_h) = (span(w), span(h));
    if span_w.min(span_h) >= 2.0 {
        120.0
    } else if span_w.max(span_h) >= 2.0 {
        84.0
    } else {
        60.0
    }
}

/// `InstanceCard.svelte`, also used by the folder overlay: the card for `inst` at `w`×`h`.
pub fn instance_card(app: &App, inst: &Instance, w: f32, h: f32) -> El {
    let theme = app.theme();
    let id = inst.id.clone();
    let (busy, running) = (app.is_busy(&id), app.is_running(&id));
    let cover_src = if inst.cover_image { app.instance_icon_src(inst) } else { None };
    let cover = cover_src.as_deref().and_then(|src| app.images.get(&app.fx, src));

    let mut card = card_face(&theme)
        .key(format!("card-{id}"))
        .size(w, h)
        .hover_group()
        .hover(theme.bg_hover)
        .role("button")
        .label(inst.name.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press({
            let id = id.clone();
            move |app: &mut App| {
                if !std::mem::take(&mut app.home.suppress_click) {
                    app.go(Route::Instance(id.clone()));
                }
            }
        })
        .on_secondary_press({
            let id = id.clone();
            move |app: &mut App, press: PointerPress, _| app.open_instance_menu(&id, press.x, press.y)
        });

    if let Some(img) = &cover {
        card = card
            .child(el().key("cover").size(w - 4.0, h - 4.0).absolute(0.0, 0.0).pixels_fit(img.pixels(), SurfaceFit::Fill))
            .child(
                el().key("scrim").size(w - 4.0, h - 4.0).absolute(0.0, 0.0).linear_gradient(
                    0.0,
                    [(0.0, Color::BLACK.alpha(0.8)), (0.32, Color::BLACK.alpha(0.35)), (0.6, Color::BLACK.alpha(0.0))],
                ),
            )
            .child(el().grow(1.0))
            .child(meta(&theme, inst, true).px(12.0).py(10.0));
    } else {
        card = card
            .padding(14.0)
            .gap(12.0)
            .child(el().key("art").grow(1.0).min_h(0.0).center().child(ui::instance_icon(app, inst, icon_size(w, h))))
            .child(meta(&theme, inst, false));
    }

    // The accent border the card takes under the pointer.
    card = card.child(
        el().key("ring").size(w, h).absolute(-2.0, -2.0).border(2.0, theme.accent).group_opacity(0.0, 1.0).a11y_hidden(),
    );
    if inst.kind == InstanceKind::Server {
        card = card.child(
            el().key("kind").absolute(4.0, 4.0).px(6.0).py(2.0).fill(theme.accent).help(tr("card.serverTitle")).child(
                text(tr("card.serverBadge"))
                    .font(TextStyle { tracking: 8.0 * 0.08, ..theme.pixel(8.0) })
                    .color(theme.bg_app),
            ),
        );
    }
    card = card.child(play_button(&theme, &id, busy, running, w));
    if app.install.is_installing(&id) {
        card = card.child(install_overlay(app, &theme, &id, w, h));
    }
    card
}

fn meta(theme: &Theme, inst: &Instance, on_cover: bool) -> El {
    let (name_color, sub_color) = if on_cover { (Color::WHITE, Color::WHITE.alpha(0.85)) } else { (theme.text, theme.text_muted) };
    col()
        .key("meta")
        .gap(2.0)
        .min_w(0.0)
        .child(text(inst.name.clone()).font(theme.sans(14.0, 600.0)).color(name_color).truncate().help(inst.name.clone()))
        .child(
            row()
                .gap(5.0)
                .center_y()
                .min_w(0.0)
                .child(ui::loader_icon(inst.loader, 13.0, sub_color))
                .child(
                    text(format!("{} · {}", inst.loader.label(), inst.mc_version))
                        .font(theme.sans(12.0, 400.0))
                        .color(sub_color)
                        .truncate(),
                ),
        )
}

fn play_button(theme: &Theme, id: &str, busy: bool, running: bool, w: f32) -> El {
    let (face, ink) = if running { (theme.danger, Color::WHITE) } else { (theme.accent, theme.accent_contrast) };
    let edge = parse_color("#b98a1e").unwrap_or(theme.edge);
    let glyph = if busy {
        ui::spinner_with(15.0, Color::WHITE.alpha(0.35), Color::WHITE)
    } else if running {
        ui::colored_icon(&icons::STOP, 16.0, ink)
    } else {
        ui::colored_icon(&icons::PLAY, 18.0, ink)
    };
    let label = if running { tr("card.stop") } else { tr("card.play") };
    let help = if running {
        tr("card.running")
    } else if busy {
        tr("card.preparing")
    } else {
        tr("card.play")
    };
    let visible = busy || running;
    let id = id.to_string();
    el().key("play")
        .size(34.0, 34.0)
        .absolute(w - 4.0 - 8.0 - 34.0, 6.0)
        .center()
        .fill(face)
        .border(2.0, edge)
        .box_shadow(Shadow { x: 0.0, y: 3.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.4), inset: false })
        .box_shadow(Shadow { x: 1.0, y: 1.0, blur: 0.0, spread: 0.0, color: Color::WHITE.alpha(0.4), inset: true })
        .group_opacity(if visible { 1.0 } else { 0.0 }, 1.0)
        .role("button")
        .label(label)
        .help(help)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .press_scale(0.94)
        .on_press(move |app: &mut App| {
            if app.is_running(&id) {
                app.stop_instance(&id);
            } else if !app.is_busy(&id) {
                app.launch_instance(&id);
            }
        })
        .child(glyph)
}

fn install_overlay(app: &App, theme: &Theme, id: &str, w: f32, h: f32) -> El {
    let pct = app.install.percent(id);
    let msg = app.install.progress.get(id).map(|progress| progress.message.clone()).filter(|message| !message.is_empty());
    let mut overlay = col()
        .key("installing")
        .size(w - 4.0, h - 4.0)
        .absolute(0.0, 0.0)
        .center()
        .gap(6.0)
        .padding(8.0)
        .fill(parse_color("#0a0908").unwrap_or(Color::BLACK).alpha(0.82))
        .on_press(|_: &mut App| {})
        .child(
            ui::spinner_with(30.0, Color::WHITE.alpha(0.2), theme.accent),
        );
    if let Some(percent) = pct {
        overlay = overlay.child(text(format!("{percent}%")).font(theme.pixel(16.0)).color(theme.accent));
    }
    overlay.child(
        text(msg.unwrap_or_else(|| tr("card.downloading")))
            .font(theme.sans(11.0, 400.0))
            .color(theme.text_secondary)
            .truncate()
            .max_w(w - 24.0),
    )
}

/// `InstanceCardSkeleton.svelte`, at `w`×`h`.
pub fn skeleton_card(app: &App, i: usize, w: f32, h: f32) -> El {
    let theme = app.theme();
    card_face(&theme)
        .key(format!("skeleton-{i}"))
        .size(w, h)
        .padding(14.0)
        .gap(12.0)
        .child(el().grow(1.0).center().py(6.0).child(ui::skeleton(&theme, Some(72.0), 72.0)))
        .child(
            col()
                .gap(6.0)
                .child(ui::skeleton(&theme, Some((w - 32.0) * 0.72), 14.0))
                .child(ui::skeleton(&theme, Some((w - 32.0) * 0.48), 11.0)),
        )
}
