// The app background is stored as a single tagged string in settings:
//   ""                      → default app colour
//   "color:#rrggbb"         → solid colour
//   "pattern:<name>"        → a built-in CSS pattern (default base colour)
//   "pattern:<name>|#rrggbb"→ a pattern over a chosen base colour
//   "image:<uri>"           → an image / GIF / sticker (cover) with a dark scrim
//   "image:#rrggbb|<uri>"   → an image with a chosen scrim tint

export const PATTERNS = ["dots", "grid", "diagonal", "checker"] as const;

export const DEFAULT_COLOR = "#17161a";

/** Split `pattern:<name>[|#color]` into its parts. */
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
  const match = /^(0(?:\.\d+)?|1(?:\.0+)?)\|/.exec(rest);
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
  const match = /^#?([0-9a-fA-F]{6})$/.exec(hex.trim());
  if (!match) return `rgba(23, 22, 26, ${alpha})`;
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

/** CSS `background` shorthand value for a stored background string. */
export function backgroundCss(bg: string): string {
  if (!bg || bg === "default") return "var(--bg-app)";
  if (bg.startsWith("color:")) return bg.slice(6) || "var(--bg-app)";
  if (bg.startsWith("pattern:")) {
    const { name, color } = parsePattern(bg);
    return patternCss(name, color);
  }
  if (bg.startsWith("image:")) {
    const { uri, color } = parseImage(bg);
    // Scrim over the image so foreground text stays readable.
    const scrim = hexToRgba(color ?? DEFAULT_COLOR, 0.55);
    return `linear-gradient(${scrim}, ${scrim}), url("${uri}") center / cover no-repeat`;
  }
  if (bg.startsWith("tile:")) {
    const { uri, color } = parseTile(bg);
    const base = color || "var(--bg-app)";
    // A decor sprite repeated as sparse wallpaper over a base colour.
    return `url("${uri}") 0 0 / 140px repeat, ${base}`;
  }
  if (bg.startsWith("texture:")) {
    const { uri, color, opacity } = parseTexture(bg);
    // A full texture tiled as wallpaper. A colour overlay fills whatever the
    // texture's opacity leaves, so it reads as a subtle surface rather than a
    // busy image. Tiled at the image's intrinsic size (no fractional scaling)
    // so the repeats meet without a subpixel seam.
    const overlayAlpha = Math.max(0, Math.min(1, 1 - opacity));
    const scrim = hexToRgba(color ?? DEFAULT_COLOR, overlayAlpha);
    return `linear-gradient(${scrim}, ${scrim}), url("${uri}") repeat`;
  }
  return "var(--bg-app)";
}

type BgKind = "default" | "color" | "pattern" | "image" | "tile" | "texture";

export function bgKind(bg: string): BgKind {
  if (bg.startsWith("color:")) return "color";
  if (bg.startsWith("pattern:")) return "pattern";
  if (bg.startsWith("image:")) return "image";
  if (bg.startsWith("tile:")) return "tile";
  if (bg.startsWith("texture:")) return "texture";
  return "default";
}
