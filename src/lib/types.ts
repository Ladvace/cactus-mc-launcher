// Mirrors the Rust types in `src-tauri/src`. Keep these in sync.

export type ModLoader = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

/// A normal game client or a dedicated server.
export type InstanceKind = "client" | "server";

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
  kind: InstanceKind;
  icon: string | null;
  mcVersion: string;
  loader: ModLoader;
  loaderVersion: string | null;
  group: string | null;
  created: string; // ISO 8601
  lastPlayed: string | null;
  totalPlaytimeSeconds: number;
  coverImage: boolean;
  /** Legacy server max heap (MB); superseded by maxMemoryMb. */
  serverMemoryMb: number | null;
  // Per-instance overrides (null = use the global setting).
  maxMemoryMb: number | null;
  minMemoryMb: number | null;
  jvmArgs: string | null;
  javaPath: string | null;
  gameWidth: number | null;
  gameHeight: number | null;
  /** Absolute game-data folder; null = default under the instances folder. */
  gameDir: string | null;
  /** Per-instance ngrok authtoken (overrides the global one); null = use global. */
  ngrokAuthtoken: string | null;
}

export interface CreateInstance {
  name: string;
  kind?: InstanceKind;
  mcVersion: string;
  loader?: ModLoader;
  loaderVersion?: string | null;
  icon?: string | null;
}

export interface OpEntry {
  uuid: string;
  name: string;
  level: number;
  bypassesPlayerLimit: boolean;
}

export interface PlayerEntry {
  uuid: string;
  name: string;
}

export interface PresencePlayer {
  uuid: string;
  name: string;
  status: string;
  serverAddress: string | null;
  mcVersion: string | null;
  loader: string | null;
  updatedAt: string;
}

export interface WorldInfo {
  name: string;
  folder: string;
  path: string;
  sizeBytes: number;
  lastModified: string | null;
  location: "saves" | "server";
}

export interface UpdateInstance {
  name?: string;
  icon?: string;
  group?: string;
  mcVersion?: string;
  loader?: ModLoader;
  loaderVersion?: string;
  coverImage?: boolean;
  /** Max heap (MB) for a server; 0 clears the override. */
  serverMemoryMb?: number;
  // Per-instance overrides. 0 (numbers) / "" (strings) clears back to global.
  maxMemoryMb?: number;
  minMemoryMb?: number;
  jvmArgs?: string;
  javaPath?: string;
  gameWidth?: number;
  gameHeight?: number;
  /** Per-instance ngrok authtoken; "" clears it (use the global one). */
  ngrokAuthtoken?: string;
}

export interface Sticker {
  id: string;
  preview: string;
  full: string;
}

export interface ImportResult {
  instance: Instance;
  installed: number;
  skipped: string[];
}

export interface ExportResult {
  path: string;
  skipped: string[];
}

// --- Boards service DTOs (mirror server/src/types.ts) ---
export type BoardKind = "streamer" | "creator" | "server";

export interface BoardSession {
  token: string;
  uuid: string;
  name: string;
}

export interface BoardCard {
  handle: string;
  displayName: string;
  kind: BoardKind;
  ownerName: string;
}

export interface BoardInstance {
  id: string;
  name: string;
  format: "cactuspack" | "mrpack";
  mcVersion: string | null;
  modLoader: string | null;
  changelog: string | null;
  createdAt: string;
}

export interface BoardMessage {
  id: string;
  body: string;
  createdAt: string;
}

export interface OwnedBoard extends BoardCard {
  description: string | null;
  streamUrl: string | null;
  serverAddress: string | null;
  isPublic: boolean;
  messages: BoardMessage[];
}

export interface Board extends BoardCard {
  description: string | null;
  streamUrl: string | null;
  serverAddress: string | null;
  isPublic: boolean;
  isOwner: boolean;
  instances: BoardInstance[];
  messages: BoardMessage[];
}

export interface SnapshotManifest {
  id: string;
  name: string;
  format: "cactuspack" | "mrpack";
  mcVersion: string | null;
  modLoader: string | null;
  changelog: string | null;
  downloadUrl: string;
  createdAt: string;
}

