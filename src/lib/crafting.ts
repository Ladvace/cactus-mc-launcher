// Real Minecraft textures are streamed at runtime from the community mcmeta
// mirror (misode/mcmeta) — nothing copyrighted is bundled in this repo. The
// recipe *shapes* below are the game's real recipes, authored here.
const TEX_VERSION = "1.21.4";
const TEX_BASE = `https://raw.githubusercontent.com/misode/mcmeta/${TEX_VERSION}-assets/assets/minecraft/textures/`;

export function texUrl(path: string): string {
  return TEX_BASE + path;
}

export type MaterialId =
  | "oak_planks"
  | "stick"
  | "cobblestone"
  | "coal"
  | "iron_ingot"
  | "gold_ingot"
  | "diamond"
  | "redstone"
  | "string"
  | "wheat"
  | "sand"
  | "gunpowder"
  | "flint"
  | "feather";

export const MATERIALS: { id: MaterialId; name: string; tex: string }[] = [
  { id: "oak_planks", name: "Oak Planks", tex: "block/oak_planks.png" },
  { id: "stick", name: "Stick", tex: "item/stick.png" },
  { id: "cobblestone", name: "Cobblestone", tex: "block/cobblestone.png" },
  { id: "coal", name: "Coal", tex: "item/coal.png" },
  { id: "iron_ingot", name: "Iron Ingot", tex: "item/iron_ingot.png" },
  { id: "gold_ingot", name: "Gold Ingot", tex: "item/gold_ingot.png" },
  { id: "diamond", name: "Diamond", tex: "item/diamond.png" },
  { id: "redstone", name: "Redstone", tex: "item/redstone.png" },
  { id: "string", name: "String", tex: "item/string.png" },
  { id: "wheat", name: "Wheat", tex: "item/wheat.png" },
  { id: "sand", name: "Sand", tex: "block/sand.png" },
  { id: "gunpowder", name: "Gunpowder", tex: "item/gunpowder.png" },
  { id: "flint", name: "Flint", tex: "item/flint.png" },
  { id: "feather", name: "Feather", tex: "item/feather.png" },
];

export const MATERIAL_TEX: Record<MaterialId, string> = Object.fromEntries(
  MATERIALS.map((m) => [m.id, m.tex])
) as Record<MaterialId, string>;

export interface Reward {
  accent?: string;
  message: string;
}

export interface Recipe {
  id: string;
  name: string;
  resultTex: string;
  pattern: (MaterialId | null)[][];
  reward: Reward;
}

const _ = null;

