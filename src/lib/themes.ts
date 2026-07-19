const spriteUrl = (id: number) => `/decor/sprites/${String(id).padStart(2, "0")}.png`;

interface Placement {
  sprite: string;
  at: string;
  size: number;
  rotate?: number;
  opacity?: number;
  flip?: boolean;
}

interface DecorTheme {
  id: string;
  placements: Placement[];
  peek: string;
}

export const DECOR_THEMES: DecorTheme[] = [
  {
    id: "desert",
    placements: [
      { sprite: spriteUrl(0), at: "bottom:6px; left:8px;", size: 118 }, // potted cactus
      { sprite: spriteUrl(8), at: "bottom:6px; right:12px;", size: 104, flip: true }, // tall cactus
      { sprite: spriteUrl(1), at: "top:72px; left:14px;", size: 42, opacity: 0.9 }, // flower
      { sprite: spriteUrl(18), at: "top:78px; right:34px;", size: 34, opacity: 0.85 }, // sparkle
      { sprite: spriteUrl(6), at: "bottom:8px; left:26%;", size: 60, opacity: 0.8 }, // rocks
    ],
    peek: spriteUrl(9), // flowering cactus perches on the dock
  },
  {
    id: "bloom",
    placements: [
      { sprite: spriteUrl(1), at: "bottom:8px; left:10px;", size: 90 }, // pink flower
      { sprite: spriteUrl(2), at: "bottom:10px; right:14px;", size: 78 }, // yellow flowers
      { sprite: spriteUrl(16), at: "top:72px; left:16px;", size: 34, rotate: -12 }, // pink butterfly
      { sprite: spriteUrl(17), at: "top:80px; right:32px;", size: 36, rotate: 10 }, // blue butterfly
      { sprite: spriteUrl(19), at: "top:44%; right:16px;", size: 28, opacity: 0.8 }, // sparkle
    ],
    peek: spriteUrl(1), // pink flower
  },
  {
    id: "mystic",
    placements: [
      { sprite: spriteUrl(22), at: "bottom:8px; left:10px;", size: 92 }, // crystals
      { sprite: spriteUrl(12), at: "bottom:8px; right:14px;", size: 92, flip: true }, // skull
      { sprite: spriteUrl(10), at: "top:74px; left:16px;", size: 46, opacity: 0.95 }, // green potion
      { sprite: spriteUrl(18), at: "top:78px; right:36px;", size: 30, opacity: 0.85 }, // sparkle
      { sprite: spriteUrl(14), at: "top:46%; left:14px;", size: 26, opacity: 0.8 }, // gem
    ],
    peek: spriteUrl(4), // crystals
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
  { name: "Sunset", bg: "color:linear-gradient(160deg, #d98a2b 0%, #8a4a2a 45%, #3a2340 100%)" },
  { name: "Orchid", bg: "color:linear-gradient(160deg, #b63a86 0%, #5a2456 55%, #2a1830 100%)" },
  { name: "Aurora", bg: "color:linear-gradient(155deg, #1fb38f 0%, #157a6e 45%, #10233a 100%)" },
  { name: "Cactus", bg: "color:linear-gradient(160deg, #d9e746 0%, #7ba32f 42%, #16230f 100%)" },
  { name: "Ruins", bg: "texture:0.5|#211c17|/textures/stone-blocks.png" },
  { name: "Desert", bg: "color:#c9a866", decor: "desert" },
  { name: "Bloom", bg: "color:#c85a92", decor: "bloom" },
  { name: "Mystic", bg: "color:#1f9c86", decor: "mystic" },
];
