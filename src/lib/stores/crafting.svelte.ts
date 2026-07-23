import { readJson, writeJson } from "$lib/storage";

const KEY = "cactus:crafted";

/** Which secret recipes the player has discovered (persisted). */
class CraftingStore {
  discovered = $state<string[]>(readJson<string[]>(KEY, []));

  has(id: string): boolean {
    return this.discovered.includes(id);
  }

  /** Record a discovery; returns true if it was new. */
  discover(id: string): boolean {
    if (this.has(id)) return false;
    this.discovered = [...this.discovered, id];
    writeJson(KEY, this.discovered);
    return true;
  }
}

export const craftingStore = new CraftingStore();