export interface CacheStats {
  files: number;
  bytes: number;
  linkedBytes: number;
  savedBytes: number;
}

export interface Settings {
  theme: string;
  javaPath: string | null;
  /** Per-major Java executables (e.g. "8"/"17"/"21" → path); overrides javaPath. */
  javaPaths: Record<string, string>;
  maxMemoryMb: number;
  minMemoryMb: number;
  jvmArgs: string;
  gameWidth: number;
  gameHeight: number;
  offlineUsername: string;
  background: string;
  uiSounds: boolean;
  giphyApiKey: string;
  dockPosition: DockPosition;
  decorTheme: string;
  dockMagnify: boolean;
  /** Default parent folder for new instances' data. Empty = app default. */
  instancesDir: string;
  /** Global ngrok authtoken for sharing servers; a per-instance token wins. */
  ngrokAuthtoken: string;
}

export type DockPosition = "bottom" | "top" | "left" | "right";

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

// --- Content sources ---

export type Source = "modrinth" | "curseforge";

export const SOURCES: { value: Source; label: string; enabled: boolean }[] = [
  { value: "modrinth", label: "Modrinth", enabled: true },
  { value: "curseforge", label: "CurseForge", enabled: false },
];

export type ProjectType =
  | "mod"
  | "modpack"
  | "resourcepack"
  | "shader"
  | "datapack";

export interface SearchHit {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  follows: number;
  iconUrl: string | null;
  categories: string[];
  versions: string[];
  projectType: string;
  source: string;
}

export interface SearchResults {
  hits: SearchHit[];
  totalHits: number;
  offset: number;
  limit: number;
}

export interface SearchParams {
  query: string;
  projectType: ProjectType;
  gameVersion?: string | null;
  loader?: string | null;
  sort?: string | null;
  offset?: number;
  limit?: number;
}

export interface ModrinthVersionFile {
  url: string;
  filename: string;
  primary: boolean;
  size: number;
  hashes: { sha1: string | null; sha512: string | null };
}

export interface ModrinthVersion {
  id: string;
  projectId: string;
  name: string;
  versionNumber: string;
  versionType: string;
  gameVersions: string[];
  loaders: string[];
  files: ModrinthVersionFile[];
  datePublished: string;
  downloads: number;
}

export interface ContentItem {
  projectId: string | null;
  versionId: string;
  projectType: string;
  title: string;
  fileName: string;
  iconUrl: string | null;
  enabled: boolean;
  source: string;
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

/** Live Server List Ping result (mirrors Rust `server_ping::ServerStatus`). */
export interface ServerStatus {
  online: number;
  max: number;
  motd: string;
  players: string[];
  pingMs: number;
  /** Server-broadcast icon as a data: URI, if any. */
  favicon?: string | null;
  /** Version name the server reports (e.g. "1.20.4"); a hint, not a guarantee. */
  version: string;
  /** Required mod ids from a Forge/NeoForge ping; empty for vanilla/Fabric. */
  mods: string[];
}

/** One point in a server's player-count history (from the boards backend). */
export interface ServerSample {
  sampledAt: string;
  online: number;
  max: number;
}

/** Loaders with working install support today. */
export const SUPPORTED_LOADERS: ModLoader[] = [
  "fabric",
  "quilt",
  "forge",
  "neoforge",
];

// --- Adaptive tune-up -------------------------------------------------------

export interface HostSpecs {
  totalRamMb: number;
  cpuCores: number;
  os: string;
  arch: string;
}

export interface ModRec {
  slug: string;
  versionId: string;
  title: string;
  reason: string;
  recommended: boolean;
  installed: boolean;
}

export interface TuneupPlan {
  specs: HostSpecs;
  mcVersion: string;
  loader: string;
  maxMemMb: number;
  minMemMb: number;
  jvmArgs: string;
  mods: ModRec[];
  unavailable: string[];
}

export interface SelectedMod {
  versionId: string;
  title: string;
}

export interface TuneupSelection {
  mods: SelectedMod[];
  applyMemory: boolean;
  applyFlags: boolean;
  maxMemMb: number;
  minMemMb: number;
  jvmArgs: string;
}
