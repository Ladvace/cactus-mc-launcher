import { invoke } from "@tauri-apps/api/core";
import type {
  AccountInfo,
  AccountsState,
  CacheStats,
  ContentItem,
  ExportResult,
  ImportResult,
  CreateInstance,
  Instance,
  LoaderVersion,
  ModLoader,
  ModrinthVersion,
  OpEntry,
  PlayerEntry,
  SearchParams,
  SearchResults,
  ServerStatus,
  Settings,
  Source,
  Sticker,
  UpdateInstance,
  VersionList,
  WorldInfo,
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

  /** Create a dedicated-server instance from a client instance (copies mods/config). */
  createServerFrom: (id: string) =>
    invoke<Instance>("create_server_from", { id }),

  /** The instance's game folder path (for revealing it in the file manager). */
  instanceFolder: (id: string) => invoke<string>("instance_folder", { id }),

  /** Move an instance's game data to `path` (null = back to the default). */
  setInstanceGameDir: (id: string, path: string | null) =>
    invoke<Instance>("set_instance_game_dir", { id, path }),

  getSettings: () => invoke<Settings>("get_settings"),

  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),

  getMinecraftVersions: () => invoke<VersionList>("get_minecraft_versions"),

  getLoaderVersions: (loader: ModLoader, mcVersion: string) =>
    invoke<LoaderVersion[]>("get_loader_versions", { loader, mcVersion }),

  launchInstance: (id: string) => invoke<void>("launch_instance", { id }),

  stopInstance: (id: string) => invoke<void>("stop_instance", { id }),

  sendServerCommand: (id: string, command: string) =>
    invoke<void>("send_server_command", { id, command }),

  readServerProperties: (id: string) =>
    invoke<string>("read_server_properties", { id }),

  writeServerProperties: (id: string, content: string) =>
    invoke<void>("write_server_properties", { id, content }),

  listWorlds: (id: string) => invoke<WorldInfo[]>("list_worlds", { id }),

  backupWorld: (id: string, folder: string) =>
    invoke<string>("backup_world", { id, folder }),

  deleteWorld: (id: string, folder: string) =>
    invoke<void>("delete_world", { id, folder }),

  getLocalIp: () => invoke<string | null>("get_local_ip"),

  readOps: (id: string) => invoke<OpEntry[]>("read_ops", { id }),
  readWhitelist: (id: string) => invoke<PlayerEntry[]>("read_whitelist", { id }),
  addOp: (id: string, name: string, level = 4) =>
    invoke<void>("add_op", { id, name, level }),
  removeOp: (id: string, name: string) => invoke<void>("remove_op", { id, name }),
  addWhitelist: (id: string, name: string) =>
    invoke<void>("add_whitelist", { id, name }),
  removeWhitelist: (id: string, name: string) =>
    invoke<void>("remove_whitelist", { id, name }),

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

  installModpack: (source: Source, versionId: string, iconUrl?: string | null) =>
    invoke<Instance>("install_modpack", {
      source,
      versionId,
      iconUrl: iconUrl ?? null,
    }),

  // Stickers (Giphy)

  searchStickers: (query: string, offset = 0) =>
    invoke<Sticker[]>("search_stickers", { query, offset }),

  downloadImage: (url: string) => invoke<string>("download_image", { url }),

  contentCacheStats: () => invoke<CacheStats>("content_cache_stats"),

  // Snapshots (share / export-import)
  exportSetup: (
    instanceId: string,
    format: "cactuspack" | "mrpack" = "cactuspack",
    note?: string | null
  ) =>
    invoke<ExportResult>("export_setup", {
      instanceId,
      format,
      note: note ?? null,
    }),

  importSetup: (bytes: number[]) =>
    invoke<ImportResult>("import_setup", { bytes }),

  /** Whether an instance is shareable (all CurseForge items re-downloadable). */
  instanceShareCheck: (instanceId: string) =>
    invoke<{ ok: boolean; optOut: string[] }>("instance_share_check", {
      instanceId,
    }),

  /** Change the active account's Minecraft skin (PNG bytes + classic/slim). */
  setSkin: (bytes: number[], variant: "classic" | "slim") =>
    invoke<void>("set_skin", { bytes, variant }),

  /** Capes owned by the active account. */
  getCapes: () =>
    invoke<{ id: string; alias: string; url: string; active: boolean }[]>(
      "get_capes"
    ),

  /** Set the active cape (id) or hide it (null). */
  setCape: (capeId: string | null) => invoke<void>("set_cape", { capeId }),

  /** Live Server List Ping for `host[:port]` (default port 25565). */
  pingServer: (address: string) =>
    invoke<ServerStatus>("ping_server", { address }),

  /** Open an ngrok tunnel to the local server; returns the public host:port. */
  tunnelStart: (authtoken: string, port = 25565) =>
    invoke<string>("tunnel_start", { authtoken, port }),

  /** Stop the running ngrok tunnel. */
  tunnelStop: () => invoke<void>("tunnel_stop"),
};
