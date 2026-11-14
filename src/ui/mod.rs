//! The shared pieces of `app.css` and the small components every screen uses: `.btn` and its
//! variants, `.panel`, `.field-label`, `.input`, checkboxes, the range slider, `Modal.svelte`,
//! `ProgressBar.svelte`, `InstanceIcon.svelte`, `.skeleton`, plus `format.ts` / `time.ts`.
//!
//! Every builder ends in a plain `El`; attach handlers with `.on_press(|app: &mut App| ..)`.
//! For text fields, selects, menus and tooltips use rune-kit with `app.theme().kit()`.

pub mod fmt;
mod loader_icons;

use std::rc::Rc;

use rune::tok::TextStyle;
use rune::{Color, Cursor, Drag, El, Icon, Phase, Shadow, Spring, Tx, col, el, icon, row, text};

use crate::app::App;
use crate::core::i18n::t;
use crate::icons;
use crate::instance::Instance;
use crate::theme::{Theme, loader_color};

/// The pixel step of bevels and notched outlines (`--p`).
pub const PIXEL_STEP: f32 = 3.0;

/// A hard-edged inner shadow offset by `x, y`: the building block of the pixel bevels.
pub fn inset_shadow(x: f32, y: f32, color: Color) -> Shadow {
    Shadow { x, y, blur: 0.0, spread: 0.0, color, inset: true }
}

/// The `.panel` / `.pixel-panel` bevel: light top-left, dark bottom-right.
pub fn bevel(element: El, depth: f32, light: f32, dark: f32) -> El {
    element.box_shadow(inset_shadow(depth, depth, Color::WHITE.alpha(light))).box_shadow(inset_shadow(-depth, -depth, Color::BLACK.alpha(dark)))
}

/// A notched pixel outline (`clip-path: var(--clip-pixel)` plus the four `drop-shadow`s of
/// `.btn`): `face` inside a `p`-wide `edge` whose corners are cut out.
pub fn pixel_frame(face: El, edge: Color, step: f32) -> El {
    let bar = |key: &str| el().key(key.to_string()).fill(edge);
    col()
        .child(row().child(el().size(step, step)).child(bar("t").h(step).grow(1.0)).child(el().size(step, step)))
        .child(row().child(bar("l").w(step).self_stretch()).child(face).child(bar("r").w(step).self_stretch()))
        .child(row().child(el().size(step, step)).child(bar("b").h(step).grow(1.0)).child(el().size(step, step)))
}

pub fn colored_icon(glyph: &'static Icon, size: f32, color: Color) -> El {
    icon(glyph, size).color(color)
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    #[default]
    Normal,
    Primary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Big,
}

/// `.btn`. `Btn::new("Play").primary().icon(&icons::PLAY).on_press(|app| ..).build(&theme)`.
#[derive(Default)]
pub struct Btn {
    key: Option<String>,
    label: Option<String>,
    icon: Option<&'static Icon>,
    variant: Variant,
    size: Size,
    disabled: bool,
    full: bool,
    help: Option<String>,
    handler: Option<Rc<dyn Fn(&mut App, &mut Tx)>>,
}

