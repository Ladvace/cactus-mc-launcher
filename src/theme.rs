//! The design tokens of `src/app.css` (`:root`, `[data-accent]`, `[data-high-contrast]`,
//! `[data-readable-font]`) and the presets of `src/lib/themes.ts`, plus a rune-kit `Kit` in the
//! same colours for the kit components (inputs, selects, menus).

use rune::tok::{Family, OpticalSize, TextStyle};
use rune::{Color, color};

use crate::settings::Settings;

pub const PIXEL_FAMILY: &str = "Pixelify Sans";
pub const SANS_FAMILY: &str = "Inter";

/// Parses `#rrggbb` (or `#rgb`); black if it isn't a colour.
pub fn hex_color(hex: &str) -> Color {
    color::hex(hex).unwrap_or(Color::BLACK)
}

/// The gradient stops of the animated `anim:nebula` / `anim:ember` / `anim:tide`
/// backgrounds; the first stop repeats at the end so the drift loops seamlessly.
pub const NEBULA_STOPS: [&str; 5] = ["#140b1e", "#3a1d5c", "#7b2f6b", "#2a1830", "#140b1e"];
pub const EMBER_STOPS: [&str; 5] = ["#1a0e08", "#4a1e0a", "#a8451f", "#3a1808", "#1a0e08"];
pub const TIDE_STOPS: [&str; 5] = ["#0a1420", "#10395a", "#1fb38f", "#10233a", "#0a1420"];

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    hex_color(&format!("#{r:02x}{g:02x}{b:02x}")).alpha(a)
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg_app: Color,
    pub bg_raised: Color,
    pub bg_card: Color,
    pub bg_hover: Color,
    pub bg_input: Color,
    pub border: Color,
    pub border_subtle: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_contrast: Color,
    pub accent_soft: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
    /// The near-black outline every `.btn` draws round itself (`--edge`).
    pub edge: Color,
    pub shadow: Color,
    /// `--font-pixel`: Pixelify Sans, or Inter with "readable font" on.
    pub pixel: Family,
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
}

pub struct AccentPreset {
    pub id: &'static str,
    pub label_key: &'static str,
    pub swatch: &'static str,
}

pub const ACCENT_PRESETS: [AccentPreset; 6] = [
    AccentPreset { id: "", label_key: "settings.accentGold", swatch: "#e8b23a" },
    AccentPreset { id: "emerald", label_key: "settings.accentEmerald", swatch: "#3fbf5f" },
    AccentPreset { id: "diamond", label_key: "settings.accentDiamond", swatch: "#4fd6d6" },
    AccentPreset { id: "redstone", label_key: "settings.accentRedstone", swatch: "#e5484d" },
    AccentPreset { id: "lapis", label_key: "settings.accentLapis", swatch: "#4f86e0" },
    AccentPreset { id: "amethyst", label_key: "settings.accentAmethyst", swatch: "#9b6dff" },
];

/// `[data-accent=…]`: accent, hover, contrast ink, soft fill.
fn accent(id: &str) -> (Color, Color, Color, Color) {
    match id {
        "emerald" => (hex_color("#3fbf5f"), hex_color("#58d878"), hex_color("#05230f"), rgba(63, 191, 95, 0.16)),
        "diamond" => (hex_color("#4fd6d6"), hex_color("#6fe9e9"), hex_color("#032321"), rgba(79, 214, 214, 0.16)),
        "redstone" => (hex_color("#e5484d"), hex_color("#f26a6f"), hex_color("#2a0708"), rgba(229, 72, 77, 0.16)),
        "lapis" => (hex_color("#4f86e0"), hex_color("#6d9ff0"), hex_color("#04122b"), rgba(79, 134, 224, 0.16)),
        "amethyst" => (hex_color("#9b6dff"), hex_color("#b389ff"), hex_color("#170a2e"), rgba(155, 109, 255, 0.16)),
        "amber" => (hex_color("#ff8c3b"), hex_color("#ffa155"), hex_color("#2a1405"), rgba(255, 140, 59, 0.16)),
        _ => (hex_color("#e8b23a"), hex_color("#f4c451"), hex_color("#211906"), rgba(232, 178, 58, 0.14)),
    }
}

/// `LOADER_COLORS` in themes.ts.
pub fn loader_color(loader: crate::instance::ModLoader) -> Color {
    use crate::instance::ModLoader::*;
    hex_color(match loader {
        Vanilla => "#5b8a3a",
        Fabric => "#c8a86a",
        Quilt => "#9b59d0",
        Forge => "#4a6b8a",
        NeoForge => "#d98a3a",
    })
}

impl Theme {
    pub fn from_settings(s: &Settings) -> Theme {
        let (accent, accent_hover, accent_contrast, accent_soft) = accent(&s.accent);
        let mut theme = Theme {
            bg_app: hex_color("#17161a"),
            bg_raised: hex_color("#201e1c"),
            bg_card: hex_color("#2a2825"),
            bg_hover: hex_color("#35322d"),
            bg_input: hex_color("#131210"),
            border: hex_color("#3d3a34"),
            border_subtle: hex_color("#2b2924"),
            text: hex_color("#ebe8e0"),
            text_secondary: hex_color("#b2aca0"),
            text_muted: hex_color("#7f7970"),
            accent,
            accent_hover,
            accent_contrast,
            accent_soft,
            danger: hex_color("#ff5b6e"),
            warning: hex_color("#ffb454"),
            success: hex_color("#57c84a"),
            edge: hex_color("#100e0b"),
            shadow: Color::BLACK.alpha(0.45),
            pixel: if s.readable_font { Family::Named(SANS_FAMILY) } else { Family::Named(PIXEL_FAMILY) },
            reduce_motion: s.reduce_motion,
            reduce_transparency: s.reduce_transparency,
        };
        if s.high_contrast {
            theme.bg_app = hex_color("#000000");
            theme.bg_raised = hex_color("#0c0c0c");
            theme.bg_card = hex_color("#151515");
            theme.bg_hover = hex_color("#2a2a2a");
            theme.bg_input = hex_color("#050505");
            theme.border = hex_color("#6f6f6f");
            theme.border_subtle = hex_color("#4c4c4c");
            theme.text = hex_color("#ffffff");
            theme.text_secondary = hex_color("#ececec");
            theme.text_muted = hex_color("#c4c4c4");
            theme.accent_soft = accent.alpha(0.3);
        }
        theme
    }

