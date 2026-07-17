import { browser } from "$app/environment";

// Per-group (folder) cover images, keyed by group name. Groups are implicit
// (just a label on instances), so their cover art lives here in localStorage.

const KEY = "cactus:groupCovers";

function load(): Record<string, string> {
  if (!browser) return {};
  try {
    return JSON.parse(localStorage.getItem(KEY) || "{}");
  } catch {
    return {};
  }
}

class GroupCovers {
  covers = $state<Record<string, string>>(load());

  private persist() {
    if (browser) localStorage.setItem(KEY, JSON.stringify(this.covers));
  }

  get(name: string): string | null {
    return this.covers[name] ?? null;
  }

  set(name: string, uri: string) {
    this.covers = { ...this.covers, [name]: uri };
    this.persist();
  }

  clear(name: string) {
    const next = { ...this.covers };
    delete next[name];
    this.covers = next;
    this.persist();
  }

  /** Keep the cover when a folder is renamed. */
  rename(from: string, to: string) {
    const uri = this.covers[from];
    if (!uri || from === to) return;
    const next = { ...this.covers };
    next[to] = uri;
    delete next[from];
    this.covers = next;
    this.persist();
  }
}

export const groupCovers = new GroupCovers();
