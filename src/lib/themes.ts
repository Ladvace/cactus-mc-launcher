// Appearance presets for Settings → Appearance.
//
// A preset sets the base `background` string (see background.ts) and optionally
// a `decor` theme id. A decor theme places a few sprites (cropped from the
// sprite sheet) at fixed spots around the page — corners and edges — for a
// natural "themed" feel rather than a repeating wallpaper.

const S = (n: number) => `/decor/sprites/${String(n).padStart(2, "0")}.png`;

export interface Placement {
  sprite: string;
  /** CSS edge offsets, e.g. "bottom:6px; left:10px;". */
  at: string;
  size: number;
  rotate?: number;
  opacity?: number;
  flip?: boolean;
}

export interface DecorTheme {
  id: string;
  placements: Placement[];
  /** A small sprite that perches on the corner of the dock. */
  peek: string;
}

export const DECOR_THEMES: DecorTheme[] = [
  {
    id: "desert",
    placements: [
      { sprite: S(0), at: "bottom:6px; left:8px;", size: 118 }, // potted cactus
      { sprite: S(8), at: "bottom:6px; right:12px;", size: 104, flip: true }, // tall cactus
      { sprite: S(1), at: "top:72px; left:14px;", size: 42, opacity: 0.9 }, // flower
      { sprite: S(18), at: "top:78px; right:34px;", size: 34, opacity: 0.85 }, // sparkle
      { sprite: S(6), at: "bottom:8px; left:26%;", size: 60, opacity: 0.8 }, // rocks
    ],
    peek: S(9), // flowering cactus perches on the dock
  },
  {
    id: "bloom",
    placements: [
      { sprite: S(1), at: "bottom:8px; left:10px;", size: 90 }, // pink flower
      { sprite: S(2), at: "bottom:10px; right:14px;", size: 78 }, // yellow flowers
      { sprite: S(16), at: "top:72px; left:16px;", size: 34, rotate: -12 }, // pink butterfly
      { sprite: S(17), at: "top:80px; right:32px;", size: 36, rotate: 10 }, // blue butterfly
      { sprite: S(19), at: "top:44%; right:16px;", size: 28, opacity: 0.8 }, // sparkle
    ],
    peek: S(1), // pink flower
  },
  {
    id: "mystic",
    placements: [
      { sprite: S(22), at: "bottom:8px; left:10px;", size: 92 }, // crystals
      { sprite: S(12), at: "bottom:8px; right:14px;", size: 92, flip: true }, // skull
      { sprite: S(10), at: "top:74px; left:16px;", size: 46, opacity: 0.95 }, // green potion
      { sprite: S(18), at: "top:78px; right:36px;", size: 30, opacity: 0.85 }, // sparkle
      { sprite: S(14), at: "top:46%; left:14px;", size: 26, opacity: 0.8 }, // gem
    ],
    peek: S(4), // crystals
  },
];

export interface ThemePreset {
  name: string;
  bg: string;
  decor?: string;
}

export const THEME_PRESETS: ThemePreset[] = [
  { name: "Default", bg: "" },
  { name: "Midnight", bg: "color:#0f1219" },
  { name: "Dusk", bg: "color:#241a26" },
  { name: "Forest", bg: "color:#152018" },
  { name: "Dots", bg: "pattern:dots|#181620" },
  { name: "Grid", bg: "pattern:grid|#14181e" },
  // Vibrant gradient backgrounds (the `color:` kind takes any CSS value).
  { name: "Sunset", bg: "color:linear-gradient(160deg, #d98a2b 0%, #8a4a2a 45%, #3a2340 100%)" },
  { name: "Orchid", bg: "color:linear-gradient(160deg, #b63a86 0%, #5a2456 55%, #2a1830 100%)" },
  { name: "Aurora", bg: "color:linear-gradient(155deg, #1fb38f 0%, #157a6e 45%, #10233a 100%)" },
  // Tiled stone-block texture at 50% over a warm overlay (opacity is adjustable).
  { name: "Ruins", bg: "texture:0.5|#211c17|/textures/stone-blocks.png" },
  // Decor themes: vibrant solid base + placed sprites.
  { name: "Desert", bg: "color:#c9a866", decor: "desert" },
  { name: "Bloom", bg: "color:#c85a92", decor: "bloom" },
  { name: "Mystic", bg: "color:#1f9c86", decor: "mystic" },
];
