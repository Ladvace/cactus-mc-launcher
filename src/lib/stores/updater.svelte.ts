import { browser } from "$app/environment";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "$lib/stores/toast.svelte";

type Phase = "idle" | "available" | "downloading" | "installing";

// Only meaningful inside the Tauri shell (skipped in browser/SSR/tests).
const inTauri = browser && "__TAURI_INTERNALS__" in window;

class Updater {
  phase = $state<Phase>("idle");
  update = $state<Update | null>(null);
  downloaded = $state(0);
  total = $state(0);
  checking = $state(false);

  get pct(): number | null {
    return this.total > 0 ? Math.round((this.downloaded / this.total) * 100) : null;
  }

  /** Check for an update. `manual` surfaces a result toast (up-to-date / error). */
  async check(manual = false) {
    if (!inTauri || this.checking || this.phase === "downloading" || this.phase === "installing") {
      return;
    }
    this.checking = true;
    try {
      const found = await check();
      if (found) {
        this.update = found;
        this.phase = "available";
      } else if (manual) {
        toast.success("You're on the latest version.");
      }
    } catch (err) {
      if (manual) toast.error(`Update check failed: ${String(err)}`);
    } finally {
      this.checking = false;
    }
  }

  dismiss() {
    this.phase = "idle";
  }

  async install() {
    if (!this.update) return;
    try {
      this.phase = "downloading";
      this.downloaded = 0;
      this.total = 0;
      await this.update.downloadAndInstall((event) => {
        if (event.event === "Started") this.total = event.data.contentLength ?? 0;
        else if (event.event === "Progress") this.downloaded += event.data.chunkLength;
        else if (event.event === "Finished") this.phase = "installing";
      });
      await relaunch();
    } catch (err) {
      toast.error(`Update failed: ${String(err)}`);
      this.phase = "available";
    }
  }
}

export const updater = new Updater();