impl Btn {
    pub fn new(label: impl Into<String>) -> Btn {
        Btn { label: Some(label.into()), ..Btn::default() }
    }
    /// An icon-only button; give it a `help` so it has a name.
    pub fn icon_only(glyph: &'static Icon) -> Btn {
        Btn { icon: Some(glyph), ..Btn::default() }
    }
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
    pub fn icon(mut self, glyph: &'static Icon) -> Self {
        self.icon = Some(glyph);
        self
    }
    pub fn primary(mut self) -> Self {
        self.variant = Variant::Primary;
        self
    }
    pub fn ghost(mut self) -> Self {
        self.variant = Variant::Ghost;
        self
    }
    pub fn danger(mut self) -> Self {
        self.variant = Variant::Danger;
        self
    }
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }
    pub fn sm(mut self) -> Self {
        self.size = Size::Sm;
        self
    }
    pub fn big(mut self) -> Self {
        self.size = Size::Big;
        self
    }
    pub fn full(mut self) -> Self {
        self.full = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
    pub fn on_press(mut self, f: impl Fn(&mut App) + 'static) -> Self {
        self.handler = Some(Rc::new(move |app, _| f(app)));
        self
    }
    pub fn on_press_tx(mut self, f: impl Fn(&mut App, &mut Tx) + 'static) -> Self {
        self.handler = Some(Rc::new(f));
        self
    }

    pub fn build(self, theme: &Theme) -> El {
        let (face, ink, light, dark) = match self.variant {
            Variant::Normal => (theme.bg_card, theme.text, 0.10, 0.34),
            Variant::Primary => (theme.accent, theme.accent_contrast, 0.45, 0.22),
            Variant::Ghost => (theme.bg_raised, theme.text, 0.10, 0.34),
            Variant::Danger => (theme.bg_raised, theme.danger, 0.10, 0.34),
        };
        let (px, py, size) = match self.size {
            Size::Sm => (12.0, 6.0, 12.0),
            Size::Md => (16.0, 9.0, 13.0),
            Size::Big => (24.0, 11.0, 14.0),
        };
        let only_icon = self.label.is_none();
        let mut inner = row().center().gap(8.0).py(py).px(if only_icon { py } else { px }).fill(face);
        inner = bevel(inner, PIXEL_STEP, light, dark);
        if let Some(glyph) = self.icon {
            inner = inner.child(colored_icon(glyph, size + 3.0, ink));
        }
        if let Some(label) = &self.label {
            inner = inner.child(text(label.clone()).font(theme.sans(size, 600.0)).color(ink));
        }
        if !self.disabled {
            let hover = if self.variant == Variant::Primary { theme.accent_hover } else { Color::WHITE.alpha(0.1) };
            inner = if self.variant == Variant::Primary { inner.hover(hover) } else { inner.hover(hover) };
        }
        if self.full {
            inner = inner.grow(1.0);
        }
        let mut button = pixel_frame(inner, theme.edge, PIXEL_STEP).role("button").focusable(!self.disabled);
        if let Some(key) = self.key {
            button = button.key(key);
        }
        if let Some(label) = &self.label {
            button = button.label(label.clone());
        }
        if let (None, Some(help)) = (&self.label, &self.help) {
            button = button.label(help.clone());
        }
        if let Some(help) = self.help {
            button = button.help(help);
        }
        if self.full {
            button = button.self_stretch();
        }
        if self.disabled {
            return button.opacity(0.5).disabled(true);
        }
        button = button.press_scale(0.94).cursor(Cursor::Pointer);
        match self.handler {
            Some(handler) => button.on_press_tx(move |app: &mut App, tx| handler(app, tx)),
            None => button,
        }
    }
}

/// `.panel`: card fill, 2 pt border, inner bevel.
pub fn panel(theme: &Theme) -> El {
    bevel(col().fill(theme.bg_card).border(2.0, theme.border), 2.0, 0.05, 0.3)
}

/// `.field-label`.
pub fn field_label(theme: &Theme, content: impl AsRef<str>) -> El {
    text(content.as_ref().to_uppercase()).font(theme.field_label()).color(theme.text_secondary).mb(6.0)
}

pub fn heading(theme: &Theme, content: impl Into<String>, size: f32) -> El {
    text(content.into()).font(theme.pixel(size)).color(theme.text)
}

/// The 14 pt pixel `<h3>` of section cards.
pub fn h3(theme: &Theme, content: impl Into<String>) -> El {
    heading(theme, content, 14.0)
}

pub fn muted(theme: &Theme, content: impl Into<String>, size: f32) -> El {
    text(content.into()).font(theme.sans(size, 400.0)).color(theme.text_muted)
}

pub fn secondary(theme: &Theme, content: impl Into<String>, size: f32) -> El {
    text(content.into()).font(theme.sans(size, 400.0)).color(theme.text_secondary)
}

pub fn body(theme: &Theme, content: impl Into<String>) -> El {
    text(content.into()).font(theme.body()).color(theme.text)
}

/// The `.input` look round any content (a rune `text_input`, a value).
pub fn input_frame(theme: &Theme, focused: bool) -> El {
    row()
        .center_y()
        .px(12.0)
        .min_h(40.0)
        .fill(theme.bg_input)
        .border(2.0, if focused { theme.accent } else { theme.border })
        .box_shadow(inset_shadow(2.0, 2.0, Color::BLACK.alpha(0.28)))
}

