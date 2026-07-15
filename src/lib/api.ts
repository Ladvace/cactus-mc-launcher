import { invoke } from "@tauri-apps/api/core";
import type {
  AccountInfo,
  AccountsState,
  ContentItem,
  CreateInstance,
  Instance,
  LoaderVersion,
  ModLoader,
  ModrinthVersion,
  SearchParams,
  SearchResults,
  Settings,
  Source,
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

  // Content (source-agnostic; defaults to Modrinth)
  listSources: () => invoke<{ id: string; enabled: boolean }[]>("list_sources"),

  searchContent: (source: Source, params: SearchParams) =>
    invoke<SearchResults>("search_content", { source, params }),

  contentVersions: (
    source: Source,
    projectId: string,
    loader?: string | null,
    gameVersion?: string | null
  ) =>
    invoke<ModrinthVersion[]>("content_versions", {
      source,
      projectId,
      loader: loader ?? null,
      gameVersion: gameVersion ?? null,
    }),

  installContent: (args: {
    instanceId: string;
    source: Source;
    versionId: string;
    projectType: string;
    title: string;
    iconUrl?: string | null;
  }) => invoke<ContentItem>("install_content", { ...args, iconUrl: args.iconUrl ?? null }),

  listContent: (instanceId: string) =>
    invoke<ContentItem[]>("list_content", { instanceId }),

  setContentEnabled: (instanceId: string, versionId: string, enabled: boolean) =>
    invoke<void>("set_content_enabled", { instanceId, versionId, enabled }),

  removeContent: (instanceId: string, versionId: string) =>
    invoke<void>("remove_content", { instanceId, versionId }),

  installModpack: (versionId: string, iconUrl?: string | null) =>
    invoke<Instance>("install_modpack", { versionId, iconUrl: iconUrl ?? null }),
};
