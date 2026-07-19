import { api } from "$lib/api";
import type { Settings } from "$lib/types";

const DEFAULTS: Settings = {
  theme: "dark",
  javaPath: null,
  javaPaths: {},
  maxMemoryMb: 4096,
  minMemoryMb: 1024,
  maxConcurrentDownloads: 16,
  jvmArgs: "",
  gameWidth: 854,
  gameHeight: 480,
  offlineUsername: "Player",
  background: "",
  uiSounds: true,
  giphyApiKey: "",
  dockPosition: "bottom",
  decorTheme: "",
  dockMagnify: true,
  instancesDir: "",
  ngrokAuthtoken: "",
  showNews: true,
  newsSingle: false,
};

class SettingsStore {
  settings = $state<Settings>({ ...DEFAULTS });
  loaded = $state(false);

  async ensureLoaded() {
    if (this.loaded) return;
    try {
      this.settings = await api.getSettings();
    } catch {
      this.settings = { ...DEFAULTS };
    }
    this.loaded = true;
  }

  async save(next: Settings) {
    // Persist to the backend first, then update in-memory state, so reactive
    // consumers only see the change once the backend can serve requests with it.
    await api.saveSettings(next);
    this.settings = next;
  }
}

export const settingsStore = new SettingsStore();
