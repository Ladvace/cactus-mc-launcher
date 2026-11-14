//! `CraftingTable.svelte`, `src/lib/crafting.ts` and `src/lib/stores/crafting.svelte.ts`: the
//! hidden vanilla crafting GUI. Textures stream from the misode/mcmeta mirror, as in the app.

use std::rc::Rc;

use rune::{Color, Cursor, Cx, Drag, El, LineCap, Phase, Shape, Transition, Tx, canvas, col, el, row, text};

use crate::app::App;
use crate::core::i18n::{self, tf};
use crate::theme::hex_color;
use crate::ui;

const TEX_BASE: &str = "https://raw.githubusercontent.com/misode/mcmeta/1.21.4-assets/assets/minecraft/textures/";
const KEY: &str = "cactus:crafted";

fn texture_url(path: &str) -> String {
    format!("{TEX_BASE}{path}")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mat {
    OakPlanks,
    Stick,
    Cobblestone,
    Coal,
    IronIngot,
    GoldIngot,
    Diamond,
    Redstone,
    Str,
    Wheat,
    Sand,
    Gunpowder,
    Flint,
    Feather,
}

use Mat::*;

/// `MATERIALS`: id, name, texture.
const MATERIALS: [(Mat, &str, &str); 14] = [
    (OakPlanks, "Oak Planks", "block/oak_planks.png"),
    (Stick, "Stick", "item/stick.png"),
    (Cobblestone, "Cobblestone", "block/cobblestone.png"),
    (Coal, "Coal", "item/coal.png"),
    (IronIngot, "Iron Ingot", "item/iron_ingot.png"),
    (GoldIngot, "Gold Ingot", "item/gold_ingot.png"),
    (Diamond, "Diamond", "item/diamond.png"),
    (Redstone, "Redstone", "item/redstone.png"),
    (Str, "String", "item/string.png"),
    (Wheat, "Wheat", "item/wheat.png"),
    (Sand, "Sand", "block/sand.png"),
    (Gunpowder, "Gunpowder", "item/gunpowder.png"),
    (Flint, "Flint", "item/flint.png"),
    (Feather, "Feather", "item/feather.png"),
];

fn mat_info(mat: Mat) -> (&'static str, &'static str) {
    MATERIALS.iter().find(|(id, ..)| *id == mat).map(|(_, name, path)| (*name, *path)).unwrap_or(("", ""))
}

struct Recipe {
    id: &'static str,
    name: &'static str,
    tex: &'static str,
    pattern: &'static [&'static [Option<Mat>]],
    accent: Option<&'static str>,
    message: &'static str,
}

const EMPTY: Option<Mat> = None;
const fn filled(mat: Mat) -> Option<Mat> {
    Some(mat)
}

