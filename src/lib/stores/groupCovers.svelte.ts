import { readJson, writeJson } from "$lib/storage";

const KEY = "cactus:groupCovers";

function load(): Record<string, string> {
  return readJson<Record<string, string>>(KEY, {});
}

class GroupCovers {
  covers = $state<Record<string, string>>(load());

  private persist() {
    writeJson(KEY, this.covers);
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
