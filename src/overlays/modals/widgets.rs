//! The pieces these overlays share: `ContextMenu.svelte`, `Select.svelte`, and the inset,
//! accent-on-hover boxes (`.loader-chip`, `.opt`, `.emoji`, …) the Svelte files style by hand.

use std::rc::Rc;

use rune::{Align, Color, Cursor, El, Icon, Side, TextEdit, Transition, Tx, col, el, row, text};

use crate::app::App;
use crate::core::i18n::t as tr;
use crate::icons;
use crate::theme::Theme;
use crate::ui;

pub type Access = fn(&mut App) -> &mut TextEdit;

/// A 2 pt bordered box whose border turns accent on hover (and stays accent when `on`):
/// `content` is padded 2 pt more to make room for the border, drawn as two layers above it.
pub fn boxed(theme: &Theme, content: El, on: bool) -> El {
    let edge = |color: Color| el().absolute(0.0, 0.0).w_full().h_full().border(2.0, color);
    let mut framed = content.hover_group().child(edge(if on { theme.accent } else { theme.border }));
    if !on {
        framed = framed.child(edge(theme.accent).group_opacity(0.0, 1.0));
    }
    framed
}

/// `.loader-chip` / `.kind-btn` / `.opt`: an input-coloured box, accent when `on`.
pub fn chip(theme: &Theme, key: impl Into<String>, on: bool, content: El) -> El {
    let face = content
        .key(key.into())
        .fill(if on { theme.accent_soft } else { theme.bg_input })
        .box_shadow(ui::inset_shadow(2.0, 2.0, Color::BLACK.alpha(0.22)))
        .role("button")
        .focusable(true)
        .cursor(Cursor::Pointer);
    boxed(theme, face, on)
}

/// Text in a chip: secondary, `text` while hovered, accent when on.
pub fn chip_text(theme: &Theme, label: impl Into<String>, size: f32, weight: f32, on: bool) -> El {
    let styled = text(label.into()).font(theme.sans(size, weight)).color(if on { theme.accent } else { theme.text_secondary });
    if on { styled } else { styled.group_color(theme.text) }
}

pub fn chip_icon(theme: &Theme, icon: &'static Icon, size: f32, on: bool) -> El {
    let glyph = ui::colored_icon(icon, size, if on { theme.accent } else { theme.text_secondary });
    if on { glyph } else { glyph.group_color(theme.text) }
}

