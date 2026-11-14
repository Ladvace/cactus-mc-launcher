//! `GroupPicker.svelte`: move an instance into a group, or out of one.

use rune::{Cursor, El, TextEdit, col, row, text};

use super::widgets::{self, boxed};
use crate::instance::UpdateInstance;
use crate::app::App;
use crate::core::i18n::t as tr;
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    new_group: TextEdit,
}

fn close(app: &mut App) {
    app.ui.group_for = None;
}

fn assign(app: &mut App, group: String) {
    let Some(id) = app.ui.group_for.take() else { return };
    super::update(app, &id, UpdateInstance { group: Some(group), ..Default::default() });
}

fn create_and_assign(app: &mut App) {
    let name = app.modals.group.new_group.text.trim().to_string();
    if name.is_empty() {
        return;
    }
    app.modals.group.new_group = TextEdit::default();
    assign(app, name);
}

/// Every group in use, sorted.
pub fn groups(app: &App) -> Vec<String> {
    let mut names: Vec<String> = app.instances.iter().filter_map(|i| i.group.clone()).filter(|group| !group.is_empty()).collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names
}

pub fn view(app: &App, layer: &mut Vec<El>) {
    let Some(inst) = app.ui.group_for.as_deref().and_then(|id| app.instance(id)) else { return };
    let theme = app.theme();
    let current = inst.group.clone().unwrap_or_default();
    let mut options = col()
        .key("options")
        .gap(4.0)
        .items_stretch()
        .child(option(&theme, "none", &icons::HOME, tr("group.noGroup"), current.is_empty(), String::new()));
    for group in groups(app) {
        let on = group == current;
        options = options.child(option(&theme, &format!("g-{group}"), &icons::FOLDER, group.clone(), on, group));
    }
    let picker = &app.modals.group;
    let body = col()
        .key("group-picker")
        .w(340.0)
        .child(
            widgets::strong(&theme, &format!("{} ", tr("group.movePrefix")), &inst.name, &format!(" {}", tr("group.moveSuffix")), 13.0, theme.text_secondary, theme.text)
                .mb(14.0),
        )
        .child(options.max_h(220.0).shrink().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).mb(14.0))
        .child(
            row()
                .key("new")
                .gap(8.0)
                .child(
                    widgets::field(&theme, &picker.new_group, tr("group.newGroupPlaceholder"), |app| &mut app.modals.group.new_group, Some(create_and_assign))
                        .grow(1.0),
                )
                .child(Btn::new(tr("common.create")).primary().key("create-group").disabled(picker.new_group.text.trim().is_empty()).on_press(create_and_assign).build(&theme)),
        );
    layer.push(ui::modal(&theme, "group-picker-modal", tr("group.moveTitle"), 380.0, body, None, close));
}

fn option(theme: &Theme, key: &str, icon: &'static rune::Icon, label: String, on: bool, group: String) -> El {
    let face = row()
        .key(key.to_string())
        .role("button")
        .label(label.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .gap(9.0)
        .center_y()
        .px(13.0)
        .py(11.0)
        .fill(theme.bg_input)
        .on_press(move |app: &mut App| assign(app, group.clone()))
        .child(ui::colored_icon(icon, 14.0, if on { theme.accent } else { theme.text_muted }))
        .child({
            let caption = text(label).font(theme.sans(13.0, 400.0)).color(if on { theme.accent } else { theme.text_secondary });
            if on { caption } else { caption.group_color(theme.text) }
        });
    boxed(theme, face, on)
}
