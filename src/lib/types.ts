export type ModLoader = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

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
  created: string;
  lastPlayed: string | null;
  totalPlaytimeSeconds: number;
  coverImage: boolean;
  serverMemoryMb: number | null;
  maxMemoryMb: number | null;
  minMemoryMb: number | null;
  jvmArgs: string | null;
  javaPath: string | null;
  gameWidth: number | null;
  gameHeight: number | null;
  gameDir: string | null;
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
  serverMemoryMb?: number;
  maxMemoryMb?: number;
  minMemoryMb?: number;
  jvmArgs?: string;
  javaPath?: string;
  gameWidth?: number;
  gameHeight?: number;
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

export interface DetectedLauncher {
  id: string;
  name: string;
  instanceCount: number;
  canDelete: boolean;
  icon: string | null;
}

export interface ExternalInstance {
  launcher: string;
  name: string;
  mcVersion: string;
  loader: string;
  loaderVersion: string | null;
  gameDir: string;
  sourceDir: string | null;
  icon: string | null;
}

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

export interface ServiceStatus {
  id: string;
  name: string;
  up: boolean | null;
  latencyMs: number | null;
}

export interface Settings {
  theme: string;
  javaPath: string | null;
  javaPaths: Record<string, string>;
  maxMemoryMb: number;
  minMemoryMb: number;
  maxConcurrentDownloads: number;
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
  instancesDir: string;
  ngrokAuthtoken: string;
  showNews: boolean;
  newsSingle: boolean;
  dateFormat: string;
  language: string;
  accent: string;
  uiScale: number;
  soundVolume: number;
  defaultLoader: string;
  reduceMotion: boolean;
  readableFont: boolean;
  highContrast: boolean;
  reduceTransparency: boolean;
  alwaysShowFocus: boolean;
  menuBranding: boolean;
  openInstanceOnInstall: boolean;
}

export type DockPosition = "bottom" | "top" | "left" | "right";

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
  categories?: string[];
  environment?: string | null;
  openSource?: boolean;
  offset?: number;
  limit?: number;
}

export interface ContentCategory {
  name: string;
  projectType: string;
  header: string;
  icon: string;
}

export interface BrowseServer {
  name: string;
  address: string;
  online: boolean;
  players: number;
  maxPlayers: number;
  version: string;
  description: string;
  votes: number;
  rating: number;
  country: string;
  favicon: string | null;
}

export interface BrowseServersResult {
  servers: BrowseServer[];
  page: number;
  hasMore: boolean;
}

export interface BrowseServersParams {
  query?: string;
  sort?: string | null;
  page?: number;
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

export interface ContentUpdate {
  versionId: string;
  projectId: string;
  projectType: string;
  source: string;
  title: string;
  iconUrl: string | null;
  latestVersionId: string;
  latestNumber: string;
}

export interface RestorePoint {
  id: string;
  created: string;
  label: string;
  auto: boolean;
  contentCount: number;
}

export interface ApplyUpdatesResult {
  updated: number;
  skipped: string[];
  restorePointId: string;
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

export interface ServerStatus {
  online: number;
  max: number;
  motd: string;
  players: string[];
  pingMs: number;
  favicon?: string | null;
}

export interface ServerSample {
  sampledAt: string;
  online: number;
  max: number;
}

export const SUPPORTED_LOADERS: ModLoader[] = [
  "fabric",
  "quilt",
  "forge",
  "neoforge",
];

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
  lowRam: boolean;
  mods: ModRec[];
  unavailable: string[];
}

export interface SelectedMod {
  versionId: string;
  title: string;
}

export interface Friend {
  profileId: string;
  name: string;
}

export interface FriendsList {
  friends: Friend[];
  incoming: Friend[];
  outgoing: Friend[];
  empty: boolean;
}

export interface FriendsPrefs {
  friendsEnabled: boolean;
  acceptInvites: boolean;
}

export interface NewsItem {
  id: string;
  title: string;
  summary: string;
  category: string;
  date: string;
  image: string | null;
  link: string | null;
  source: string;
}

export interface AchievementsPayload {
  player: { name: string; worldsScanned: number; instancesScanned: number };
  hasData: boolean;
  completion: { earned: number; total: number; percent: number };
  categories: { key: string; earned: number; total: number }[];
  advancements: AdvancementView[];
  custom: CustomAchievement[];
  stats: LifetimeStats;
  instances: InstanceBreakdown[];
}

export interface AdvancementView {
  id: string;
  name: string;
  category: string;
  done: boolean;
  hidden: boolean;
  earnedAt: string | null;
  earnedIn: string | null;
}

export interface CustomAchievement {
  id: string;
  name: string;
  description: string;
  icon: string;
  earned: boolean;
  progress: number;
}

export interface StatEntry {
  key: string;
  count: number;
}

export interface LifetimeStats {
  playTimeTicks: number;
  blocksMined: number;
  mobsKilled: number;
  deaths: number;
  distanceCm: number;
  jumps: number;
  itemsPickedUp: number;
  damageDealt: number;
  timesSlept: number;
  topMined: StatEntry[];
  topKilled: StatEntry[];
}

export interface InstanceBreakdown {
  id: string;
  name: string;
  worlds: number;
  playTimeTicks: number;
  blocksMined: number;
  mobsKilled: number;
  deaths: number;
}

export interface TuneupSelection {
  mods: SelectedMod[];
  applyMemory: boolean;
  applyFlags: boolean;
  maxMemMb: number;
  minMemMb: number;
  jvmArgs: string;
}
