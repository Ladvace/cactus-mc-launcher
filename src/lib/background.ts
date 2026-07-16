// The app background is stored as a single tagged string in settings:
//   ""              → default app colour
//   "color:#rrggbb" → solid colour
//   "pattern:<name>"→ a built-in CSS pattern
//   "image:<uri>"   → an image / GIF / sticker (cover), with a scrim for contrast

export const PATTERNS = ["dots", "grid", "diagonal", "checker"] as const;
export type PatternName = (typeof PATTERNS)[number];

export const DEFAULT_COLOR = "#17161a";

function patternCss(name: string): string {
  switch (name) {
    case "dots":
      return "radial-gradient(var(--border) 1.5px, transparent 1.6px) 0 0 / 18px 18px, var(--bg-app)";
    case "grid":
      return (
        "linear-gradient(var(--border-subtle) 1px, transparent 1px) 0 0 / 22px 22px," +
        "linear-gradient(90deg, var(--border-subtle) 1px, transparent 1px) 0 0 / 22px 22px," +
        "var(--bg-app)"
      );
    case "diagonal":
      return "repeating-linear-gradient(45deg, var(--bg-raised) 0 10px, var(--bg-app) 10px 20px)";
    case "checker":
      return "repeating-conic-gradient(var(--bg-raised) 0% 25%, var(--bg-app) 0% 50%) 0 0 / 28px 28px";
    default:
      return "var(--bg-app)";
  }
}

/** CSS `background` shorthand value for a stored background string. */
export function backgroundCss(bg: string): string {
  if (!bg || bg === "default") return "var(--bg-app)";
  if (bg.startsWith("color:")) return bg.slice(6) || "var(--bg-app)";
  if (bg.startsWith("pattern:")) return patternCss(bg.slice(8));
  if (bg.startsWith("image:")) {
    const url = bg.slice(6);
    // Dark scrim over the image so foreground text stays readable.
    return `linear-gradient(rgba(23,22,26,0.55), rgba(23,22,26,0.55)), url("${url}") center / cover no-repeat`;
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