/// `RECIPES`: the game's real shapes.
const RECIPES: &[Recipe] = &[
    Recipe { id: "stick", name: "Stick", tex: "item/stick.png", pattern: &[&[filled(OakPlanks)], &[filled(OakPlanks)]], accent: None, message: "Every journey starts with a stick." },
    Recipe { id: "crafting_table", name: "Crafting Table", tex: "block/crafting_table_front.png", pattern: &[&[filled(OakPlanks), filled(OakPlanks)], &[filled(OakPlanks), filled(OakPlanks)]], accent: None, message: "You crafted the very table you're standing at. Meta." },
    Recipe { id: "torch", name: "Torch", tex: "block/torch.png", pattern: &[&[filled(Coal)], &[filled(Stick)]], accent: Some("amber"), message: "Let there be light — unlocked the Amber accent." },
    Recipe { id: "redstone_torch", name: "Redstone Torch", tex: "block/redstone_torch.png", pattern: &[&[filled(Redstone)], &[filled(Stick)]], accent: Some("redstone"), message: "It's got the glow — unlocked the Redstone accent." },
    Recipe {
        id: "furnace", name: "Furnace", tex: "block/furnace_front.png",
        pattern: &[&[filled(Cobblestone), filled(Cobblestone), filled(Cobblestone)], &[filled(Cobblestone), EMPTY, filled(Cobblestone)], &[filled(Cobblestone), filled(Cobblestone), filled(Cobblestone)]],
        accent: None, message: "Time to smelt something.",
    },
    Recipe { id: "wooden_pickaxe", name: "Wooden Pickaxe", tex: "item/wooden_pickaxe.png", pattern: &[&[filled(OakPlanks), filled(OakPlanks), filled(OakPlanks)], &[EMPTY, filled(Stick), EMPTY], &[EMPTY, filled(Stick), EMPTY]], accent: None, message: "Your first tool. Go mine something." },
    Recipe { id: "wooden_sword", name: "Wooden Sword", tex: "item/wooden_sword.png", pattern: &[&[filled(OakPlanks)], &[filled(OakPlanks)], &[filled(Stick)]], accent: None, message: "En garde!" },
    Recipe { id: "iron_pickaxe", name: "Iron Pickaxe", tex: "item/iron_pickaxe.png", pattern: &[&[filled(IronIngot), filled(IronIngot), filled(IronIngot)], &[EMPTY, filled(Stick), EMPTY], &[EMPTY, filled(Stick), EMPTY]], accent: None, message: "Now we're mining in style." },
    Recipe { id: "diamond_sword", name: "Diamond Sword", tex: "item/diamond_sword.png", pattern: &[&[filled(Diamond)], &[filled(Diamond)], &[filled(Stick)]], accent: Some("diamond"), message: "Sharp — unlocked the Diamond accent." },
    Recipe {
        id: "diamond_block", name: "Block of Diamond", tex: "block/diamond_block.png",
        pattern: &[&[filled(Diamond), filled(Diamond), filled(Diamond)], &[filled(Diamond), filled(Diamond), filled(Diamond)], &[filled(Diamond), filled(Diamond), filled(Diamond)]],
        accent: Some("diamond"), message: "Flexing — unlocked the Diamond accent.",
    },
    Recipe {
        id: "tnt", name: "TNT", tex: "block/tnt_side.png",
        pattern: &[&[filled(Gunpowder), filled(Sand), filled(Gunpowder)], &[filled(Sand), filled(Gunpowder), filled(Sand)], &[filled(Gunpowder), filled(Sand), filled(Gunpowder)]],
        accent: None, message: "…you probably shouldn't have.",
    },
    Recipe { id: "bow", name: "Bow", tex: "item/bow.png", pattern: &[&[EMPTY, filled(Stick), filled(Str)], &[filled(Stick), EMPTY, filled(Str)], &[EMPTY, filled(Stick), filled(Str)]], accent: None, message: "Take aim." },
    Recipe { id: "arrow", name: "Arrow", tex: "item/arrow.png", pattern: &[&[filled(Flint)], &[filled(Stick)], &[filled(Feather)]], accent: None, message: "Ammo secured." },
    Recipe { id: "bread", name: "Bread", tex: "item/bread.png", pattern: &[&[filled(Wheat), filled(Wheat), filled(Wheat)]], accent: None, message: "A snack for the road." },
    Recipe { id: "clock", name: "Clock", tex: "item/clock_00.png", pattern: &[&[EMPTY, filled(GoldIngot), EMPTY], &[filled(GoldIngot), filled(Redstone), filled(GoldIngot)], &[EMPTY, filled(GoldIngot), EMPTY]], accent: None, message: "Always on time now." },
    Recipe { id: "flint_and_steel", name: "Flint and Steel", tex: "item/flint_and_steel.png", pattern: &[&[filled(IronIngot), EMPTY], &[EMPTY, filled(Flint)]], accent: None, message: "Careful with that." },
];

type Grid = [Option<Mat>; 9];