/// `.input` bound to `edit`, with Enter running `submit`.
pub fn field(theme: &Theme, edit: &TextEdit, placeholder: impl Into<String>, access: Access, submit: Option<fn(&mut App)>) -> El {
    let mut input = rune::text_input(edit, theme.sans(13.0, 400.0))
        .single_line()
        .placeholder(placeholder.into(), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(access)
        .grow(1.0);
    if let Some(f) = submit {
        input = input.on_submit(move |app: &mut App, _: &mut Tx| f(app));
    }
    ui::input_frame(theme, edit.focused).child(input)
}

// ContextMenu.svelte

pub struct MenuItem {
    pub label: String,
    pub icon: Option<&'static Icon>,
    pub danger: bool,
    pub disabled: bool,
    pub on: Rc<dyn Fn(&mut App, &mut Tx)>,
}

pub enum Entry {
    Item(MenuItem),
    Sep,
}

impl Entry {
    pub fn item(label: impl Into<String>, icon: &'static Icon, on: impl Fn(&mut App) + 'static) -> Entry {
        Entry::Item(MenuItem { label: label.into(), icon: Some(icon), danger: false, disabled: false, on: Rc::new(move |app, _| on(app)) })
    }
    pub fn item_tx(label: impl Into<String>, icon: &'static Icon, on: impl Fn(&mut App, &mut Tx) + 'static) -> Entry {
        Entry::Item(MenuItem { label: label.into(), icon: Some(icon), danger: false, disabled: false, on: Rc::new(on) })
    }
    pub fn disabled(mut self, disabled: bool) -> Entry {
        if let Entry::Item(item) = &mut self {
            item.disabled = disabled;
        }
        self
    }
    pub fn danger(mut self) -> Entry {
        if let Entry::Item(item) = &mut self {
            item.danger = true;
        }
        self
    }
}

/// The menu at window point (`x`, `y`), kept inside the window. Choosing an item closes it
/// first, then runs it; a press outside or Escape closes it.
pub fn context_menu(theme: &Theme, key: &str, x: f32, y: f32, entries: Vec<Entry>, close: fn(&mut App), layer: &mut Vec<El>) {
    let point = format!("{key}-point");
    layer.push(el().key(point.clone()).absolute(x, y).size(0.0, 0.0));
    let mut list = col().key("items").items_stretch();
    for (i, entry) in entries.into_iter().enumerate() {
        list = list.child(match entry {
            Entry::Sep => el().key(format!("sep{i}")).h(2.0).mt(5.0).mb(5.0).mx(4.0).fill(theme.border_subtle).a11y_hidden(),
            Entry::Item(item) => menu_item(theme, format!("item{i}"), item, close),
        });
    }
    let surface = col()
        .key(key.to_string())
        .role("menu")
        .focus_group()
        .min_w(176.0)
        .padding(5.0)
        .fill(theme.bg_raised)
        .border(2.0, theme.border)
        .shadow(6.0, 24.0, theme.shadow)
        .transition(Transition::opacity().combined(Transition::offset(0.0, -4.0)).combined(Transition::scale(0.98)))
        .modal(true, close)
        .child(list);
    layer.push(ui::bevel(surface, 2.0, 0.05, 0.3).anchored(point, Side::Bottom, Align::Start, 0.0));
}

fn menu_item(theme: &Theme, key: String, item: MenuItem, close: fn(&mut App)) -> El {
    let (ink, hover_ink, hover_fill) = if item.danger {
        (theme.danger, theme.danger, theme.danger.alpha(0.12))
    } else {
        (theme.text_secondary, theme.text, theme.bg_hover)
    };
    let mut face = row()
        .key(key)
        .role("menuitem")
        .label(item.label.clone())
        .gap(9.0)
        .center_y()
        .px(10.0)
        .py(8.0)
        .focusable(!item.disabled);
    if let Some(icon) = item.icon {
        let glyph = ui::colored_icon(icon, 14.0, if item.danger { theme.danger } else { theme.text_muted });
        face = face.child(if item.disabled { glyph } else { glyph.group_color(if item.danger { theme.danger } else { theme.accent }) });
    }
    let label = text(item.label).font(theme.sans(13.0, 500.0)).color(ink);
    face = face.child(if item.disabled { label } else { label.group_color(hover_ink) });
    if item.disabled {
        return face.opacity(0.4).disabled(true);
    }
    let on = item.on.clone();
    face.hover_group().hover(hover_fill).cursor(Cursor::Pointer).on_press_tx(move |app: &mut App, tx| {
        close(app);
        on(app, tx);
    })
}

// Select.svelte

pub struct Opt {
    pub value: String,
    pub label: String,
    pub muted: bool,
}

impl Opt {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Opt {
        Opt { value: value.into(), label: label.into(), muted: false }
    }
}

/// A `Select`: a trigger showing `value`'s label and, while `open`, its list below it (`width`
/// wide). With `search`, the list filters on a search field, followed by `header`.
pub struct Select<'a> {
    pub key: &'a str,
    pub options: &'a [Opt],
    pub value: &'a str,
    pub open: bool,
    pub width: f32,
    pub toggle: fn(&mut App),
    pub close: fn(&mut App),
    pub choose: Rc<dyn Fn(&mut App, &str)>,
    pub search: Option<(&'a TextEdit, Access)>,
    pub header: Option<El>,
}

impl Select<'_> {
    pub fn build(self, theme: &Theme, layer: &mut Vec<El>) -> El {
        let label = self.options.iter().find(|opt| opt.value == self.value).map(|opt| opt.label.clone());
        let placeholder = label.is_none();
        let anchor = format!("{}-trigger", self.key);
        let face = row()
            .key(anchor.clone())
            .role("combobox")
            .expanded(self.open)
            .focusable(true)
            .cursor(Cursor::Pointer)
            .w(self.width)
            .gap(8.0)
            .center_y()
            .px(14.0)
            .py(11.0)
            .fill(theme.bg_input)
            .box_shadow(ui::inset_shadow(2.0, 2.0, Color::BLACK.alpha(0.28)))
            .on_press(self.toggle)
            .child(
                text(label.unwrap_or_default())
                    .font(theme.sans(13.0, 400.0))
                    .color(if placeholder { theme.text_muted } else { theme.text })
                    .truncate()
                    .grow(1.0),
            )
            .child(ui::colored_icon(if self.open { &icons::CHEVRON_UP } else { &icons::CHEVRON_DOWN }, 12.0, theme.text_muted));
        let trigger = boxed(theme, face, self.open);
        if !self.open {
            return trigger;
        }
        let query = self.search.map(|(edit, _)| edit.text.trim().to_lowercase()).unwrap_or_default();
        let mut list = col().key("list").padding(4.0).gap(2.0).items_stretch();
        let mut any = false;
        for (i, opt) in self.options.iter().enumerate() {
            if !query.is_empty() && !opt.label.to_lowercase().contains(&query) {
                continue;
            }
            any = true;
            list = list.child(option(theme, i, opt, opt.value == self.value, self.choose.clone(), self.close));
        }
        if !any {
            list = list.child(
                el().key("empty").padding(10.0).center().child(text(t_no_matches()).font(theme.sans(12.5, 400.0)).color(theme.text_muted)),
            );
        }
        let mut menu = col()
            .key(format!("{}-menu", self.key))
            .w(self.width)
            .fill(theme.bg_raised)
            .border(2.0, theme.border)
            .shadow(12.0, 30.0, Color::BLACK.alpha(0.5))
            .modal(true, self.close);
        if let Some((edit, access)) = self.search {
            let input = rune::text_input(edit, theme.sans(13.0, 400.0))
                .single_line()
                .placeholder(t_search(), theme.text_muted)
                .color(theme.text)
                .selection_color(theme.accent.alpha(0.6))
                .on_edit(access)
                .autofocus()
                .grow(1.0)
                .px(10.0)
                .py(9.0);
            let mut head = row()
                .key("head")
                .center_y()
                .gap(6.0)
                .fill(theme.bg_input)
                .border_sides([0.0, 0.0, 2.0, 0.0], if edit.focused { theme.accent } else { theme.border })
                .child(input);
            if let Some(header) = self.header {
                head = head.child(el().pr(8.0).child(header));
            }
            menu = menu.child(head);
        }
        menu = menu.child(list.max_h(240.0).shrink().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }));
        layer.push(menu.anchored_fit(anchor, Side::Bottom, Align::Start, 4.0));
        trigger
    }
}

