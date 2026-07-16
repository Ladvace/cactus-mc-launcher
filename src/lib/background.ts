// The app background is stored as a single tagged string in settings:
//   ""                      → default app colour
//   "color:#rrggbb"         → solid colour
//   "pattern:<name>"        → a built-in CSS pattern (default base colour)
//   "pattern:<name>|#rrggbb"→ a pattern over a chosen base colour
//   "image:<uri>"           → an image / GIF / sticker (cover) with a dark scrim
//   "image:#rrggbb|<uri>"   → an image with a chosen scrim tint

export const PATTERNS = ["dots", "grid", "diagonal", "checker"] as const;
export type PatternName = (typeof PATTERNS)[number];

export const DEFAULT_COLOR = "#17161a";

/** Split `pattern:<name>[|#color]` into its parts. */
export function parsePattern(bg: string): { name: string; color: string | null } {
  const rest = bg.slice("pattern:".length);
  const i = rest.indexOf("|");
  if (i < 0) return { name: rest, color: null };
  return { name: rest.slice(0, i), color: rest.slice(i + 1) || null };
}

/** Split `image:[#color|]<uri>` into its parts. Data URIs/URLs never start with `#`. */
export function parseImage(bg: string): { uri: string; color: string | null } {
  const rest = bg.slice("image:".length);
  if (rest.startsWith("#")) {
    const i = rest.indexOf("|");
    if (i > 0) return { color: rest.slice(0, i), uri: rest.slice(i + 1) };
  }
  return { uri: rest, color: null };
}

function hexToRgba(hex: string, a: number): string {
  const m = /^#?([0-9a-fA-F]{6})$/.exec(hex.trim());
  if (!m) return `rgba(23, 22, 26, ${a})`;
  const n = parseInt(m[1], 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
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
    const scrim = color ? hexToRgba(color, 0.55) : "rgba(23, 22, 26, 0.55)";
    return `linear-gradient(${scrim}, ${scrim}), url("${uri}") center / cover no-repeat`;
  }
  return "var(--bg-app)";
}

export type BgKind = "default" | "color" | "pattern" | "image";

export function bgKind(bg: string): BgKind {
  if (bg.startsWith("color:")) return "color";
  if (bg.startsWith("pattern:")) return "pattern";
  if (bg.startsWith("image:")) return "image";
  return "default";
}
