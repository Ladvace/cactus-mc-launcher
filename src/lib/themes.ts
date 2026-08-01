import type { MessageKey } from "./i18n";
import type { ModLoader } from "./types";

const spriteUrl = (id: number) => `/decor/sprites/${String(id).padStart(2, "0")}.png`;

export interface AccentPreset {
  id: string;
  labelKey: MessageKey;
  swatch: string;
}

export const ACCENT_PRESETS: AccentPreset[] = [
  { id: "", labelKey: "settings.accentGold", swatch: "#e8b23a" },
  { id: "emerald", labelKey: "settings.accentEmerald", swatch: "#3fbf5f" },
  { id: "diamond", labelKey: "settings.accentDiamond", swatch: "#4fd6d6" },
  { id: "redstone", labelKey: "settings.accentRedstone", swatch: "#e5484d" },
  { id: "lapis", labelKey: "settings.accentLapis", swatch: "#4f86e0" },
  { id: "amethyst", labelKey: "settings.accentAmethyst", swatch: "#9b6dff" },
];

export const LOADER_COLORS: Record<ModLoader, string> = {
  vanilla: "#5b8a3a",
  fabric: "#c8a86a",
  quilt: "#9b59d0",
  forge: "#4a6b8a",
  neoforge: "#d98a3a",
};

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
      { sprite: spriteUrl(0), at: "bottom:6px; left:8px;", size: 118 },
      { sprite: spriteUrl(8), at: "bottom:6px; right:12px;", size: 104, flip: true },
      { sprite: spriteUrl(1), at: "top:72px; left:14px;", size: 42, opacity: 0.9 },
      { sprite: spriteUrl(18), at: "top:78px; right:34px;", size: 34, opacity: 0.85 },
      { sprite: spriteUrl(6), at: "bottom:8px; left:26%;", size: 60, opacity: 0.8 },
    ],
    peek: spriteUrl(9),
  },
  {
    id: "bloom",
    placements: [
      { sprite: spriteUrl(1), at: "bottom:8px; left:10px;", size: 90 },
      { sprite: spriteUrl(2), at: "bottom:10px; right:14px;", size: 78 },
      { sprite: spriteUrl(16), at: "top:72px; left:16px;", size: 34, rotate: -12 },
      { sprite: spriteUrl(17), at: "top:80px; right:32px;", size: 36, rotate: 10 },
      { sprite: spriteUrl(19), at: "top:44%; right:16px;", size: 28, opacity: 0.8 },
    ],
    peek: spriteUrl(1),
  },
  {
    id: "mystic",
    placements: [
      { sprite: spriteUrl(22), at: "bottom:8px; left:10px;", size: 92 },
      { sprite: spriteUrl(12), at: "bottom:8px; right:14px;", size: 92, flip: true },
      { sprite: spriteUrl(10), at: "top:74px; left:16px;", size: 46, opacity: 0.95 },
      { sprite: spriteUrl(18), at: "top:78px; right:36px;", size: 30, opacity: 0.85 },
      { sprite: spriteUrl(14), at: "top:46%; left:14px;", size: 26, opacity: 0.8 },
    ],
    peek: spriteUrl(4),
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
  { name: "Nebula", bg: "anim:nebula" },
  { name: "Ember", bg: "anim:ember" },
  { name: "Tide", bg: "anim:tide" },
  { name: "Starfield", bg: "anim:starfield" },
  { name: "Beams", bg: "anim:beams" },
  { name: "Circuit", bg: "anim:circuit" },
  { name: "Lattice", bg: "anim:lattice" },
];
