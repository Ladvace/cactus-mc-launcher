import { invoke } from "@tauri-apps/api/core";
import type {
  AccountInfo,
  AccountsState,
  CreateInstance,
  Instance,
  LoaderVersion,
  ModLoader,
  Settings,
  UpdateInstance,
  VersionList,
} from "./types";

/// Thin typed wrapper around the Rust command layer.
export const api = {
  listInstances: () => invoke<Instance[]>("list_instances"),

  getInstance: (id: string) => invoke<Instance>("get_instance", { id }),

  createInstance: (payload: CreateInstance) =>
    invoke<Instance>("create_instance", { payload }),

  updateInstance: (id: string, patch: UpdateInstance) =>
    invoke<Instance>("update_instance", { id, patch }),

  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),

  getSettings: () => invoke<Settings>("get_settings"),

  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),

  getMinecraftVersions: () => invoke<VersionList>("get_minecraft_versions"),

  getLoaderVersions: (loader: ModLoader, mcVersion: string) =>
    invoke<LoaderVersion[]>("get_loader_versions", { loader, mcVersion }),

  launchInstance: (id: string) => invoke<void>("launch_instance", { id }),

  stopInstance: (id: string) => invoke<void>("stop_instance", { id }),

  isInstanceRunning: (id: string) =>
    invoke<boolean>("is_instance_running", { id }),

  setupJava: () => invoke<string[]>("setup_java"),

  loginMicrosoft: () => invoke<AccountInfo>("login_microsoft"),

  getAccounts: () => invoke<AccountsState>("get_accounts"),

  setActiveAccount: (id: string | null) =>
    invoke<void>("set_active_account", { id }),

  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
};
