//! The page's own controls from its `<style>`: `.card-block`, `.setting`, `.label`, `.switch`,
//! `.seg`, `.chip`, the `range` input and the `Select` / LanguageSelect dropdowns.

use std::rc::Rc;

use rune::{Align, Color, Cursor, Drag, El, Link, Phase, Side, Span, Tx, col, el, row, text};

use crate::app::App;
use crate::icons;
use crate::theme::Theme;
use crate::ui;

/// `.card-block` with its `h3`.
pub fn card(theme: &Theme, key: &str, title: impl Into<String>, rows: Vec<El>) -> El {
    card_with_head(theme, key, h3(theme, title), rows)
}

pub fn card_with_head(theme: &Theme, key: &str, head: El, rows: Vec<El>) -> El {
    ui::bevel(col().key(key.to_string()).w_full().fill(theme.bg_card).border(2.0, theme.border), 2.0, 0.04, 0.28)
        .px(20.0)
        .py(18.0)
        .mb(18.0)
        .child(head)
        .children(rows)
}

pub fn h3(theme: &Theme, title: impl Into<String>) -> El {
    text(title.into()).font(theme.pixel(14.0)).color(theme.text).mb(8.0)
}

/// `.label`: the setting's name and its `small` line.
pub fn label(theme: &Theme, title: impl Into<String>, desc: Option<String>) -> El {
    let mut labels = col().gap(2.0).grow(1.0).shrink().min_w(0.0).child(text(title.into()).font(theme.sans(13.5, 500.0)).color(theme.text));
    if let Some(desc) = desc {
        labels = labels.child(small(theme, desc));
    }
    labels
}

pub fn small(theme: &Theme, content: impl Into<String>) -> El {
    text(content.into()).font(rune::tok::TextStyle { line_height: 12.0 * 1.4, ..theme.sans(12.0, 400.0) }).color(theme.text_muted).wrap().max_w(420.0)
}

/// `.setting`: label left, control right, a hairline above unless it follows the `h3`.
pub fn setting(theme: &Theme, key: &str, first: bool, label: El, control: El) -> El {
    let line = row().key(key.to_string()).w_full().center_y().justify_between().gap(20.0).py(14.0).child(label).child(control);
    if first { line } else { line.border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle) }
}

/// `.switch`: a 44×24 track, a 16 pt thumb that slides 20 pt when on.
pub fn switch(theme: &Theme, key: &str, label: &str, on: bool, f: impl Fn(&mut App) + 'static) -> El {
    // Rune draws the 2 pt border inside the box and `absolute` measures from the outer edge,
    // so CSS's `top: 2px` inside the border is 4 pt here.
    let inset = 4.0;
    let thumb = el()
        .key("thumb")
        .size(16.0, 16.0)
        .absolute(if on { 44.0 - 16.0 - inset } else { inset }, inset)
        .fill(if on { theme.accent } else { theme.text_muted })
        .motion(rune::Spring::new(0.14, 0.0));
    el().key(key.to_string())
        .size(44.0, 24.0)
        .fill(if on { theme.accent_soft } else { theme.bg_input })
        .border(2.0, if on { theme.accent } else { theme.border })
        .role("switch")
        .checked(on)
        .label(label.to_string())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| f(app))
        .child(thumb)
}

/// `.seg`: buttons joined edge to edge, the chosen one in accent.
pub fn seg(theme: &Theme, key: &str, items: Vec<(String, bool, Rc<dyn Fn(&mut App)>)>) -> El {
    let mut group = row().key(key.to_string());
    for (i, (name, on, f)) in items.into_iter().enumerate() {
        let mut button = row()
            .key(format!("{key}-{i}"))
            .px(14.0)
            .py(8.0)
            .fill(if on { theme.accent_soft } else { theme.bg_input })
            .border(2.0, if on { theme.accent } else { theme.border })
            .role("button")
            .checked(on)
            .label(name.clone())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| f(app))
            .child(text(name).font(theme.sans(12.5, 600.0)).color(if on { theme.accent } else { theme.text_secondary }).a11y_hidden());
        if i > 0 {
            button = button.ml(-2.0);
        }
        group = group.child(button);
    }
    group
}