/// A single-line `.input` bound to `edit`: `input(&theme, &app.home.search, "Search…", |app| &mut app.home.search)`.
pub fn input(
    theme: &Theme,
    edit: &rune::TextEdit,
    placeholder: impl Into<String>,
    access: impl for<'a> Fn(&'a mut App) -> &'a mut rune::TextEdit + 'static,
) -> El {
    let field = rune::text_input(edit, theme.sans(13.0, 400.0))
        .single_line()
        .placeholder(placeholder.into(), theme.text_muted)
        .color(theme.text)
        .selection_color(theme.accent.alpha(0.6))
        .on_edit(access)
        .grow(1.0);
    input_frame(theme, edit.focused).child(field)
}

/// `input[type=checkbox]` (visual only; put the handler on the row that holds it).
pub fn checkbox(theme: &Theme, checked: bool) -> El {
    let mut check = el()
        .size(18.0, 18.0)
        .center()
        .fill(if checked { theme.accent } else { theme.bg_input })
        .border(2.0, if checked { theme.accent } else { theme.border })
        .role("checkbox")
        .checked(checked);
    if checked {
        check = check.child(colored_icon(&icons::CHECK, 13.0, theme.accent_contrast));
    }
    check
}

/// A checkbox and its label as one pressable row.
pub fn check_row(theme: &Theme, key: &str, label: impl Into<String>, checked: bool, on_press: impl Fn(&mut App) + 'static) -> El {
    let label = label.into();
    row()
        .key(key.to_string())
        .gap(10.0)
        .center_y()
        .role("checkbox")
        .checked(checked)
        .label(label.clone())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_press(move |app: &mut App| on_press(app))
        .child(checkbox(theme, checked).a11y_hidden())
        .child(text(label).font(theme.sans(13.0, 400.0)).color(theme.text).a11y_hidden())
}

/// The settings page's `.switch` toggle.
pub fn switch(theme: &Theme, on: bool) -> El {
    let (w, h, knob) = (36.0, 20.0, 12.0);
    let pad = (h - knob) / 2.0;
    el().size(w, h)
        .fill(if on { theme.accent } else { theme.bg_input })
        .border(2.0, if on { theme.accent } else { theme.border })
        .motion(Spring::new(0.18, 0.0))
        .child(
            el().size(knob, knob)
                // The border is drawn inside the box, so `pad` from the outer edge centres the knob.
                .absolute(if on { w - knob - pad } else { pad }, pad)
                .fill(if on { theme.accent_contrast } else { theme.text_muted }),
        )
}

/// `input[type=range]`: an accent fill on a sunken track, a square thumb. `value` 0..=1;
/// `steps > 0` snaps and draws tick marks. `on_change` runs while dragging.
pub fn slider(theme: &Theme, key: &str, width: f32, value: f32, steps: u32, on_change: impl Fn(&mut App, f32) + 'static) -> El {
    let clamped = value.clamp(0.0, 1.0);
    let thumb = 14.0;
    let travel = width - thumb;
    let snap = move |x: f32| {
        let fraction = ((x - thumb / 2.0) / travel).clamp(0.0, 1.0);
        if steps > 0 { (fraction * steps as f32).round() / steps as f32 } else { fraction }
    };
    let on_change = Rc::new(on_change);
    let mut track = el()
        .key("track")
        .size(width, 6.0)
        .absolute(0.0, 6.0)
        .fill(theme.border)
        .box_shadow(inset_shadow(1.0, 1.0, Color::BLACK.alpha(0.28)))
        .child(el().size(thumb / 2.0 + clamped * travel, 6.0).fill(theme.accent));
    if steps > 0 {
        for i in 1..steps {
            track = track.child(el().size(2.0, 6.0).absolute(i as f32 * width / steps as f32, 0.0).fill(Color::BLACK.alpha(0.42)));
        }
    }
    el().key(key.to_string())
        .size(width, 18.0)
        .role("slider")
        .range_value(f64::from(clamped), 0.0, 1.0)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_drag({
            let on_change = on_change.clone();
            move |app: &mut App, drag: &mut Drag, tx| {
                if matches!(drag.phase, Phase::Down | Phase::Move) {
                    on_change(app, snap(drag.x));
                    tx.instant();
                }
            }
        })
        .on_step(move |app: &mut App, n, _| {
            let step = if steps > 0 { 1.0 / steps as f32 } else { 0.05 };
            on_change(app, (clamped + n as f32 * step).clamp(0.0, 1.0));
        })
        .child(track)
        .child(
            el().key("thumb")
                .size(thumb, thumb)
                .absolute(clamped * travel, 2.0)
                .fill(theme.accent)
                .border(2.0, theme.accent_contrast),
        )
}

