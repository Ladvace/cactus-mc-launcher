// Client for the hosted Boards service (server/). Disabled until a base URL is
// configured via VITE_STREAMER_API_URL, so online features stay inert until the
// backend is deployed. Auth is the player's Minecraft account (via a Rust
// command that does the Mojang hasJoined handshake) — no Supabase/OAuth.

import { invoke } from "@tauri-apps/api/core";
import { api } from "$lib/api";
import type {
  Board,
  BoardCard,
  BoardSession,
  ImportResult,
  OwnedBoard,
  PresencePlayer,
  ServerSample,
  SnapshotManifest,
} from "$lib/types";

const env = (import.meta as any).env ?? {};
const BASE = (env.VITE_STREAMER_API_URL ?? "").replace(/\/$/, "");

export function boardsConfigured(): boolean {
  return BASE.length > 0;
}

async function ensureOk(res: Response): Promise<void> {
  if (res.ok) return;
  const body = await res.json().catch(() => ({}));
  throw new Error((body as any).error ?? `request failed (${res.status})`);
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  await ensureOk(res);
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
  await ensureOk(res);
  return (res.status === 204 ? undefined : await res.json()) as T;
}

export const boardApi = {
  configured: boardsConfigured,

  /** Sign in with the active Minecraft account; returns a session token. */
  login: () =>
    invoke<BoardSession>("board_login", { apiBase: BASE }),

  // --- Discovery / viewing (public) ---
  search: (query: string) =>
    get<{ results: BoardCard[] }>(
      `/v1/boards?q=${encodeURIComponent(query)}`
    ).then((data) => data.results),

  board: (handle: string) =>
    get<Board>(`/v1/boards/${encodeURIComponent(handle)}`),

  snapshot: (id: string) =>
    get<SnapshotManifest>(`/v1/snapshots/${encodeURIComponent(id)}`),

  resolveCode: (code: string) =>
    get<{ snapshotId: string }>(
      `/v1/codes/${encodeURIComponent(code.trim().toLowerCase())}`
    ),

  /** Download a hosted snapshot and import it, reusing the Rust importer. */
  async importSnapshot(id: string): Promise<ImportResult> {
    const res = await fetch(`${BASE}/v1/snapshots/${id}/blob`);
    if (!res.ok) throw new Error(`couldn't download snapshot (${res.status})`);
    const bytes = Array.from(new Uint8Array(await res.arrayBuffer()));
    return api.importSetup(bytes);
  },

  // --- Creator (authed) ---
  myBoards: (token: string) =>
    authed<{ boards: OwnedBoard[] }>(`/v1/boards/me`, token).then((data) => data.boards),

  createBoard: (
    token: string,
    body: {
      handle: string;
      displayName: string;
      kind: string;
      description?: string;
      streamUrl?: string;
      serverAddress?: string;
    }
  ) =>
    authed<BoardCard>(`/v1/boards`, token, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  updateBoard: (token: string, handle: string, patch: Record<string, unknown>) =>
    authed<void>(`/v1/boards/${encodeURIComponent(handle)}`, token, {
      method: "PATCH",
      body: JSON.stringify(patch),
    }),

  deleteBoard: (token: string, handle: string) =>
    authed<void>(`/v1/boards/${encodeURIComponent(handle)}`, token, {
      method: "DELETE",
    }),

  postMessage: (token: string, handle: string, body: string) =>
    authed<void>(`/v1/boards/${encodeURIComponent(handle)}/messages`, token, {
      method: "POST",
      body: JSON.stringify({ body }),
    }),

  deleteMessage: (token: string, handle: string, id: string) =>
    authed<void>(
      `/v1/boards/${encodeURIComponent(handle)}/messages/${encodeURIComponent(id)}`,
      token,
      { method: "DELETE" }
    ),

  mintCode: (token: string, snapshotId: string) =>
    authed<{ code: string }>(`/v1/codes`, token, {
      method: "POST",
      body: JSON.stringify({ snapshotId }),
    }),

  report: (token: string, targetHandle: string, reason: string) =>
    authed<void>(`/v1/reports`, token, {
      method: "POST",
      body: JSON.stringify({ targetHandle, reason }),
    }),

  // --- Presence (opt-in "who's online") ---
  listPresence: (token: string) =>
    authed<{ players: PresencePlayer[] }>(`/v1/presence`, token).then((data) => data.players),
  setPresence: (
    token: string,
    body: { status: string; serverAddress: string; mcVersion: string; loader: string }
  ) => authed<void>(`/v1/presence`, token, { method: "PUT", body: JSON.stringify(body) }),
  clearPresence: (token: string) =>
    authed<void>(`/v1/presence`, token, { method: "DELETE" }),

  /** Export an instance and publish it (to a board, or standalone). Returns id. */
  publish: (
    instanceId: string,
    format: "cactuspack" | "mrpack",
    token: string,
    opts: { boardHandle?: string; name?: string; changelog?: string } = {}
  ) =>
    invoke<string>("publish_setup", {
      instanceId,
      format,
      apiBase: BASE,
      accessToken: token,
      boardHandle: opts.boardHandle ?? null,
      name: opts.name ?? null,
      changelog: opts.changelog?.trim() || null,
    }),

  /** Player-count history for a server board's address (default last 24h). */
  serverHistory: (address: string, hours = 24) =>
    get<{ samples: ServerSample[] }>(
      `/v1/servers/history?address=${encodeURIComponent(address)}&hours=${hours}`
    ).then((data) => data.samples),
};
