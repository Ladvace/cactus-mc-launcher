import { listen } from "@tauri-apps/api/event";
import { api } from "$lib/api";
import type {
  LaunchLogEvent,
  LaunchProgressEvent,
  LaunchState,
  LaunchStatusEvent,
} from "$lib/types";
import { instancesStore } from "./instances.svelte";

const MAX_LOG_LINES = 2000;

export interface InstanceRuntime {
  state: LaunchState | "idle";
  message: string | null;
  stage: string;
  current: number;
  total: number;
  logs: string[];
}

function blank(): InstanceRuntime {
  return { state: "idle", message: null, stage: "", current: 0, total: 0, logs: [] };
}

/// Tracks live launch state per instance, driven by backend events.
class LaunchStore {
  byId = $state<Record<string, InstanceRuntime>>({});
  #started = false;

  /** Subscribe to backend launch events once. */
  async init() {
    if (this.#started) return;
    this.#started = true;

    await listen<LaunchStatusEvent>("launch-status", (e) => {
      const r = this.ensure(e.payload.instanceId);
      r.state = e.payload.state;
      r.message = e.payload.message;
      if (e.payload.state === "exited" || e.payload.state === "error") {
        r.current = 0;
        r.total = 0;
        r.stage = "";
        // Refresh playtime / last-played after a session ends.
        instancesStore.refresh();
      }
    });

    await listen<LaunchProgressEvent>("launch-progress", (e) => {
      const r = this.ensure(e.payload.instanceId);
      r.stage = e.payload.stage;
      r.current = e.payload.current;
      r.total = e.payload.total;
    });

    await listen<LaunchLogEvent>("launch-log", (e) => {
      const r = this.ensure(e.payload.instanceId);
      r.logs.push(e.payload.line);
      if (r.logs.length > MAX_LOG_LINES) {
        r.logs.splice(0, r.logs.length - MAX_LOG_LINES);
      }
    });
  }

  ensure(id: string): InstanceRuntime {
    if (!this.byId[id]) this.byId[id] = blank();
    return this.byId[id];
  }

  get(id: string): InstanceRuntime {
    return this.byId[id] ?? blank();
  }

  /** Preparing / downloading / launching — busy but not yet playing. */
  isBusy(id: string): boolean {
    const s = this.byId[id]?.state;
    return s === "preparing" || s === "downloading" || s === "launching";
  }

  isRunning(id: string): boolean {
    return this.byId[id]?.state === "running";
  }

  async launch(id: string) {
    const r = this.ensure(id);
    r.logs = [];
    r.state = "preparing";
    r.message = "Starting…";
    r.current = 0;
    r.total = 0;
    try {
      await api.launchInstance(id);
    } catch (e) {
      r.state = "error";
      r.message = String(e);
    }
  }

  async stop(id: string) {
    try {
      await api.stopInstance(id);
    } catch (e) {
      console.error("stop failed", e);
    }
  }
}

export const launchStore = new LaunchStore();