/// `ProgressBar.svelte`: a label, a percentage and a striped, stepped bar. `None` is indeterminate.
pub fn progress(theme: &Theme, label: impl Into<String>, pct: Option<u32>) -> El {
    let mut head = row().justify_between().mb(6.0).child(secondary(theme, label, 12.5));
    if let Some(percent) = pct {
        head = head.child(secondary(theme, format!("{percent}%"), 12.5));
    }
    col().w_full().child(head).child(bar(theme, pct, 14.0))
}

/// Just the bar of [`progress`].
pub fn bar(theme: &Theme, pct: Option<u32>, h: f32) -> El {
    let fill = match pct {
        Some(percent) => el().h_full().w_frac(percent.min(100) as f32 / 100.0).fill(theme.accent).motion(Spring::new(0.2, 0.0)),
        None => el()
            .h_full()
            .w_frac(0.35)
            .fill(theme.accent)
            .looping(rune_motion::LoopSpec::opacity(1.1, 0.0, 0.45, 1.0)),
    };
    let mut stripes = row().h_full().w_full().absolute(0.0, 0.0).gap(6.0);
    for _ in 0..80 {
        stripes = stripes.child(el().w(2.0).h_full().fill(Color::BLACK.alpha(0.18)));
    }
    row()
        .w_full()
        .h(h)
        .fill(theme.bg_input)
        .border(2.0, theme.border)
        .clip_corners([0.0; 4])
        .child(fill.child(stripes).clip_corners([0.0; 4]))
}

/// The app's round loaders (`.spinner`, `.dock-spinner`, `.dl-spinner`): a faint ring with its
/// top quarter in the accent, turning every 0.7 s. Rune's loop rotation turns icon quads only,
/// so the ring and the arc are icons.
pub fn spinner(theme: &Theme, size: f32) -> El {
    spinner_with(size, Color::WHITE.alpha(0.2), theme.accent)
}

pub fn spinner_with(size: f32, track: Color, arc: Color) -> El {
    el().size(size, size)
        .a11y_hidden()
        .child(icon(&SPIN_TRACK, size).color(track).absolute(0.0, 0.0))
        .child(icon(&SPIN_ARC, size).color(arc).absolute(0.0, 0.0).looping(rune_motion::LoopSpec::rotation(0.7)))
}

/// A 24-unit ring about 3 units thick (CSS: a 2 px border on a 15 px circle).
static SPIN_TRACK: Icon = Icon {
    name: "spin-track",
    view: 24.0,
    body: r#"<path fill-rule="evenodd" d="M12 0A12 12 0 1 1 12 24A12 12 0 1 1 12 0ZM12 2.9A9.1 9.1 0 1 0 12 21.1A9.1 9.1 0 1 0 12 2.9Z"/>"#,
};

/// The ring's top quarter, from -45° to 45° (what `border-top-color` paints on a circle).
static SPIN_ARC: Icon = Icon {
    name: "spin-arc",
    view: 24.0,
    body: r#"<path d="M3.515 3.515A12 12 0 0 1 20.485 3.515L18.435 5.565A9.1 9.1 0 0 0 5.565 5.565Z"/>"#,
};

/// `.skeleton`.
pub fn skeleton(theme: &Theme, w: Option<f32>, h: f32) -> El {
    let placeholder = el().h(h).fill(theme.bg_card).border(2.0, theme.border_subtle).looping(rune_motion::LoopSpec::opacity(1.2, 0.0, 0.55, 1.0));
    match w {
        Some(w) => placeholder.w(w),
        None => placeholder.w_full(),
    }
}

