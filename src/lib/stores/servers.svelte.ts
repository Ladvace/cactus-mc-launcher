import { readJson, writeJson } from "$lib/storage";
import { FEATURED_SERVERS, type FeaturedServer } from "$lib/servers";

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

  has(address: string): boolean {
    const addr = normalizeAddress(address);
    return this.servers.some((s) => normalizeAddress(s.address) === addr);
  }

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

  reset() {
    this.servers = [...FEATURED_SERVERS];
    this.persist();
  }
}

export const serversStore = new ServersStore();
