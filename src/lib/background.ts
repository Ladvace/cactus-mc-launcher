// The app background is stored as a single tagged string in settings:
//   ""                      → default app colour
//   "color:#rrggbb"         → solid colour
//   "pattern:<name>"        → a built-in CSS pattern (default base colour)
//   "pattern:<name>|#rrggbb"→ a pattern over a chosen base colour
//   "image:<uri>"           → an image / GIF / sticker (cover) with a dark scrim
//   "image:#rrggbb|<uri>"   → an image with a chosen scrim tint

export const PATTERNS = ["dots", "grid", "diagonal", "checker"] as const;

/** Default app background; mirrors `--bg-app` in app.css. */
export const DEFAULT_COLOR = "#17161a";

/** Characters that could break out of a CSS `url("…")` value. */
const CSS_URL_UNSAFE_RE = /["'()\\\s]/g;

/** Characters not allowed in an animated-theme name (→ CSS class suffix). */
const ANIM_NAME_UNSAFE_RE = /[^a-z0-9-]/gi;

/** Build a CSS `url("…")` from a URL, percent-encoding characters that could
 * otherwise break out of the declaration and inject arbitrary CSS. */
export function cssUrl(uri: string): string {
  return `url("${uri.replace(CSS_URL_UNSAFE_RE, encodeURIComponent)}")`;
}

/** A `#rrggbb` colour (leading `#` optional), capturing the six hex digits. */
const SIX_DIGIT_HEX_RE = /^#?([0-9a-fA-F]{6})$/;
/** A leading `0–1` opacity followed by `|` at the start of a `texture:` value. */
const TEXTURE_OPACITY_PREFIX_RE = /^(0(?:\.\d+)?|1(?:\.0+)?)\|/;

export function parsePattern(bg: string): { name: string; color: string | null } {
  const rest = bg.slice("pattern:".length);
  const separatorIndex = rest.indexOf("|");
  if (separatorIndex < 0) return { name: rest, color: null };
  return { name: rest.slice(0, separatorIndex), color: rest.slice(separatorIndex + 1) || null };
}

/** Split `image:[#color|]<uri>` into its parts. Data URIs/URLs never start with `#`. */
export function parseImage(bg: string): { uri: string; color: string | null } {
  return parsePrefixed(bg, "image:");
}

/** Split `tile:[#color|]<uri>` (a repeating decor-sprite wallpaper). */
function parseTile(bg: string): { uri: string; color: string | null } {
  return parsePrefixed(bg, "tile:");
}

/**
 * Split `texture:[<opacity>|][#color|]<uri>` — a repeating texture shown at
 * `opacity` (0–1) over a colour overlay that fills the rest. Opacity defaults
 * to 0.5 when the leading number is absent.
 */
export function parseTexture(bg: string): {
  uri: string;
  color: string | null;
  opacity: number;
} {
  let rest = bg.slice("texture:".length);
  let opacity = 0.5;
  const match = TEXTURE_OPACITY_PREFIX_RE.exec(rest);
  if (match) {
    opacity = parseFloat(match[1]);
    rest = rest.slice(match[0].length);
  }
  let color: string | null = null;
  if (rest.startsWith("#")) {
    const separatorIndex = rest.indexOf("|");
    if (separatorIndex > 0) {
      color = rest.slice(0, separatorIndex);
      rest = rest.slice(separatorIndex + 1);
    }
  }
  return { uri: rest, color, opacity };
}

function parsePrefixed(bg: string, prefix: string): { uri: string; color: string | null } {
  const rest = bg.slice(prefix.length);
  if (rest.startsWith("#")) {
    const separatorIndex = rest.indexOf("|");
    if (separatorIndex > 0)
      return { color: rest.slice(0, separatorIndex), uri: rest.slice(separatorIndex + 1) };
  }
  return { uri: rest, color: null };
}

function hexToRgba(hex: string, alpha: number): string {
  const match = SIX_DIGIT_HEX_RE.exec(hex.trim());
  // Fall back to the default app colour (which is a valid hex, so this recurses
  // at most once) rather than repeating its rgb literal here.
  if (!match) return hexToRgba(DEFAULT_COLOR, alpha);
  const rgb = parseInt(match[1], 16);
  return `rgba(${(rgb >> 16) & 255}, ${(rgb >> 8) & 255}, ${rgb & 255}, ${alpha})`;
}

function patternCss(name: string, color: string | null): string {
  const base = color || "var(--bg-app)";
  switch (name) {
    case "dots":
      return `radial-gradient(var(--border) 1.5px, transparent 1.6px) 0 0 / 18px 18px, ${base}`;
    case "grid":
      return (
        "linear-gradient(var(--border-subtle) 1px, transparent 1px) 0 0 / 22px 22px," +
        "linear-gradient(90deg, var(--border-subtle) 1px, transparent 1px) 0 0 / 22px 22px," +
        base
      );
    case "diagonal":
      return `repeating-linear-gradient(45deg, var(--border-subtle) 0 10px, ${base} 10px 20px)`;
    case "checker":
      return `repeating-conic-gradient(var(--border-subtle) 0% 25%, ${base} 0% 50%) 0 0 / 28px 28px`;
    default:
      return base;
  }
}

export function backgroundCss(bg: string): string {
  if (!bg || bg === "default") return "var(--bg-app)";
  // Animated themes are driven by a CSS class (keyframes live in app.css), so the
  // consumer applies `animClass(bg)` and skips the inline background. This is only
  // a fallback base if a caller doesn't.
  if (bg.startsWith("anim:")) return "var(--bg-app)";
  if (bg.startsWith("color:")) return bg.slice(6) || "var(--bg-app)";
  if (bg.startsWith("pattern:")) {
    const { name, color } = parsePattern(bg);
    return patternCss(name, color);
  }
  if (bg.startsWith("image:")) {
    const { uri, color } = parseImage(bg);
    const scrim = hexToRgba(color ?? DEFAULT_COLOR, 0.55);
    return `linear-gradient(${scrim}, ${scrim}), ${cssUrl(uri)} center / cover no-repeat`;
  }
  if (bg.startsWith("tile:")) {
    const { uri, color } = parseTile(bg);
    const base = color || "var(--bg-app)";
    return `${cssUrl(uri)} 0 0 / 140px repeat, ${base}`;
  }
  if (bg.startsWith("texture:")) {
    const { uri, color, opacity } = parseTexture(bg);
    // Tiled at the image's intrinsic size (no fractional scaling) so the repeats
    // meet without a subpixel seam.
    const overlayAlpha = Math.max(0, Math.min(1, 1 - opacity));
    const scrim = hexToRgba(color ?? DEFAULT_COLOR, overlayAlpha);
    return `linear-gradient(${scrim}, ${scrim}), ${cssUrl(uri)} repeat`;
  }
  return "var(--bg-app)";
}

type BgKind = "default" | "color" | "pattern" | "image" | "tile" | "texture" | "anim";

export function bgKind(bg: string): BgKind {
  if (bg.startsWith("color:")) return "color";
  if (bg.startsWith("pattern:")) return "pattern";
  if (bg.startsWith("image:")) return "image";
  if (bg.startsWith("tile:")) return "tile";
  if (bg.startsWith("texture:")) return "texture";
  if (bg.startsWith("anim:")) return "anim";
  return "default";
}

/** CSS class for an animated theme (`anim:<name>`) — the element showing the
 * background gets this class so app.css can drive its keyframes. "" for static
 * backgrounds. Callers apply it and skip the inline `background`. */
export function animClass(bg: string): string {
  if (!bg.startsWith("anim:")) return "";
  const name = bg.slice("anim:".length).replace(ANIM_NAME_UNSAFE_RE, "");
  return name ? `anim-${name}` : "";
}
