// Mirrors the Rust types in `src-tauri/src`. Keep these in sync.

export type ModLoader = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

export const MOD_LOADERS: { value: ModLoader; label: string }[] = [
  { value: "vanilla", label: "Vanilla" },
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

export interface Instance {
  id: string;
  name: string;
  icon: string | null;
  mcVersion: string;
  loader: ModLoader;
  loaderVersion: string | null;
  group: string | null;
  created: string; // ISO 8601
  lastPlayed: string | null;
  totalPlaytimeSeconds: number;
}

export interface CreateInstance {
  name: string;
  mcVersion: string;
  loader?: ModLoader;
  loaderVersion?: string | null;
  icon?: string | null;
}

export interface UpdateInstance {
  name?: string;
  icon?: string;
  group?: string;
  mcVersion?: string;
  loader?: ModLoader;
  loaderVersion?: string;
}

export interface Settings {
  theme: string;
  javaPath: string | null;
  maxMemoryMb: number;
  minMemoryMb: number;
  jvmArgs: string;
  gameWidth: number;
  gameHeight: number;
  offlineUsername: string;
}

// --- Launch events (mirror the Rust payloads in src-tauri/src/launch) ---

export type LaunchState =
  | "preparing"
  | "downloading"
  | "launching"
  | "running"
  | "exited"
  | "error";

export interface LaunchStatusEvent {
  instanceId: string;
  state: LaunchState;
  message: string | null;
}

export interface LaunchProgressEvent {
  instanceId: string;
  stage: string;
  current: number;
  total: number;
}

export interface LaunchLogEvent {
  instanceId: string;
  line: string;
}

// --- Accounts / Microsoft auth ---

export interface AccountInfo {
  id: string;
  username: string;
  uuid: string;
  kind: "microsoft";
}

export interface AccountsState {
  accounts: AccountInfo[];
  activeId: string | null;
  microsoftConfigured: boolean;
}

export interface DeviceCodeEvent {
  userCode: string;
  verificationUri: string;
  message: string;
  expiresIn: number;
}

export interface MinecraftVersion {
  id: string;
  type: "release" | "snapshot" | "old_beta" | "old_alpha";
  url: string;
  releaseTime: string;
}

export interface VersionList {
  latestRelease: string;
  latestSnapshot: string;
  versions: MinecraftVersion[];
}

export interface LoaderVersion {
  version: string;
  stable: boolean;
}

/** Loaders with working install support today. */
export const SUPPORTED_LOADERS: ModLoader[] = ["fabric", "quilt"];
