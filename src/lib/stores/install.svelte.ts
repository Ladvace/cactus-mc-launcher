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
      if (!instanceId) return; // pre-creation phase (no instance yet)

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
    return progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : null;
  }
}

export const installStore = new InstallStore();
