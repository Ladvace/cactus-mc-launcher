import { listen } from "@tauri-apps/api/event";
import { api } from "$lib/api";
import type { AccountInfo, DeviceCodeEvent } from "$lib/types";
import { settingsStore } from "./settings.svelte";

export interface DeviceCodeState extends DeviceCodeEvent {
  status: "waiting" | "authorizing";
}

class AccountsStore {
  accounts = $state<AccountInfo[]>([]);
  activeId = $state<string | null>(null);
  microsoftConfigured = $state(false);
  loaded = $state(false);

  deviceCode = $state<DeviceCodeState | null>(null);
  loginError = $state<string | null>(null);
  loggingIn = $state(false);

  #started = false;

  async init() {
    if (this.#started) return;
    this.#started = true;

    await listen<DeviceCodeEvent>("auth-device-code", (event) => {
      this.deviceCode = { ...event.payload, status: "waiting" };
    });
    await listen("auth-login-done", () => {
      if (this.deviceCode) this.deviceCode.status = "authorizing";
    });

    await this.refresh();
  }

  async refresh() {
    const state = await api.getAccounts();
    this.accounts = state.accounts;
    this.activeId = state.activeId;
    this.microsoftConfigured = state.microsoftConfigured;
    this.loaded = true;
  }

  get active(): AccountInfo | null {
    return this.accounts.find((account) => account.id === this.activeId) ?? null;
  }

  get activeName(): string {
    return this.active?.username ?? settingsStore.settings.offlineUsername ?? "Player";
  }

  async login() {
    this.loggingIn = true;
    this.loginError = null;
    this.deviceCode = null;
    try {
      await api.loginMicrosoft();
      await this.refresh();
    } catch (error) {
      this.loginError = String(error);
    } finally {
      this.loggingIn = false;
      this.deviceCode = null;
    }
  }

  async setActive(id: string | null) {
    await api.setActiveAccount(id);
    this.activeId = id;
  }

  async remove(id: string) {
    await api.removeAccount(id);
    await this.refresh();
  }
}

export const accountsStore = new AccountsStore();
