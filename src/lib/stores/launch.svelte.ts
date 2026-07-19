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

class LaunchStore {
  byId = $state<Record<string, InstanceRuntime>>({});
  #started = false;

  async init() {
    if (this.#started) return;
    this.#started = true;

    await listen<LaunchStatusEvent>("launch-status", (event) => {
      const runtime = this.ensure(event.payload.instanceId);
      runtime.state = event.payload.state;
      runtime.message = event.payload.message;
      if (event.payload.state === "exited" || event.payload.state === "error") {
        runtime.current = 0;
        runtime.total = 0;
        runtime.stage = "";
        instancesStore.refresh();
      }
    });

    await listen<LaunchProgressEvent>("launch-progress", (event) => {
      const runtime = this.ensure(event.payload.instanceId);
      runtime.stage = event.payload.stage;
      runtime.current = event.payload.current;
      runtime.total = event.payload.total;
    });

    await listen<LaunchLogEvent>("launch-log", (event) => {
      const runtime = this.ensure(event.payload.instanceId);
      runtime.logs.push(event.payload.line);
      if (runtime.logs.length > MAX_LOG_LINES) {
        runtime.logs.splice(0, runtime.logs.length - MAX_LOG_LINES);
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

  isBusy(id: string): boolean {
    const state = this.byId[id]?.state;
    return state === "preparing" || state === "downloading" || state === "launching";
  }

  isRunning(id: string): boolean {
    return this.byId[id]?.state === "running";
  }

  async launch(id: string) {
    const runtime = this.ensure(id);
    runtime.logs = [];
    runtime.state = "preparing";
    runtime.message = "Starting…";
    runtime.current = 0;
    runtime.total = 0;
    try {
      await api.launchInstance(id);
    } catch (error) {
      runtime.state = "error";
      runtime.message = String(error);
    }
  }

  async stop(id: string) {
    try {
      await api.stopInstance(id);
    } catch (error) {
      console.error("stop failed", error);
    }
  }
}

export const launchStore = new LaunchStore();