fn option(theme: &Theme, i: usize, opt: &Opt, selected: bool, choose: Rc<dyn Fn(&mut App, &str)>, close: fn(&mut App)) -> El {
    let value = opt.value.clone();
    let mut face = row()
        .key(format!("opt{i}"))
        .role("option")
        .focusable(true)
        .cursor(Cursor::Pointer)
        .gap(8.0)
        .center_y()
        .pl(if opt.muted { 26.0 } else { 12.0 })
        .pr(12.0)
        .py(10.0)
        .hover(theme.bg_hover)
        .on_press(move |app: &mut App| {
            close(app);
            choose(app, &value);
        })
        .child(text(opt.label.clone()).font(theme.sans(13.0, 400.0)).color(if selected { theme.accent } else { theme.text }).truncate().grow(1.0));
    if selected {
        face = face.child(ui::colored_icon(&icons::CHECK, 12.0, theme.accent));
    }
    if opt.muted {
        face = face.group_opacity(0.55, 1.0);
    }
    // `.opt` draws a transparent 2 pt border that turns accent on hover.
    let edge = el().absolute(0.0, 0.0).w_full().h_full().border(2.0, theme.accent).group_opacity(0.0, 1.0);
    face.hover_group().child(edge)
}

fn t_search() -> String {
    tr("home.search")
}

fn t_no_matches() -> String {
    tr("palette.noMatches")
}

/// The compact `.dd-snap` switch (`Snapshots [■ ]`).
pub fn mini_switch(theme: &Theme, on: bool) -> El {
    el().size(28.0, 16.0)
        .fill(if on { theme.accent_soft } else { theme.bg_card })
        .border(2.0, if on { theme.accent } else { theme.border })
        // 1 pt gap inside the 2 pt border, which Rune draws inside the box.
        .child(el().size(10.0, 10.0).absolute(if on { 15.0 } else { 3.0 }, 3.0).fill(if on { theme.accent } else { theme.text_muted }))
}

/// `before <strong>mid</strong> after` as one wrapping paragraph.
pub fn strong(theme: &Theme, before: &str, mid: &str, after: &str, size: f32, color: Color, strong_color: Color) -> El {
    let full = format!("{before}{mid}{after}");
    let (start, end) = (before.len(), before.len() + mid.len());
    let spans = if mid.is_empty() { Vec::new() } else { vec![rune::Span::new(start, end).weight(700.0).color(strong_color)] };
    rune::rich(full, spans).font(theme.sans(size, 400.0)).color(color)
}
