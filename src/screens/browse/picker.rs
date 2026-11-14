//! `InstancePicker.svelte`: a trigger showing the chosen instance and a searchable, grouped
//! popover of every instance. The popover is as wide as the trigger.

use std::collections::BTreeMap;
use std::rc::Rc;

use rune::{Align, Color, Cursor, Cx, El, ScrollSpec, Side, col, row, text};

use crate::app::App;
use crate::core::i18n::t as tr;
use crate::icons;
use crate::instance::Instance;
use crate::ui;

use super::select::trigger_frame;

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub query: rune::TextEdit,
}

const KEY: &str = "instance-picker";

fn matches(inst: &Instance, term: &str) -> bool {
    term.is_empty()
        || inst.name.to_lowercase().contains(term)
        || inst.mc_version.to_lowercase().contains(term)
        || inst.loader.id().contains(term)
}

/// `value` is the chosen id; `on_choose` gets the new one.
pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>, value: &str, on_choose: impl Fn(&mut App, String) + 'static) -> El {
    let theme = app.theme();
    let picker = &app.browse.picker;
    let selected = app.instance(value);
    let mut trigger = trigger_frame(&theme, KEY.to_string(), picker.open)
        .w_full()
        .label(tr("picker.selectInstance"))
        .on_press(|app: &mut App| {
            let picker = &mut app.browse.picker;
            picker.open = !picker.open;
            if picker.open {
                picker.query.replace_text("");
                picker.query.focused = true;
            }
        });
    trigger = match selected {
        Some(inst) => trigger
            .child(ui::instance_icon(app, inst, 22.0))
            .child(text(inst.name.clone()).font(theme.sans(13.0, 600.0)).color(theme.text).shrink().truncate())
            .child(text(format!("{} {}", inst.loader.id(), inst.mc_version)).font(theme.sans(12.0, 400.0)).color(theme.text_muted)),
        None => trigger.child(text(tr("picker.selectInstance")).font(theme.sans(13.0, 400.0)).color(theme.text_muted)),
    };
    trigger = trigger.child(rune::el().grow(1.0)).child(caret(theme.text_muted));
    if !picker.open {
        return trigger;
    }

    let width = cx.layout_size(KEY).map_or(560.0, |[w, _]| w);
    let term = picker.query.text.trim().to_lowercase();
    let mut ungrouped = Vec::new();
    let mut groups: BTreeMap<&str, Vec<&Instance>> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for inst in app.instances.iter().filter(|candidate| matches(candidate, &term)) {
        match inst.group.as_deref() {
            Some(group) => {
                if !groups.contains_key(group) {
                    order.push(group);
                }
                groups.entry(group).or_default().push(inst);
            }
            None => ungrouped.push(inst),
        }
    }
    let on_choose = Rc::new(on_choose);
    let option_row = |inst: &Instance| {
        let selected = inst.id == value;
        let (choose, id) = (on_choose.clone(), inst.id.clone());
        row()
            .key(format!("opt-{}", inst.id))
            .role("option")
            .label(inst.name.clone())
            .checked(selected)
            .gap(8.0)
            .center_y()
            .px(8.0)
            .py(7.0)
            .hover(theme.bg_hover)
            .hover_group()
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| {
                let picker = &mut app.browse.picker;
                picker.open = false;
                picker.query.replace_text("");
                choose(app, id.clone());
            })
            .child(ui::instance_icon(app, inst, 22.0))
            .child(
                text(inst.name.clone())
                    .font(theme.sans(13.0, 600.0))
                    .color(if selected { theme.accent } else { theme.text_secondary })
                    .group_color(if selected { theme.accent } else { theme.text })
                    .shrink()
                    .truncate(),
            )
            .child(rune::el().grow(1.0))
            .child(text(format!("{} {}", inst.loader.id(), inst.mc_version)).font(theme.sans(11.5, 400.0)).color(theme.text_muted))
    };
    let mut list = col().key("list").padding(4.0).items_stretch();
    if ungrouped.is_empty() && groups.is_empty() {
        list = list.child(text(tr("picker.noMatch")).font(theme.sans(12.5, 400.0)).color(theme.text_muted).padding(14.0).self_center());
    }
    for inst in ungrouped {
        list = list.child(option_row(inst));
    }
    for group in order {
        list = list.child(
            row()
                .key(format!("group-{group}"))
                .gap(5.0)
                .center_y()
                .px(8.0)
                .pt(8.0)
                .pb(4.0)
                .child(ui::colored_icon(&icons::FOLDER, 11.0, theme.text_muted))
                .child(text(group.to_uppercase()).font(rune::tok::TextStyle { tracking: 0.5, ..theme.pixel(10.0) }).color(theme.text_muted)),
        );
        for inst in &groups[group] {
            list = list.child(option_row(inst));
        }
    }
    let field = rune::text_input(&picker.query, theme.sans(13.0, 400.0))
        .single_line()
        .placeholder(tr("picker.searchInstances"), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(|app: &mut App| &mut app.browse.picker.query)
        .grow(1.0);
    let popover = col()
        .key("picker-pop")
        .w(width)
        .fill(theme.bg_raised)
        .border(2.0, theme.border)
        .shadow(8.0, 24.0, theme.shadow)
        .anchored_fit(KEY, Side::Bottom, Align::Start, 4.0)
        .modal(true, |app: &mut App| app.browse.picker.open = false)
        .child(
            row()
                .key("search")
                .gap(8.0)
                .center_y()
                .px(10.0)
                .py(8.0)
                .border_sides([0.0, 0.0, 2.0, 0.0], theme.border_subtle)
                .child(ui::colored_icon(&icons::SEARCH, 14.0, theme.text_muted))
                .child(field),
        )
        .child(col().key("scroll").max_h(240.0).min_h(0.0).shrink().scroll(ScrollSpec { stick_bottom: false }).child(list));
    layer.push(popover);
    trigger
}

/// `.caret`: a 8×5 downward triangle, drawn as pixel rows.
fn caret(color: Color) -> El {
    let mut caret = col().items_center();
    for (i, w) in [8.0, 6.0, 4.0, 2.0].into_iter().enumerate() {
        caret = caret.child(rune::el().key(format!("c{i}")).size(w, if i == 0 { 2.0 } else { 1.0 }).fill(color));
    }
    caret
}
