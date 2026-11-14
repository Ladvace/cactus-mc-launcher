//! `src/routes/+layout.svelte`: background, the page, decor, the dock and every overlay, plus the
//! splash, UI zoom and the toaster (`Toaster.svelte`).

use rune::{Color, Cursor, Cx, El, HelpLook, Shadow, Tx, col, el, row, text};

use crate::app::App;
use crate::core::i18n::t;
use crate::core::toast::ToastKind;
use crate::icons;
use crate::theme::Theme;
use crate::ui::colored_icon;
use crate::{overlays, screens};

/// `<main class="content">` clears the dock by this much on its side.
pub const DOCK_CLEARANCE: f32 = 90.0;

/// The splash shows for 1.8 s and fades over the last third, as `splash-fade` does.
const SPLASH_SECS: f64 = 1.8;

#[derive(Default)]
pub struct State {
    splash_done: bool,
    toasts_expanded: bool,
    copied: Option<u64>,
}

pub fn poll(app: &mut App, now: f64, tx: &mut Tx) -> bool {
    if !app.shell.splash_done {
        if now >= SPLASH_SECS {
            app.shell.splash_done = true;
            return true;
        }
        tx.wake_after(std::time::Duration::from_secs_f64(SPLASH_SECS - now));
    }
    false
}

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let theme = app.theme();
    cx.zoom(app.settings.ui_scale.clamp(50, 200) as f32 / 100.0);
    cx.reduced_motion(app.settings.reduce_motion);
    cx.help_look(
        HelpLook::system(true)
            .fill(theme.bg_raised)
            .ink(theme.text)
            .ring(2.0, theme.border)
            .radius(0.0)
            .padding(9.0, 5.0)
            .font(theme.sans(11.0, 500.0))
            .max_width(Some(260.0))
            .shadows([Shadow { x: 0.0, y: 6.0, blur: 24.0, spread: 0.0, color: Color::BLACK.alpha(0.45), inset: false }]),
    );
    if app.settings.always_show_focus {
        cx.force_focus_rings();
    }
    cx.focus_ring(2.0, 2.0, theme.accent);

    let mut layer = Vec::new();
    let page = screens::view(app, cx, &mut layer);
    let dock = overlays::dock::view(app, cx, &mut layer);
    overlays::view(app, cx, &mut layer);
    if let Some(toaster) = toaster(app, &theme) {
        layer.push(toaster);
    }
    if !app.shell.splash_done {
        layer.push(splash(app, &theme));
    }

    let mut base = el()
        .key("app")
        .w_full()
        .h_full()
        .dir(if crate::core::i18n::is_rtl() { rune::Direction::Rtl } else { rune::Direction::Ltr })
        .child(overlays::extras::background(app, cx).key("bg").absolute(0.0, 0.0))
        .child(content(app, page));
    if let Some(decor) = overlays::extras::decor(app, cx) {
        base = base.child(decor.key("decor").absolute(0.0, 0.0).w_full().h_full().inert());
    }
    rune_kit::root(base.child(dock), layer)
}

fn content(app: &App, page: El) -> El {
    let container = el().key("content").w_full().h_full();
    let container = match app.settings.dock_position.as_str() {
        "top" => container.pt(DOCK_CLEARANCE),
        "left" => container.pl(DOCK_CLEARANCE),
        "right" => container.pr(DOCK_CLEARANCE),
        _ => container.pb(DOCK_CLEARANCE),
    };
    container.child(page)
}

const SPLASH_LOGO: &str = "/cactus-logo.png";

fn splash(app: &App, theme: &Theme) -> El {
    // `width: min(60%, 380px); height: auto`: the logo's own aspect (it's nearly square).
    let w = 380.0;
    let h = app.images.get(&app.fx, SPLASH_LOGO).and_then(|img| img.size()).map_or(w, |[iw, ih]| w * ih as f32 / iw.max(1) as f32);
    let logo = crate::ui::image(app, SPLASH_LOGO, w, h).transition(rune::Transition::scale(0.9));
    el().key("splash")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .fill(theme.bg_app)
        .on_press(|_: &mut App| {})
        .transition(rune::Transition::opacity().springs(rune_motion::SpringPair::new(rune::Spring::new(0.6, 0.0), rune::Spring::new(0.6, 0.0))))
        .child(logo)
}

const TOAST_OFFSET: f32 = 14.0;
const TOAST_GAP: f32 = 12.0;
const TOAST_H: f32 = 48.0;
const MAX_VISIBLE: usize = 3;

