import { boardApi } from "$lib/boardApi";
import { readJson, writeJson } from "$lib/storage";
import { boardAuth } from "$lib/stores/boardAuth.svelte";
import type { PresencePlayer } from "$lib/types";

const KEY = "cactus:presence";
const POLL_MS = 20000;

interface Persisted {
  enabled: boolean;
  status: string;
  serverAddress: string;
  mcVersion: string;
  loader: string;
}

function load(): Persisted {
  const base: Persisted = {
    enabled: false,
    status: "",
    serverAddress: "",
    mcVersion: "",
    loader: "",
  };
  return { ...base, ...readJson<Partial<Persisted>>(KEY, {}) };
}

class PresenceStore {
  enabled = $state(false);
  status = $state("");
  serverAddress = $state("");
  mcVersion = $state("");
  loader = $state("");
  players = $state<PresencePlayer[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  private timer: ReturnType<typeof setInterval> | null = null;
  private polling = false;

  constructor() {
    const persisted = load();
    this.enabled = persisted.enabled;
    this.status = persisted.status;
    this.serverAddress = persisted.serverAddress;
    this.mcVersion = persisted.mcVersion;
    this.loader = persisted.loader;
  }

  private persist() {
    writeJson(KEY, {
      enabled: this.enabled,
      status: this.status,
      serverAddress: this.serverAddress,
      mcVersion: this.mcVersion,
      loader: this.loader,
    });
  }

  open() {
    void this.poll();
    this.timer ??= setInterval(() => void this.poll(), POLL_MS);
  }

  close() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
    if (this.enabled) void this.goOffline();
  }

  async setEnabled(enabled: boolean) {
    this.enabled = enabled;
    this.persist();
    if (enabled) await this.poll();
    else await this.goOffline();
  }

  saveFields(fields: {
    status: string;
    serverAddress: string;
    mcVersion: string;
    loader: string;
  }) {
    this.status = fields.status;
    this.serverAddress = fields.serverAddress;
    this.mcVersion = fields.mcVersion;
    this.loader = fields.loader;
    this.persist();
    if (this.enabled) void this.heartbeat().catch(() => {});
  }

  private async heartbeat() {
    const token = boardAuth.token;
    if (!token) return;
    await boardApi.setPresence(token, {
      status: this.status,
      serverAddress: this.serverAddress,
      mcVersion: this.mcVersion,
      loader: this.loader,
    });
  }

  private async goOffline() {
    const token = boardAuth.token;
    if (!token) return;
    await boardApi.clearPresence(token).catch(() => {});
  }

  async poll() {
    const token = boardAuth.token;
    if (!token || this.polling) return; // one cycle at a time — no stale overwrites
    this.polling = true;
    this.loading = this.players.length === 0;
    try {
      if (this.enabled) await this.heartbeat();
      this.players = await boardApi.listPresence(token);
      this.error = null;
    } catch (error) {
      this.error = String(error);
    } finally {
      this.loading = false;
      this.polling = false;
    }
  }
}

export const presence = new PresenceStore();
