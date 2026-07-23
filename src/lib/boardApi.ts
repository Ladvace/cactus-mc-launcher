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

let BASE = "";
let ready: Promise<void> | null = null;

export function initBoardApi(): Promise<void> {
  return (ready ??= invoke<string>("backend_base")
    .then((base) => {
      BASE = (base ?? "").replace(/\/$/, "");
    })
    .catch(() => {}));
}

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

// The board session token expires server-side. When a call is rejected for that
// reason we silently re-mint a fresh session (board_login uses the Minecraft
// access token, so no user interaction) and retry once. The store registers
// this to keep its reactive session in sync.
let onSessionRefresh: ((session: BoardSession) => void) | null = null;
export function onBoardSessionRefresh(cb: (session: BoardSession) => void): void {
  onSessionRefresh = cb;
}

function doAuthedFetch(path: string, token: string, init: RequestInit): Promise<Response> {
  return fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init.headers ?? {}),
      Authorization: `Bearer ${token}`,
    },
  });
}

async function looksExpired(res: Response): Promise<boolean> {
  if (res.status === 401 || res.status === 403) return true;
  const body = await res.clone().json().catch(() => ({}));
  return /expired|invalid.*session/i.test((body as any)?.error ?? "");
}

async function authed<T>(
  path: string,
  token: string,
  init: RequestInit = {}
): Promise<T> {
  let res = await doAuthedFetch(path, token, init);
  if (!res.ok && (await looksExpired(res))) {
    try {
      const fresh = await invoke<BoardSession>("board_login", { apiBase: BASE });
      onSessionRefresh?.(fresh);
      res = await doAuthedFetch(path, fresh.token, init);
    } catch {
      // Re-login failed (e.g. signed-out MC account) — surface the original error.
    }
  }
  await ensureOk(res);
  return (res.status === 204 ? undefined : await res.json()) as T;
}

export const boardApi = {
  configured: boardsConfigured,

  login: () =>
    invoke<BoardSession>("board_login", { apiBase: BASE }),

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

  async importSnapshot(id: string): Promise<ImportResult> {
    const res = await fetch(`${BASE}/v1/snapshots/${id}/blob`);
    if (!res.ok) throw new Error(`couldn't download snapshot (${res.status})`);
    const bytes = Array.from(new Uint8Array(await res.arrayBuffer()));
    return api.importSetup(bytes);
  },

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

  listPresence: (token: string) =>
    authed<{ players: PresencePlayer[] }>(`/v1/presence`, token).then((data) => data.players),
  setPresence: (
    token: string,
    body: { status: string; serverAddress: string; mcVersion: string; loader: string }
  ) => authed<void>(`/v1/presence`, token, { method: "PUT", body: JSON.stringify(body) }),
  clearPresence: (token: string) =>
    authed<void>(`/v1/presence`, token, { method: "DELETE" }),

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

  serverHistory: (address: string, hours = 24) =>
    get<{ samples: ServerSample[] }>(
      `/v1/servers/history?address=${encodeURIComponent(address)}&hours=${hours}`
    ).then((data) => data.samples),
};