/// Newest in front; collapsed the cards peek above it, hovered they fan out and the timers pause.
fn toaster(app: &App, theme: &Theme) -> Option<El> {
    if app.toasts.list.is_empty() {
        return None;
    }
    let expanded = app.shell.toasts_expanded;
    let list: Vec<_> = app.toasts.list.iter().rev().collect();
    let height = if expanded {
        list.len() as f32 * (TOAST_H + TOAST_GAP)
    } else {
        TOAST_H + (list.len().min(MAX_VISIBLE) - 1) as f32 * TOAST_OFFSET
    };
    let mut stack = el()
        .key("toaster")
        .role("region")
        .label("Notifications")
        .w(380.0)
        .h(height)
        .motion(rune::Spring::new(0.25, 0.0))
        .on_hover_change(|app: &mut App, inside, _tx: &mut Tx| {
            app.shell.toasts_expanded = inside;
            app.toasts.paused = inside;
        });
    // Back to front, so the front card paints last.
    for (i, toast) in list.iter().enumerate().rev() {
        let y = if expanded { height - TOAST_H - i as f32 * (TOAST_H + TOAST_GAP) } else { height - TOAST_H - i as f32 * TOAST_OFFSET };
        let scale = if expanded { 1.0 } else { (1.0 - i as f32 * 0.05).max(0.82) };
        let visible = expanded || i < MAX_VISIBLE;
        let (edge, glyph): (Color, El) = match toast.kind {
            ToastKind::Success => (theme.accent, colored_icon(&icons::CHECK, 13.0, theme.accent_contrast)),
            ToastKind::Error => (theme.danger, text("!").font(theme.pixel(13.0)).color(theme.accent_contrast)),
            ToastKind::Info => (theme.border, colored_icon(&icons::CLOCK, 13.0, theme.accent_contrast)),
        };
        let id = toast.id;
        let mut actions = row().gap(6.0).center_y();
        if let Some(action) = &toast.action {
            let run = action.run.clone();
            actions = actions.child(action_button(theme, &format!("act-{id}"), action.label.clone()).on_press(move |app: &mut App| {
                app.toasts.dismiss(id);
                run(app);
            }));
        }
        if toast.kind == ToastKind::Error {
            let msg = toast.message.clone();
            let label = if app.shell.copied == Some(id) { t("toaster.copied") } else { t("toaster.copy") };
            actions = actions.child(action_button(theme, &format!("copy-{id}"), label).on_press_tx(move |app: &mut App, tx| {
                tx.copy(msg.clone());
                app.shell.copied = Some(id);
            }));
        }
        actions = actions.child(
            el().key(format!("close-{id}"))
                .px(6.0)
                .py(2.0)
                .role("button")
                .label(t("toaster.dismiss"))
                .cursor(Cursor::Pointer)
                .hover(theme.bg_hover)
                .on_press(move |app: &mut App| app.toasts.dismiss(id))
                .child(text("✕").font(theme.sans(12.0, 400.0)).color(theme.text_muted)),
        );
        let card = row()
            .key(format!("toast-{id}"))
            .role(if toast.kind == ToastKind::Error { "alert" } else { "status" })
            .w_full()
            .min_h(TOAST_H)
            .absolute(0.0, y)
            .gap(10.0)
            .center_y()
            .px(12.0)
            .py(11.0)
            .fill(theme.bg_raised)
            .border(2.0, edge)
            .shadow(6.0, 24.0, theme.shadow)
            .scale(scale)
            .origin(0.5, 1.0)
            .opacity(if visible { 1.0 } else { 0.0 })
            .motion(rune::Spring::new(0.25, 0.0))
            .transition(rune::Transition::offset(0.0, 16.0).combined(rune::Transition::opacity()))
            .child(el().size(20.0, 20.0).center().fill(edge).child(glyph))
            .child(text(toast.message.clone()).font(theme.sans(12.5, 500.0)).color(theme.text).grow(1.0).shrink().max_lines(3))
            .child(actions);
        stack = stack.child(if i == 0 || expanded { card } else { card.inert() });
    }
    Some(
        col().key("toaster-wrap").absolute(0.0, 0.0).w_full().h_full().items_end().justify_end().pr(18.0).pb(100.0).child(stack),
    )
}

fn action_button(theme: &Theme, key: &str, label: String) -> El {
    el().key(key.to_string())
        .px(8.0)
        .py(3.0)
        .role("button")
        .label(label.clone())
        .cursor(Cursor::Pointer)
        .fill(theme.bg_card)
        .border(1.0, theme.border)
        .hover(theme.bg_hover)
        .child(text(label).font(theme.sans(11.0, 600.0)).color(theme.text_secondary))
}