    /// Headings, labels and anything `font-family: var(--font-pixel)`.
    pub fn pixel(&self, size: f32) -> TextStyle {
        TextStyle { size, weight: 600.0, family: self.pixel, line_height: 0.0, tracking: size * 0.01, ..sans(size, 400.0) }
    }

    pub fn sans(&self, size: f32, weight: f32) -> TextStyle {
        sans(size, weight)
    }

    /// 14 / 1.5, the body copy of `:root`.
    pub fn body(&self) -> TextStyle {
        TextStyle { line_height: 21.0, ..sans(14.0, 400.0) }
    }

    pub fn mono(&self, size: f32) -> TextStyle {
        TextStyle { family: Family::SystemMono, ..sans(size, 400.0) }
    }

    /// `.field-label`: pixel, 12, uppercase is applied by the caller's text.
    pub fn field_label(&self) -> TextStyle {
        TextStyle { tracking: 12.0 * 0.04, ..self.pixel(12.0) }
    }

    pub fn kit(&self) -> rune_kit::Kit {
        let mut k = rune_kit::Kit::from_palette(rune_kit::PaletteSeed { bg: self.bg_raised, fg: self.text, accent: self.accent });
        k.radius = rune_kit::Radius { xs: 0.0, sm: 0.0, md: 0.0, lg: 0.0, xl: 0.0 };
        k.c.border_focus = self.accent;
        k
    }
}

pub const fn sans(size: f32, weight: f32) -> TextStyle {
    TextStyle {
        size,
        weight,
        family: Family::Named(SANS_FAMILY),
        line_height: 0.0,
        tracking: 0.0,
        variations: "",
        features: "",
        optical_size: OpticalSize::Auto,
        italic: false,
    }
}

pub struct ThemePreset {
    pub name: &'static str,
    pub bg: &'static str,
    pub decor: Option<&'static str>,
}

/// `THEME_PRESETS` in themes.ts (background strings as `Settings::background` stores them).
pub const THEME_PRESETS: [ThemePreset; 21] = [
    ThemePreset { name: "Default", bg: "", decor: None },
    ThemePreset { name: "Midnight", bg: "color:#0f1219", decor: None },
    ThemePreset { name: "Dusk", bg: "color:#241a26", decor: None },
    ThemePreset { name: "Forest", bg: "color:#152018", decor: None },
    ThemePreset { name: "Dots", bg: "pattern:dots|#181620", decor: None },
    ThemePreset { name: "Grid", bg: "pattern:grid|#14181e", decor: None },
    ThemePreset { name: "Sunset", bg: "color:linear-gradient(160deg, #d98a2b 0%, #8a4a2a 45%, #3a2340 100%)", decor: None },
    ThemePreset { name: "Orchid", bg: "color:linear-gradient(160deg, #b63a86 0%, #5a2456 55%, #2a1830 100%)", decor: None },
    ThemePreset { name: "Aurora", bg: "color:linear-gradient(155deg, #1fb38f 0%, #157a6e 45%, #10233a 100%)", decor: None },
    ThemePreset { name: "Cactus", bg: "color:linear-gradient(160deg, #d9e746 0%, #7ba32f 42%, #16230f 100%)", decor: None },
    ThemePreset { name: "Ruins", bg: "texture:0.5|#211c17|/textures/stone-blocks.png", decor: None },
    ThemePreset { name: "Desert", bg: "color:#c9a866", decor: Some("desert") },
    ThemePreset { name: "Bloom", bg: "color:#c85a92", decor: Some("bloom") },
    ThemePreset { name: "Mystic", bg: "color:#1f9c86", decor: Some("mystic") },
    ThemePreset { name: "Nebula", bg: "anim:nebula", decor: None },
    ThemePreset { name: "Ember", bg: "anim:ember", decor: None },
    ThemePreset { name: "Tide", bg: "anim:tide", decor: None },
    ThemePreset { name: "Starfield", bg: "anim:starfield", decor: None },
    ThemePreset { name: "Beams", bg: "anim:beams", decor: None },
    ThemePreset { name: "Circuit", bg: "anim:circuit", decor: None },
    ThemePreset { name: "Lattice", bg: "anim:lattice", decor: None },
];

/// Each decor theme's sprite that peeks over the dock (`peek` in `DECOR_THEMES`).
pub const DECOR_PEEKS: [(&str, &str); 3] =
    [("desert", "/decor/sprites/09.png"), ("bloom", "/decor/sprites/01.png"), ("mystic", "/decor/sprites/04.png")];

/// `hex` parsing for strings coming from settings and themes.ts; None for anything else.
pub fn parse_color(s: &str) -> Option<Color> {
    color::hex(s.trim())
}