/// The filled cells' bounding box.
fn normalize(grid: &Grid) -> Option<Vec<Vec<Option<Mat>>>> {
    let cells: Vec<(usize, usize)> = (0..9).filter(|&i| grid[i].is_some()).map(|i| (i / 3, i % 3)).collect();
    if cells.is_empty() {
        return None;
    }
    let (r0, r1) = (cells.iter().map(|cell| cell.0).min()?, cells.iter().map(|cell| cell.0).max()?);
    let (c0, c1) = (cells.iter().map(|cell| cell.1).min()?, cells.iter().map(|cell| cell.1).max()?);
    Some((r0..=r1).map(|row_i| (c0..=c1).map(|col_i| grid[row_i * 3 + col_i]).collect()).collect())
}

fn same(shape: &[Vec<Option<Mat>>], pattern: &[&[Option<Mat>]]) -> bool {
    shape.len() == pattern.len() && shape.iter().zip(pattern).all(|(got, want)| got.as_slice() == *want)
}

/// `matchRecipe`: position-independent, mirror-tolerant.
fn match_recipe(grid: &Grid) -> Option<&'static Recipe> {
    let shape = normalize(grid)?;
    let flipped: Vec<Vec<Option<Mat>>> = shape.iter().map(|line| line.iter().rev().copied().collect()).collect();
    RECIPES.iter().find(|recipe| same(&shape, recipe.pattern) || same(&flipped, recipe.pattern))
}

pub struct State {
    grid: Grid,
    show_recipes: bool,
    /// The item on the cursor and where the cursor is, in window points.
    carry: Option<(Mat, [f32; 2])>,
    /// Bumped on each craft so the output slot pops back in.
    crafted: u32,
    discovered: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        State { grid: [None; 9], show_recipes: true, carry: None, crafted: 0, discovered: Vec::new() }
    }
}

pub fn on_start(app: &mut App) {
    app.extras.crafting.discovered = app.prefs.read(KEY, Vec::new());
}

fn close(app: &mut App) {
    app.ui.crafting_open = false;
    let crafting = &mut app.extras.crafting;
    crafting.grid = [None; 9];
    crafting.carry = None;
}

fn craft(app: &mut App) {
    let Some(recipe) = match_recipe(&app.extras.crafting.grid) else { return };
    let crafting = &mut app.extras.crafting;
    if !crafting.discovered.iter().any(|id| id == recipe.id) {
        crafting.discovered.push(recipe.id.to_string());
        let list = crafting.discovered.clone();
        app.prefs.write(KEY, list);
    }
    if let Some(accent) = recipe.accent {
        app.update_settings(|settings| settings.accent = accent.to_string());
    }
    app.toasts.success(format!("{} {}", tf("craft.crafted", &[("name", recipe.name)]), recipe.message));
    let crafting = &mut app.extras.crafting;
    crafting.crafted += 1;
    crafting.grid = [None; 9];
}

#[derive(Clone, Copy)]
enum Source {
    Inventory(Mat),
    Grid(usize),
}

/// Pick up on press (lifting a grid item off the grid), follow the pointer, drop into the grid
/// slot under the release; dropped anywhere else, the item is gone.
fn drag_item(app: &mut App, from: Source, drag: &Drag, tx: &mut Tx, slots: &[Option<[f32; 4]>]) {
    let at = drag.window_point();
    let crafting = &mut app.extras.crafting;
    match drag.phase {
        Phase::Down => {
            let mat = match from {
                Source::Inventory(mat) => Some(mat),
                Source::Grid(i) => crafting.grid[i].take(),
            };
            crafting.carry = mat.map(|mat| (mat, at));
        }
        Phase::Move => {
            if let Some((_, pos)) = &mut crafting.carry {
                *pos = at;
                tx.instant();
            }
        }
        Phase::Up => {
            if let Some((mat, _)) = crafting.carry.take() {
                let hit = slots.iter().position(|rect| rect.is_some_and(|[x, y, w, h]| at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h));
                if let (Some(i), false) = (hit, drag.cancelled()) {
                    crafting.grid[i] = Some(mat);
                }
            }
        }
    }
}

