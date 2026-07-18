import { listen } from "@tauri-apps/api/event";
import { instancesStore } from "./instances.svelte";

interface Progress {
  current: number;
  total: number;
  message: string;
}

/// Tracks modpack installs in flight, keyed by the instance being created, so
/// tiles + the dock can show progress even after the install modal is closed.
class InstallStore {
  progress = $state<Record<string, Progress>>({});
  // The pre-creation phase of a modpack install (downloading the pack, resolving
  // versions) before an instance exists — so the dock can show it immediately.
  pending = $state<Progress | null>(null);
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
      this.pending = null; // the instance now exists; tracked per-id below

      if (message === "Done") {
        const { [instanceId]: _done, ...rest } = this.progress;
        this.progress = rest;
        instancesStore.refresh();
        return;
      }

      const isNew = !(instanceId in this.progress);
      this.progress = { ...this.progress, [instanceId]: { current, total, message } };
      // The instance was just created backend-side — pull it into the list so
      // its tile appears (with this progress) right away.
      if (isNew) instancesStore.refresh();
    });
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

  /** Clear the pre-creation phase (e.g. the install failed before an instance). */
  clearPending() {
    this.pending = null;
  }

  /** Any modpack install in flight (pre-creation or downloading into a tile). */
  anyActive(): boolean {
    return this.pending !== null || Object.keys(this.progress).length > 0;
  }
  /** The instance a global indicator should link to, if one exists yet. */
  primaryInstanceId(): string | null {
    return Object.keys(this.progress)[0] ?? null;
  }
  /** Representative percent for a global indicator (per-instance, else pending). */
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
