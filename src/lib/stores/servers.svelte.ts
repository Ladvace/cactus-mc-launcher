import { readJson, writeJson } from "$lib/storage";
import { FEATURED_SERVERS, type FeaturedServer } from "$lib/servers";

// The user's Servers page list. Seeded once from the curated defaults, then
// fully user-owned: they can remove any (including defaults) and add their own.
const KEY = "cactus:servers";

function load(): FeaturedServer[] {
  // `null` fallback distinguishes "never set" (seed defaults) from "emptied".
  return readJson<FeaturedServer[] | null>(KEY, null) ?? [...FEATURED_SERVERS];
}

function normalizeAddress(address: string): string {
  return address.trim().toLowerCase();
}

class ServersStore {
  servers = $state<FeaturedServer[]>(load());

  private persist() {
    writeJson(KEY, this.servers);
  }

  /** True if an entry with this address already exists (case-insensitive). */
  has(address: string): boolean {
    const addr = normalizeAddress(address);
    return this.servers.some((s) => normalizeAddress(s.address) === addr);
  }

  /** Add a custom server. Returns false if the address is blank or a duplicate. */
  add(server: FeaturedServer): boolean {
    const address = server.address.trim();
    if (!address || this.has(address)) return false;
    this.servers = [
      ...this.servers,
      { ...server, name: server.name.trim() || address, address },
    ];
    this.persist();
    return true;
  }

  remove(address: string) {
    const addr = normalizeAddress(address);
    this.servers = this.servers.filter((s) => normalizeAddress(s.address) !== addr);
    this.persist();
  }

  /** Restore the curated default list. */
  reset() {
    this.servers = [...FEATURED_SERVERS];
    this.persist();
  }
}

export const serversStore = new ServersStore();