const PANEL: &str = "#c6c6c6";
const SLOT: &str = "#8b8b8b";
const DARK: &str = "#373737";
const FRAME: &str = "#555555";
const INK: &str = "#404040";

/// The recessed vanilla slot: dark top-left, light bottom-right.
fn slot(size: f32) -> El {
    el().size(size, size).center().fill(hex_color(SLOT)).box_shadow(ui::inset_shadow(2.0, 2.0, hex_color(DARK))).box_shadow(ui::inset_shadow(-2.0, -2.0, Color::WHITE))
}

fn item(app: &App, path: &str, size: f32) -> El {
    ui::image(app, &texture_url(path), size, size)
}

fn title(theme: &crate::theme::Theme, label: String) -> El {
    text(label).font(theme.pixel(15.0)).color(hex_color(INK)).ml(2.0).mr(2.0).mt(2.0).mb(8.0)
}

fn arrow() -> El {
    let points = vec![[0.0, 5.0], [12.0, 5.0], [12.0, 1.0], [21.0, 8.0], [12.0, 15.0], [12.0, 11.0], [0.0, 11.0]];
    let shape = Shape::new().fill(points.clone(), hex_color(SLOT)).stroke_with(points, true, 1.0, LineCap::Round, hex_color("#5a5a5a")).view_box(22.0, 16.0);
    canvas(shape).size(30.0, 22.0).a11y_hidden()
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) {
    if !app.ui.crafting_open {
        return;
    }
    let theme = app.theme();
    let crafting = &app.extras.crafting;
    let result = match_recipe(&crafting.grid);
    let slots: Rc<Vec<Option<[f32; 4]>>> = Rc::new((0..9).map(|i| cx.layout(&format!("craft-slot-{i}"))).collect());

    let book = el()
        .key("craft-book")
        .size(40.0, 40.0)
        .center()
        .fill(hex_color(if crafting.show_recipes { "#57a844" } else { "#4a8f3c" }))
        .box_shadow(ui::inset_shadow(2.0, 2.0, hex_color("#7bc25f")))
        .box_shadow(ui::inset_shadow(-2.0, -2.0, hex_color("#2f5f26")))
        .hover(hex_color("#57a844"))
        .role("button")
        .label(i18n::t("craft.title"))
        .help(tf("craft.discovered", &[("n", &crafting.discovered.len().to_string()), ("total", &RECIPES.len().to_string())]))
        .cursor(Cursor::Pointer)
        .on_press(|app: &mut App| app.extras.crafting.show_recipes = !app.extras.crafting.show_recipes)
        .child(item(app, "item/knowledge_book.png", 26.0));

    let mut grid = col().key("craft-grid").gap(2.0);
    for row_i in 0..3 {
        let mut line = row().gap(2.0);
        for col_i in 0..3 {
            let i = row_i * 3 + col_i;
            let slots = slots.clone();
            let mut cell = slot(44.0)
                .key(format!("craft-slot-{i}"))
                .role("button")
                .hover(Color::WHITE.alpha(0.55))
                .cursor(Cursor::Pointer)
                .on_drag(move |app: &mut App, drag: &mut Drag, tx| drag_item(app, Source::Grid(i), drag, tx, &slots));
            if let Some(mat) = crafting.grid[i] {
                cell = cell.label(mat_info(mat).0).child(item(app, mat_info(mat).1, 30.0));
            }
            line = line.child(cell);
        }
        grid = grid.child(line);
    }

    let mut output = slot(52.0).key(format!("craft-output-{}", crafting.crafted)).role("button").transition(Transition::scale(0.9));
    if let Some(recipe) = result {
        let mut glow = el().pin(Some(2.0), Some(2.0), Some(2.0), Some(2.0)).fill(Color::WHITE.alpha(0.35));
        if !theme.reduce_motion {
            glow = glow.looping(rune_motion::LoopSpec::opacity(1.3, 0.0, 0.35, 1.0));
        }
        output = output
            .label(recipe.name)
            .help(recipe.name)
            .hover(Color::WHITE.alpha(0.55))
            .cursor(Cursor::Pointer)
            .on_press(craft)
            .child(glow)
            .child(item(app, recipe.tex, 36.0));
    } else {
        output = output.disabled(true);
    }

    let top = row().center_y().gap(12.0).child(book).child(grid).child(arrow()).child(output);

    let mut inventory = col().key("craft-inv").gap(2.0);
    for row_i in 0..3 {
        let mut line = row().gap(2.0);
        for col_i in 0..9 {
            let i = row_i * 9 + col_i;
            let cell = match MATERIALS.get(i) {
                Some(&(mat, name, path)) => {
                    let slots = slots.clone();
                    slot(40.0)
                        .key(format!("craft-mat-{i}"))
                        .role("button")
                        .label(name)
                        .help(name)
                        .hover(Color::WHITE.alpha(0.55))
                        .cursor(Cursor::Pointer)
                        .on_drag(move |app: &mut App, drag: &mut Drag, tx| drag_item(app, Source::Inventory(mat), drag, tx, &slots))
                        .child(item(app, path, 28.0))
                }
                None => slot(40.0),
            };
            line = line.child(cell);
        }
        inventory = inventory.child(line);
    }

    let mut gui = col()
        .key("craft-gui")
        .role("dialog")
        .label(i18n::t("craft.title"))
        .padding(12.0)
        .fill(hex_color(PANEL))
        .box_shadow(ui::inset_shadow(2.0, 2.0, Color::WHITE))
        .box_shadow(ui::inset_shadow(-2.0, -2.0, hex_color(FRAME)))
        .transition(Transition::scale(0.97))
        .on_press(|_: &mut App| {})
        .child(title(&theme, i18n::t("craft.title")))
        .child(top)
        .child(title(&theme, i18n::t("craft.inventory")).mt(12.0))
        .child(inventory);
    if crafting.show_recipes {
        let mut recipes = col().key("craft-recipes").gap(2.0).mt(8.0);
        for chunk in RECIPES.chunks(9) {
            let mut line = row().gap(2.0);
            for recipe in chunk {
                let found = crafting.discovered.iter().any(|id| id == recipe.id);
                let mut cell = slot(40.0).key(format!("craft-rec-{}", recipe.id));
                if found {
                    cell = cell.help(recipe.name).child(item(app, recipe.tex, 28.0));
                }
                line = line.child(cell);
            }
            recipes = recipes.child(line);
        }
        gui = gui.child(recipes);
    }

    let mut overlay = el()
        .key("craft-overlay")
        .w_full()
        .h_full()
        .absolute(0.0, 0.0)
        .center()
        .padding(24.0)
        .fill(Color::BLACK.alpha(0.55))
        .transition(Transition::opacity())
        .modal(true, close)
        .child(gui);
    if let Some((mat, [x, y])) = crafting.carry {
        overlay = overlay.child(item(app, mat_info(mat).1, 32.0).key("craft-carry").absolute(x - 16.0, y - 16.0).inert());
    }
    layer.push(overlay);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_anywhere_and_mirrored() {
        let mut grid: Grid = [None; 9];
        grid[2] = Some(OakPlanks);
        grid[5] = Some(OakPlanks);
        assert_eq!(match_recipe(&grid).map(|recipe| recipe.id), Some("stick"));
        let mut grid: Grid = [None; 9];
        grid[1] = Some(Flint);
        grid[3] = Some(IronIngot);
        assert!(match_recipe(&grid).is_none());
        let mut grid: Grid = [None; 9];
        grid[3] = Some(Flint);
        grid[1] = Some(IronIngot);
        assert_eq!(match_recipe(&grid).map(|recipe| recipe.id), Some("flint_and_steel"));
        let mut grid: Grid = [None; 9];
        grid[0] = Some(IronIngot);
        grid[4] = Some(Flint);
        assert_eq!(match_recipe(&grid).map(|recipe| recipe.id), Some("flint_and_steel"));
    }
}
