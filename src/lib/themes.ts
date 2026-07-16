// One-click background presets for Settings → Appearance. Each is just a
// stored background string (see background.ts): a solid colour, a coloured
// pattern, or a decor-sprite wallpaper (tile:) cropped from the sprite sheet.

export interface ThemePreset {
  name: string;
  bg: string;
}

export const THEME_PRESETS: ThemePreset[] = [
  { name: "Default", bg: "" },
  { name: "Midnight", bg: "color:#0f1219" },
  { name: "Dusk", bg: "color:#241a26" },
  { name: "Sand", bg: "color:#2b2620" },
  { name: "Forest", bg: "color:#152018" },
  { name: "Dots", bg: "pattern:dots|#181620" },
  { name: "Grid", bg: "pattern:grid|#14181e" },
  { name: "Adobe", bg: "pattern:diagonal|#231a15" },
  { name: "Cacti", bg: "tile:#18241a|/decor/sprites/56.png" },
  { name: "Blooms", bg: "tile:#241924|/decor/sprites/01.png" },
  { name: "Stardust", bg: "tile:#1a1b26|/decor/sprites/18.png" },
  { name: "Crystals", bg: "tile:#12201d|/decor/sprites/22.png" },
  { name: "Bones", bg: "tile:#201f1a|/decor/sprites/12.png" },
];
