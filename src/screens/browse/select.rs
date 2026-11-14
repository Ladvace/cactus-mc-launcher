//! `Select.svelte` as Browse uses it: an `.input`-styled trigger and a listbox under it, with an
//! optional search row (the game-version select). Which one is open lives in `app.browse.open_select`.

use std::rc::Rc;

use rune::{Align, Color, Cursor, El, ScrollSpec, Side, col, el, row, text};

use crate::app::App;
use crate::core::i18n::t as tr;
use crate::icons;
use crate::theme::Theme;
use crate::ui;

pub struct Opt {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl Opt {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Opt {
        Opt { value: value.into(), label: label.into(), disabled: false }
    }
}

/// The 2 pt accent border `:hover` draws, shown while the nearest hover group is hovered.
pub fn hover_ring(color: Color) -> El {
    el().absolute(0.0, 0.0).w_full().h_full().border(2.0, color).group_opacity(0.0, 1.0)
}

/// `inset 2px 2px 0 rgba(0,0,0,.28)`, the sunken look of inputs.
pub fn sunken(element: El) -> El {
    element.box_shadow(rune::Shadow { x: 2.0, y: 2.0, blur: 0.0, spread: 0.0, color: Color::BLACK.alpha(0.28), inset: true })
}

/// The `.trigger` box: bg-input, 2 pt border (accent when open or hovered), inset shadow.
pub fn trigger_frame(theme: &Theme, key: String, open: bool) -> El {
    let frame = row()
        .key(key)
        .gap(8.0)
        .center_y()
        .px(12.0)
        .py(9.0)
        .min_h(40.0)
        .fill(theme.bg_input)
        .border(2.0, if open { theme.accent } else { theme.border })
        .role("combobox")
        .expanded(open)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover_group();
    sunken(frame).child(hover_ring(theme.accent))
}

/// `<Select>`: `width` in points; `on_choose` gets the option's value.
#[allow(clippy::too_many_arguments)]
pub fn select(
    app: &App,
    layer: &mut Vec<El>,
    key: &'static str,
    width: f32,
    opts: Vec<Opt>,
    current: &str,
    searchable: bool,
    aria: Option<String>,
    on_choose: impl Fn(&mut App, String) + 'static,
) -> El {
    let theme = app.theme();
    let open = app.browse.open_select == Some(key);
    let anchor = format!("sel-{key}");
    let label = opts.iter().find(|opt| opt.value == current).map(|opt| opt.label.clone());
    let mut trigger = trigger_frame(&theme, anchor.clone(), open)
        .w(width)
        .on_press(move |app: &mut App| {
            if app.browse.open_select == Some(key) {
                app.browse.open_select = None;
            } else {
                app.browse.open_select = Some(key);
                app.browse.select_query.replace_text("");
                if searchable {
                    app.browse.select_query.focused = true;
                }
            }
        })
        .child(
            text(label.clone().unwrap_or_default())
                .font(theme.sans(13.0, 400.0))
                .color(if label.is_some() { theme.text } else { theme.text_muted })
                .grow(1.0)
                .truncate(),
        )
        .child(ui::colored_icon(if open { &icons::CHEVRON_UP } else { &icons::CHEVRON_DOWN }, 12.0, theme.text_muted));
    if let Some(aria_label) = aria {
        trigger = trigger.label(aria_label);
    } else if let Some(current_label) = &label {
        trigger = trigger.label(current_label.clone());
    }
    if !open {
        return trigger;
    }

    let on_choose = Rc::new(on_choose);
    let term = app.browse.select_query.text.trim().to_lowercase();
    let filtered: Vec<&Opt> =
        opts.iter().filter(|opt| !searchable || term.is_empty() || opt.label.to_lowercase().contains(&term)).collect();
    let mut list = col().key("list").padding(4.0).gap(2.0).items_stretch();
    for (i, opt) in filtered.iter().enumerate() {
        let selected = opt.value == current;
        let ink = if selected { theme.accent } else { theme.text };
        let mut item = row()
            .key(format!("opt-{i}"))
            .role("option")
            .label(opt.label.clone())
            .checked(selected)
            .gap(8.0)
            .center_y()
            .px(10.0)
            .py(8.0)
            .border(2.0, Color::TRANSPARENT)
            .child(text(opt.label.clone()).font(theme.sans(13.0, 400.0)).color(ink).grow(1.0).truncate());
        if selected {
            item = item.child(ui::colored_icon(&icons::CHECK, 12.0, theme.accent));
        }
        if opt.disabled {
            item = item.opacity(0.5).disabled(true);
        } else {
            let (choose, value) = (on_choose.clone(), opt.value.clone());
            item = item
                .hover_group()
                .hover(theme.bg_hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .child(hover_ring(theme.accent))
                .on_press(move |app: &mut App| {
                    app.browse.open_select = None;
                    choose(app, value.clone());
                });
        }
        list = list.child(item);
    }
    if filtered.is_empty() {
        list = list.child(text(t_no_matches()).font(theme.sans(12.5, 400.0)).color(theme.text_muted).padding(10.0).self_center());
    }

    let mut menu = col()
        .key(format!("menu-{key}"))
        .role("listbox")
        .w(width)
        .fill(theme.bg_raised)
        .border(2.0, theme.border)
        .shadow(12.0, 30.0, Color::BLACK.alpha(0.5))
        .anchored_fit(anchor, Side::Bottom, Align::Start, 4.0)
        .modal(true, |app: &mut App| app.browse.open_select = None);
    if searchable {
        let query = &app.browse.select_query;
        let field = rune::text_input(query, theme.sans(13.0, 400.0))
            .single_line()
            .placeholder(tr("home.search"), theme.text_muted)
            .color(theme.text)
            .selection_color(theme.accent.alpha(0.6))
            .on_edit(|app: &mut App| &mut app.browse.select_query)
            .grow(1.0);
        menu = menu.child(
            row()
                .key("search")
                .px(10.0)
                .py(9.0)
                .fill(theme.bg_input)
                .border_sides([0.0, 0.0, 2.0, 0.0], if query.focused { theme.accent } else { theme.border })
                .child(field),
        );
    }
    menu = menu.child(col().key("scroll").max_h(240.0).min_h(0.0).shrink().scroll(ScrollSpec { stick_bottom: false }).child(list));
    if searchable {
        layer.push(
            el().key(format!("backdrop-{key}"))
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .fill(Color::BLACK.alpha(0.32))
                .transition(rune::Transition::opacity()),
        );
    }
    layer.push(menu);
    trigger
}

fn t_no_matches() -> String {
    tr("palette.noMatches")
}
