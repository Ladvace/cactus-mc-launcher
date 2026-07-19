import { readJson, writeJson } from "$lib/storage";

export interface Cell {
  w: number;
  h: number;
  order: number;
}

const KEY = "cactus:instanceLayout";

// Legacy size-label → span, for migrating older saved layouts.
const LEGACY: Record<string, [number, number]> = {
  s: [1, 1],
  w: [2, 1],
  t: [1, 2],
  l: [2, 2],
};

function load(): Record<string, Cell> {
  const raw = readJson<Record<string, any>>(KEY, {});
  const out: Record<string, Cell> = {};
  for (const [id, value] of Object.entries(raw)) {
    if (value && typeof value.w === "number" && typeof value.h === "number") {
      out[id] = { w: value.w, h: value.h, order: value.order ?? 0 };
    } else if (value && typeof value.size === "string" && LEGACY[value.size]) {
      const [width, height] = LEGACY[value.size];
      out[id] = { w: width, h: height, order: value.order ?? 0 };
    }
  }
  return out;
}

class InstanceLayout {
  cells = $state<Record<string, Cell>>(load());

  cellOf(id: string): Cell {
    return this.cells[id] ?? { w: 1, h: 1, order: Number.MAX_SAFE_INTEGER };
  }

  set(id: string, cell: Cell) {
    this.cells = { ...this.cells, [id]: cell };
    this.save();
  }

  reorder(ids: string[]) {
    const next = { ...this.cells };
    ids.forEach((id, index) => {
      const cell = this.cellOf(id);
      next[id] = { w: cell.w, h: cell.h, order: index };
    });
    this.cells = next;
    this.save();
  }

  reset() {
    this.cells = {};
    this.save();
  }

  get customized(): boolean {
    return Object.keys(this.cells).length > 0;
  }

  private save() {
    writeJson(KEY, this.cells);
  }
}

export const instanceLayout = new InstanceLayout();
