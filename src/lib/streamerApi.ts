// Client for the hosted Streamer API (server/). Disabled until a base URL is
// configured via VITE_STREAMER_API_URL, so the online features stay inert until
// the backend is deployed — the offline .drakepack/.mrpack import always works.

import { invoke } from "@tauri-apps/api/core";
import { api } from "$lib/api";
import type {
  ImportResult,
  OwnedProfile,
  SnapshotManifest,
  StreamerCard,
  StreamerProfile,
} from "$lib/types";

const env = (import.meta as any).env ?? {};
const BASE = (env.VITE_STREAMER_API_URL ?? "").replace(/\/$/, "");
export const SUPABASE_URL = (env.VITE_SUPABASE_URL ?? "").replace(/\/$/, "");
export const SUPABASE_ANON_KEY = env.VITE_SUPABASE_ANON_KEY ?? "";

export function streamerApiConfigured(): boolean {
  return BASE.length > 0;
}
export function supabaseConfigured(): boolean {
  return SUPABASE_URL.length > 0 && SUPABASE_ANON_KEY.length > 0;
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as any).error ?? `request failed (${res.status})`);
  }
  return res.json() as Promise<T>;
}

async function authed<T>(
  path: string,
  token: string,
  init: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init.headers ?? {}),
      Authorization: `Bearer ${token}`,
    },
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as any).error ?? `request failed (${res.status})`);
  }
  return (res.status === 204 ? undefined : await res.json()) as T;
}

export const streamerApi = {
  configured: streamerApiConfigured,

  search: (q: string) =>
    get<{ results: StreamerCard[] }>(
      `/v1/streamers?q=${encodeURIComponent(q)}`
    ).then((r) => r.results),

  profile: (handle: string) =>
    get<StreamerProfile>(`/v1/streamers/${encodeURIComponent(handle)}`),

  live: (handle: string) =>
    get<{ isLive: boolean; currentActivity: string | null }>(
      `/v1/streamers/${encodeURIComponent(handle)}/live`
    ),

  resolveCode: (code: string) =>
    get<{ snapshotId: string; streamerHandle: string | null }>(
      `/v1/codes/${encodeURIComponent(code.trim().toLowerCase())}`
    ),

  snapshot: (id: string) =>
    get<SnapshotManifest>(`/v1/snapshots/${encodeURIComponent(id)}`),

  // --- Authenticated (streamer) ---
  me: (token: string) =>
    authed<{ profile: OwnedProfile | null }>(`/v1/me`, token),

  claimHandle: (token: string, handle: string, displayName: string) =>
    authed<OwnedProfile>(`/v1/streamers`, token, {
      method: "POST",
      body: JSON.stringify({ handle, displayName }),
    }),

  mintCode: (token: string) =>
    authed<{ code: string }>(`/v1/codes`, token, { method: "POST", body: "{}" }),

  report: (token: string, targetHandle: string, reason: string) =>
    authed<void>(`/v1/reports`, token, {
      method: "POST",
      body: JSON.stringify({ targetHandle, reason }),
    }),

  /** Export an instance and publish it as the current snapshot (via Rust). */
  publish: (
    instanceId: string,
    format: "drakepack" | "mrpack",
    token: string,
    changelog?: string
  ) =>
    invoke<string>("publish_setup", {
      instanceId,
      format,
      apiBase: BASE,
      accessToken: token,
      changelog: changelog?.trim() || null,
    }),

  /** Download a hosted snapshot blob and import it as a new instance, reusing
      the same Rust importer the offline flow uses. */
  async importSnapshot(id: string): Promise<ImportResult> {
    const manifest = await this.snapshot(id);
    const url = manifest.downloadUrl.startsWith("http")
      ? manifest.downloadUrl
      : `${BASE}${manifest.downloadUrl}`;
    const res = await fetch(url);
    if (!res.ok) throw new Error(`couldn't download snapshot (${res.status})`);
    const bytes = Array.from(new Uint8Array(await res.arrayBuffer()));
    return api.importSetup(bytes);
  },
};