/// `Modal.svelte`: a dimmed backdrop and a centred panel with a title bar, a scrolling body
/// and an optional footer. Push the result onto the overlay layer.
pub fn modal(theme: &Theme, key: &str, title: impl Into<String>, width: f32, body: El, footer: Option<El>, close: impl Fn(&mut App) + Clone + 'static) -> El {
    let title = title.into();
    let close_btn = el()
        .key("close")
        .px(8.0)
        .py(4.0)
        .role("button")
        .label(t_close())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .hover(theme.bg_hover)
        .on_press({
            let close = close.clone();
            move |app: &mut App| close(app)
        })
        .child(text("✕").font(theme.sans(15.0, 400.0)).color(theme.text_muted));
    let head = row()
        .key("head")
        .justify_between()
        .center_y()
        .px(18.0)
        .py(16.0)
        .border_sides([0.0, 0.0, 1.0, 0.0], theme.border_subtle)
        .child(heading(theme, title.clone(), 16.0))
        .child(close_btn);
    let mut dialog = col()
        .key("dialog")
        .role("dialog")
        .label(title)
        .w(width)
        .max_h(720.0)
        .fill(theme.bg_raised)
        .border(2.0, theme.border)
        .shadow(6.0, 24.0, theme.shadow)
        .transition(rune::Transition::scale(0.98).combined(rune::Transition::offset(0.0, 8.0)))
        .on_press(|_: &mut App| {})
        .child(head)
        .child(col().key("body").padding(18.0).max_h(560.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(body));
    dialog = bevel(dialog, 2.0, 0.05, 0.3);
    if let Some(footer) = footer {
        dialog = dialog.child(
            row().key("foot").justify_end().gap(10.0).px(18.0).py(14.0).border_sides([1.0, 0.0, 0.0, 0.0], theme.border_subtle).child(footer),
        );
    }
    el().key(key.to_string())
        .w_full()
        .h_full()
        .absolute(0.0, 0.0)
        .center()
        .padding(24.0)
        .fill(Color::BLACK.alpha(0.55))
        .transition(rune::Transition::opacity())
        .modal(true, close)
        .child(dialog)
}

fn t_close() -> String {
    t("common.close")
}

/// Footer helper: buttons in a row, right-aligned by `modal`.
pub fn buttons(items: impl IntoIterator<Item = El>) -> El {
    row().gap(10.0).center_y().children(items)
}

/// `InstanceIcon.svelte`: the instance's image, or its initials on its loader's colour.
pub fn instance_icon(app: &App, inst: &Instance, size: f32) -> El {
    let theme = app.theme();
    if let Some(src) = app.instance_icon_src(inst) {
        if let Some(img) = app.images.get(&app.fx, &src) {
            return el().size(size, size).fill(theme.bg_card).border(2.0, Color::BLACK.alpha(0.35)).pixels(img.pixels());
        }
        if !app.images.failed(&src) {
            return el().size(size, size).fill(theme.bg_card);
        }
    }
    let initials: String = inst
        .name
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect();
    let initials = if initials.is_empty() { "?".into() } else { initials };
    bevel(
        el().size(size, size).center().fill(loader_color(inst.loader)).border(2.0, Color::BLACK.alpha(0.3)),
        2.0,
        0.18,
        0.25,
    )
    .child(text(initials).font(TextStyle { weight: 700.0, ..theme.pixel((size * 0.38).round()) }).color(Color::WHITE.alpha(0.92)))
}

/// Any `<img src>`: drawn at `w`×`h` once loaded, an empty box of the same size until then.
pub fn image(app: &App, src: &str, w: f32, h: f32) -> El {
    let frame = el().size(w, h);
    match app.images.get(&app.fx, src) {
        Some(img) => frame.pixels(img.pixels()),
        None => frame,
    }
}

/// `LoaderIcon.svelte`: the loader's SVG mark from src/lib/loaders.
/// A mod loader's logo in `color`, the colour of the text beside it (`currentColor`).
pub fn loader_icon(loader: crate::instance::ModLoader, size: f32, color: Color) -> El {
    use crate::instance::ModLoader;
    let glyph = match loader {
        ModLoader::Vanilla => &loader_icons::VANILLA,
        ModLoader::Fabric => &loader_icons::FABRIC,
        ModLoader::Quilt => &loader_icons::QUILT,
        ModLoader::Forge => &loader_icons::FORGE,
        ModLoader::NeoForge => &loader_icons::NEOFORGE,
    };
    colored_icon(glyph, size, color)
}

/// `font-size` of a pixel heading with the shadow `text-shadow: 0 2px 0 rgba(0,0,0,.4)` look.
pub fn shadowed(theme: &Theme, content: impl Into<String>, size: f32) -> El {
    text(content.into()).font(theme.pixel(size)).color(theme.text)
}

/// A centred empty state: icon, title, a line of text.
pub fn empty_state(theme: &Theme, glyph: &'static Icon, title: impl Into<String>, line: impl Into<String>) -> El {
    col()
        .center()
        .gap(8.0)
        .padding(40.0)
        .child(colored_icon(glyph, 36.0, theme.text_muted))
        .child(heading(theme, title, 16.0))
        .child(muted(theme, line, 13.0))
}
