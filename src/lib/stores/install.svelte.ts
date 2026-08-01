import { listen } from "@tauri-apps/api/event";
import { instancesStore } from "./instances.svelte";

interface Progress {
  current: number;
  total: number;
  message: string;
}

class InstallStore {
  progress = $state<Record<string, Progress>>({});
  pending = $state<Progress | null>(null);
  // Downloads that have been kicked off but whose first progress event hasn't
  // arrived yet. Kept as a count of outstanding tokens so the dock shows a
  // placeholder immediately for each pack — without this, starting two packs
  // back-to-back would only surface the second once its first event landed.
  starting = $state(0);
  private startTokens = new Set<number>();
  private nextToken = 0;
  private started = false;

  init() {
    if (this.started) return;
    this.started = true;
    listen<{
      instanceId: string | null;
      current: number;
      total: number;
      message: string;
    }>("modpack-progress", (event) => {
      const { instanceId, current, total, message } = event.payload;
      if (!instanceId) {
        this.pending = message === "Done" ? null : { current, total, message };
        return;
      }
      this.pending = null;

      if (message === "Done") {
        const { [instanceId]: _done, ...rest } = this.progress;
        this.progress = rest;
        instancesStore.refresh();
        return;
      }

      const isNew = !(instanceId in this.progress);
      this.progress = { ...this.progress, [instanceId]: { current, total, message } };
      if (isNew) {
        this.consumeStart();
        instancesStore.refresh();
      }
    });
  }

  beginStart(): number {
    const token = this.nextToken++;
    this.startTokens.add(token);
    this.starting = this.startTokens.size;
    return token;
  }
  // Progress events don't carry the caller's token, so when any download's
  // first event arrives we retire the oldest outstanding token (FIFO): that
  // download is now tracked in `progress` and no longer merely "starting".
  private consumeStart() {
    const first = this.startTokens.values().next();
    if (!first.done) {
      this.startTokens.delete(first.value);
      this.starting = this.startTokens.size;
    }
  }
  endStart(token: number) {
    if (this.startTokens.delete(token)) this.starting = this.startTokens.size;
  }

  isInstalling(id: string): boolean {
    return id in this.progress;
  }
  progressFor(id: string): Progress | undefined {
    return this.progress[id];
  }
  pct(id: string): number | null {
    const progress = this.progress[id];
    return progress ? toPct(progress.current, progress.total) : null;
  }

  clearPending() {
    this.pending = null;
  }

  anyActive(): boolean {
    return (
      this.starting > 0 ||
      this.pending !== null ||
      Object.keys(this.progress).length > 0
    );
  }
  primaryInstanceId(): string | null {
    return Object.keys(this.progress)[0] ?? null;
  }
  overallPct(): number | null {
    const id = this.primaryInstanceId();
    if (id) return this.pct(id);
    return this.pending ? toPct(this.pending.current, this.pending.total) : null;
  }
  overallMessage(): string {
    const id = this.primaryInstanceId();
    return this.progressFor(id ?? "")?.message ?? this.pending?.message ?? "Installing…";
  }
}

export function toPct(current: number, total: number): number | null {
  return total > 0 ? Math.round((current / total) * 100) : null;
}

export const installStore = new InstallStore();