/// `.chip`.
pub fn chip(theme: &Theme, key: &str, name: impl Into<String>, on: bool, f: impl Fn(&mut App) + 'static) -> El {
    let name = name.into();
    row()
        .key(key.to_string())
        .px(12.0)
        .py(7.0)
        .fill(if on { theme.accent_soft } else { theme.bg_input })
        .border(2.0, if on { theme.accent } else { theme.border })
        .box_shadow(ui::inset_shadow(2.0, 2.0, Color::BLACK.alpha(0.22)))
        .role("button")
        .checked(on)
        .label(name.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| f(app))
        .child(text(name).font(theme.sans(12.5, 600.0)).color(if on { theme.accent } else { theme.text_secondary }).a11y_hidden())
}

/// `input[type=range]`: `value` 0..=1 snapped to `snap` steps (0 = continuous), `ticks` > 0
/// draws the `.stepped` marks. `on_change` gets the snapped fraction while dragging.
pub fn range(
    theme: &Theme,
    key: &str,
    width: f32,
    value: f32,
    snap: u32,
    ticks: u32,
    disabled: bool,
    on_change: impl Fn(&mut App, f32) + 'static,
) -> El {
    let clamped = value.clamp(0.0, 1.0);
    let thumb = 14.0;
    let travel = width - thumb;
    let snapped = move |fraction: f32| if snap > 0 { (fraction * snap as f32).round() / snap as f32 } else { fraction };
    let fraction_at = move |x: f32| snapped(((x - thumb / 2.0) / travel).clamp(0.0, 1.0));
    let mut track = el()
        .key("track")
        .size(width, 6.0)
        .absolute(0.0, 6.0)
        .fill(theme.border)
        .box_shadow(ui::inset_shadow(1.0, 1.0, Color::BLACK.alpha(0.28)))
        .child(el().size(thumb / 2.0 + clamped * travel, 6.0).fill(theme.accent));
    for i in 1..ticks {
        track = track.child(el().size(2.0, 6.0).absolute(i as f32 * width / ticks as f32, 0.0).fill(Color::BLACK.alpha(0.42)));
    }
    let mut slider = el()
        .key(key.to_string())
        .size(width, 18.0)
        .role("slider")
        .range_value(f64::from(clamped), 0.0, 1.0)
        .child(track)
        .child(
            el().key("thumb")
                .size(thumb, thumb)
                .absolute(clamped * travel, 2.0)
                .fill(theme.accent)
                .border(2.0, theme.accent_contrast)
                .shadow(0.0, 0.0, theme.border),
        );
    if disabled {
        return slider.opacity(0.45).disabled(true);
    }
    let on_change = Rc::new(on_change);
    slider = slider
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_drag({
            let on_change = on_change.clone();
            move |app: &mut App, drag: &mut Drag, tx| {
                if matches!(drag.phase, Phase::Down | Phase::Move) {
                    on_change(app, fraction_at(drag.x));
                    tx.instant();
                }
            }
        })
        .on_step(move |app: &mut App, n, _| {
            let step = if snap > 0 { 1.0 / snap as f32 } else { 0.05 };
            on_change(app, snapped((clamped + n as f32 * step).clamp(0.0, 1.0)));
        });
    slider
}

/// `.mem-ticks`: labels under a `range`, at fractions of its width; the first starts there,
/// the last ends there, the rest centre.
pub fn ticks(theme: &Theme, width: f32, items: Vec<(f32, String, Option<String>)>) -> El {
    let n = items.len();
    let mut labels = el().w(width).h(14.0).mt(4.0);
    for (i, (frac, main, unit)) in items.into_iter().enumerate() {
        let box_w = 48.0;
        let x = frac * width;
        let (left, align) = if i == 0 {
            (x, Align::Start)
        } else if i + 1 == n {
            (x - box_w, Align::End)
        } else {
            (x - box_w / 2.0, Align::Center)
        };
        let mut caption = main.clone();
        let mut spans = vec![Span::new(0, caption.len())];
        if let Some(unit) = unit {
            let start = caption.len();
            caption.push_str(&unit);
            spans.push(Span::new(start, caption.len()).size(8.0).color(theme.text_muted.alpha(0.7)));
        }
        labels = labels.child(
            text(caption).font(theme.sans(10.0, 400.0)).color(theme.text_muted).spans(spans).align(align).w(box_w).absolute(left, 0.0),
        );
    }
    labels
}

