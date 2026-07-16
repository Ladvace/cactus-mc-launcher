import { browser } from "$app/environment";
import { boardApi } from "$lib/boardApi";
import { boardAuth } from "$lib/stores/boardAuth.svelte";
import type { PresencePlayer } from "$lib/types";

const KEY = "cactus:presence";
const POLL_MS = 20000;

interface Persisted {
  enabled: boolean;
  status: string;
  serverAddress: string;
}

function load(): Persisted {
  const base: Persisted = { enabled: false, status: "", serverAddress: "" };
  if (!browser) return base;
  try {
    return { ...base, ...JSON.parse(localStorage.getItem(KEY) || "{}") };
  } catch {
    return base;
  }
}

/// Opt-in "who's online" presence. While the panel is open it polls the list
/// every ~20s (and heartbeats the player's own presence when they've chosen to
/// appear online). Leaving the panel drops the player from the list.
class PresenceStore {
  enabled = $state(false); // broadcast my presence to others
  status = $state("");
  serverAddress = $state("");
  players = $state<PresencePlayer[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  private timer: ReturnType<typeof setInterval> | null = null;

  constructor() {
    const p = load();
    this.enabled = p.enabled;
    this.status = p.status;
    this.serverAddress = p.serverAddress;
  }

  private persist() {
    if (!browser) return;
    localStorage.setItem(
      KEY,
      JSON.stringify({
        enabled: this.enabled,
        status: this.status,
        serverAddress: this.serverAddress,
      })
    );
  }

  /** Start polling; call when the panel mounts. */
  open() {
    void this.poll();
    this.timer ??= setInterval(() => void this.poll(), POLL_MS);
  }

  /** Stop polling and drop out of the list; call when the panel unmounts. */
  close() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
    if (this.enabled) void this.goOffline();
  }

  async setEnabled(v: boolean) {
    this.enabled = v;
    this.persist();
    if (v) await this.poll();
    else await this.goOffline();
  }

  /** Update the broadcast fields; re-heartbeats immediately when online. */
  saveFields(status: string, serverAddress: string) {
    this.status = status;
    this.serverAddress = serverAddress;
    this.persist();
    if (this.enabled) void this.heartbeat();
  }

  private async heartbeat() {
    const t = boardAuth.token;
    if (!t) return;
    await boardApi.setPresence(t, {
      status: this.status,
      serverAddress: this.serverAddress,
    });
  }

  private async goOffline() {
    const t = boardAuth.token;
    if (!t) return;
    await boardApi.clearPresence(t).catch(() => {});
  }

  /** One cycle: heartbeat (if online) then refresh the list. Safe to call anytime. */
  async poll() {
    const t = boardAuth.token;
    if (!t) return;
    this.loading = this.players.length === 0;
    try {
      if (this.enabled) await this.heartbeat();
      this.players = await boardApi.listPresence(t);
      this.error = null;
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }
}

export const presence = new PresenceStore();
