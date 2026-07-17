import { browser } from "$app/environment";
import { writeJson } from "$lib/storage";

// Per-instance tile layout (span + order), shared across every InstanceGrid so
// grouped grids don't clobber each other's entries. Persisted to localStorage.
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
  if (!browser) return {};
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "{}") ?? {};
    const out: Record<string, Cell> = {};
    for (const [id, v] of Object.entries<any>(raw)) {
      if (v && typeof v.w === "number" && typeof v.h === "number") {
        out[id] = { w: v.w, h: v.h, order: v.order ?? 0 };
      } else if (v && typeof v.size === "string" && LEGACY[v.size]) {
        const [w, h] = LEGACY[v.size];
        out[id] = { w, h, order: v.order ?? 0 };
      }
    }
    return out;
  } catch {
    return {};
  }
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

  /** Re-index the order of the given ids (a single grid's items), preserving
      every other instance's entry. */
  reorder(ids: string[]) {
    const next = { ...this.cells };
    ids.forEach((id, idx) => {
      const c = this.cellOf(id);
      next[id] = { w: c.w, h: c.h, order: idx };
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