export const RECIPES: Recipe[] = [
  {
    id: "stick", name: "Stick", resultTex: "item/stick.png",
    pattern: [["oak_planks"], ["oak_planks"]],
    reward: { message: "Every journey starts with a stick." },
  },
  {
    id: "crafting_table", name: "Crafting Table", resultTex: "block/crafting_table_front.png",
    pattern: [["oak_planks", "oak_planks"], ["oak_planks", "oak_planks"]],
    reward: { message: "You crafted the very table you're standing at. Meta." },
  },
  {
    id: "torch", name: "Torch", resultTex: "block/torch.png",
    pattern: [["coal"], ["stick"]],
    reward: { accent: "amber", message: "Let there be light — unlocked the Amber accent." },
  },
  {
    id: "redstone_torch", name: "Redstone Torch", resultTex: "block/redstone_torch.png",
    pattern: [["redstone"], ["stick"]],
    reward: { accent: "redstone", message: "It's got the glow — unlocked the Redstone accent." },
  },
  {
    id: "furnace", name: "Furnace", resultTex: "block/furnace_front.png",
    pattern: [
      ["cobblestone", "cobblestone", "cobblestone"],
      ["cobblestone", _, "cobblestone"],
      ["cobblestone", "cobblestone", "cobblestone"],
    ],
    reward: { message: "Time to smelt something." },
  },
  {
    id: "wooden_pickaxe", name: "Wooden Pickaxe", resultTex: "item/wooden_pickaxe.png",
    pattern: [["oak_planks", "oak_planks", "oak_planks"], [_, "stick", _], [_, "stick", _]],
    reward: { message: "Your first tool. Go mine something." },
  },
  {
    id: "wooden_sword", name: "Wooden Sword", resultTex: "item/wooden_sword.png",
    pattern: [["oak_planks"], ["oak_planks"], ["stick"]],
    reward: { message: "En garde!" },
  },
  {
    id: "iron_pickaxe", name: "Iron Pickaxe", resultTex: "item/iron_pickaxe.png",
    pattern: [["iron_ingot", "iron_ingot", "iron_ingot"], [_, "stick", _], [_, "stick", _]],
    reward: { message: "Now we're mining in style." },
  },
  {
    id: "diamond_sword", name: "Diamond Sword", resultTex: "item/diamond_sword.png",
    pattern: [["diamond"], ["diamond"], ["stick"]],
    reward: { accent: "diamond", message: "Sharp — unlocked the Diamond accent." },
  },
  {
    id: "diamond_block", name: "Block of Diamond", resultTex: "block/diamond_block.png",
    pattern: [
      ["diamond", "diamond", "diamond"],
      ["diamond", "diamond", "diamond"],
      ["diamond", "diamond", "diamond"],
    ],
    reward: { accent: "diamond", message: "Flexing — unlocked the Diamond accent." },
  },
  {
    id: "tnt", name: "TNT", resultTex: "block/tnt_side.png",
    pattern: [
      ["gunpowder", "sand", "gunpowder"],
      ["sand", "gunpowder", "sand"],
      ["gunpowder", "sand", "gunpowder"],
    ],
    reward: { message: "…you probably shouldn't have." },
  },
  {
    id: "bow", name: "Bow", resultTex: "item/bow.png",
    pattern: [[_, "stick", "string"], ["stick", _, "string"], [_, "stick", "string"]],
    reward: { message: "Take aim." },
  },
  {
    id: "arrow", name: "Arrow", resultTex: "item/arrow.png",
    pattern: [["flint"], ["stick"], ["feather"]],
    reward: { message: "Ammo secured." },
  },
  {
    id: "bread", name: "Bread", resultTex: "item/bread.png",
    pattern: [["wheat", "wheat", "wheat"]],
    reward: { message: "A snack for the road." },
  },
  {
    id: "clock", name: "Clock", resultTex: "item/clock_00.png",
    pattern: [[_, "gold_ingot", _], ["gold_ingot", "redstone", "gold_ingot"], [_, "gold_ingot", _]],
    reward: { message: "Always on time now." },
  },
  {
    id: "flint_and_steel", name: "Flint and Steel", resultTex: "item/flint_and_steel.png",
    pattern: [["iron_ingot", _], [_, "flint"]],
    reward: { message: "Careful with that." },
  },
];

export const RECIPE_COUNT = RECIPES.length;

function normalize(grid: (MaterialId | null)[]): (MaterialId | null)[][] | null {
  const cells: { r: number; c: number }[] = [];
  for (let i = 0; i < 9; i++) if (grid[i]) cells.push({ r: Math.floor(i / 3), c: i % 3 });
  if (cells.length === 0) return null;
  const minR = Math.min(...cells.map((p) => p.r));
  const maxR = Math.max(...cells.map((p) => p.r));
  const minC = Math.min(...cells.map((p) => p.c));
  const maxC = Math.max(...cells.map((p) => p.c));
  const out: (MaterialId | null)[][] = [];
  for (let r = minR; r <= maxR; r++) {
    const row: (MaterialId | null)[] = [];
    for (let c = minC; c <= maxC; c++) row.push(grid[r * 3 + c]);
    out.push(row);
  }
  return out;
}

function mirror(shape: (MaterialId | null)[][]): (MaterialId | null)[][] {
  return shape.map((row) => [...row].reverse());
}

function sameShape(a: (MaterialId | null)[][], b: (MaterialId | null)[][]): boolean {
  if (a.length !== b.length || a[0].length !== b[0].length) return false;
  for (let r = 0; r < a.length; r++)
    for (let c = 0; c < a[0].length; c++) if (a[r][c] !== b[r][c]) return false;
  return true;
}

/** The recipe the grid matches (position-independent, mirror-tolerant). */
export function matchRecipe(grid: (MaterialId | null)[]): Recipe | null {
  const shape = normalize(grid);
  if (!shape) return null;
  const flipped = mirror(shape);
  return RECIPES.find((r) => sameShape(shape, r.pattern) || sameShape(flipped, r.pattern)) ?? null;
}