/// A `small` line with inline `.linkish` buttons: `parts` are (text, url) where a url makes it a link.
pub fn linked_small(theme: &Theme, key: &str, parts: &[(&str, Option<&'static str>)]) -> El {
    let mut content = String::new();
    let mut spans = Vec::new();
    let mut links = Vec::new();
    for (part, url) in parts {
        let start = content.len();
        content.push_str(part);
        if let Some(link_url) = url {
            let mut span = Span::new(start, content.len()).color(theme.accent);
            span.underline = Some(theme.accent);
            spans.push(span);
            links.push(Link::new(start..content.len(), *link_url));
        }
    }
    let mut paragraph = text(content)
        .key(key.to_string())
        .font(rune::tok::TextStyle { line_height: 12.0 * 1.4, ..theme.sans(12.0, 400.0) })
        .color(theme.text_muted)
        .wrap()
        .max_w(420.0)
        .spans(spans);
    for link in links {
        paragraph = paragraph.link(link);
    }
    paragraph.on_link(|_: &mut App, url: &str, tx: &mut Tx| {
        let _ = tx.open_url(url);
    })
}

/// `Select.svelte` / LanguageSelect: the `.trigger` and, while open, the `.menu` on `layer`
/// under it. `which` names it in `State::open`; `options` are (value, label); `lead` draws
/// what goes before a label (LanguageSelect's flag).
pub fn dropdown(
    theme: &Theme,
    layer: &mut Vec<El>,
    which: &'static str,
    width: f32,
    label: &str,
    open: bool,
    current: &str,
    options: Vec<(String, String)>,
    lead: Option<&dyn Fn(&str) -> El>,
    choose: impl Fn(&mut App, &str) + 'static,
) -> El {
    let trigger_key = format!("{which}-trigger");
    let selected = options.iter().position(|(value, _)| value == current);
    let flags = lead.is_some();
    let mut trigger = row()
        .key(trigger_key.clone())
        .w(width)
        .center_y()
        .gap(if flags { 9.0 } else { 8.0 })
        .px(if flags { 10.0 } else { 12.0 })
        .py(if flags { 8.0 } else { 9.0 })
        .fill(theme.bg_input)
        .border(2.0, if open { theme.accent } else { theme.border })
        .box_shadow(ui::inset_shadow(2.0, 2.0, Color::BLACK.alpha(0.28)))
        .role("combobox")
        .label(label.to_string())
        .expanded(open)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| {
            app.settings_page.open = if app.settings_page.open == Some(which) { None } else { Some(which) };
        });
    if let (Some(i), Some(draw_lead)) = (selected, lead) {
        trigger = trigger.child(draw_lead(&options[i].0));
    }
    let selected_label = selected.map(|i| options[i].1.clone()).unwrap_or_default();
    trigger = trigger
        .child(text(selected_label).font(theme.sans(13.0, 400.0)).color(theme.text).grow(1.0).shrink().truncate())
        .child(ui::colored_icon(if open { &icons::CHEVRON_UP } else { &icons::CHEVRON_DOWN }, 12.0, theme.text_muted));
    if !open {
        return trigger;
    }
    let choose = Rc::new(choose);
    let mut menu = col()
        .key(format!("{which}-menu"))
        .w(width)
        .padding(4.0)
        .gap(2.0)
        .fill(theme.bg_raised)
        .border(2.0, theme.border)
        .shadow(12.0, 30.0, Color::BLACK.alpha(0.5))
        .anchored(trigger_key, Side::Bottom, Align::Start, 4.0)
        .modal(true, |app: &mut App| app.settings_page.open = None);
    for (i, (value, name)) in options.into_iter().enumerate() {
        let is_selected = Some(i) == selected;
        let choose = choose.clone();
        let mut item = row()
            .key(format!("{which}-opt-{value}"))
            .w_full()
            .center_y()
            .gap(9.0)
            .px(10.0)
            .py(8.0)
            .hover(theme.bg_hover)
            .role("option")
            .checked(is_selected)
            .label(name.clone())
            .focusable(true)
            .cursor(Cursor::Pointer);
        if let Some(draw_lead) = lead {
            item = item.child(draw_lead(&value));
        }
        item = item
            .child(text(name).font(theme.sans(13.0, 400.0)).color(if is_selected { theme.accent } else { theme.text }).grow(1.0).shrink().truncate())
            .on_press(move |app: &mut App| {
                app.settings_page.open = None;
                choose(app, &value);
            });
        if is_selected {
            item = item.child(ui::colored_icon(&icons::CHECK, 12.0, theme.accent));
        }
        menu = menu.child(item);
    }
    layer.push(menu);
    trigger
}
