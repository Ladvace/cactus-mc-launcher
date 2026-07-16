import { invoke } from "@tauri-apps/api/core";
import { browser } from "$app/environment";
import { SUPABASE_URL, SUPABASE_ANON_KEY } from "$lib/streamerApi";
import type { StreamerSession } from "$lib/types";

const KEY = "drake:streamerSession";

function load(): StreamerSession | null {
  if (!browser) return null;
  try {
    return JSON.parse(localStorage.getItem(KEY) || "null");
  } catch {
    return null;
  }
}

/// Streamer (creator) session for the hosted service. Separate from the
/// Microsoft/game account — this authorizes publishing a profile & snapshots.
class StreamerAuth {
  session = $state<StreamerSession | null>(load());
  loggingIn = $state(false);
  error = $state<string | null>(null);

  get token(): string | null {
    return this.session?.accessToken ?? null;
  }
  get signedIn(): boolean {
    return !!this.session && this.session.expiresAt * 1000 > Date.now();
  }

  async login(provider: "twitch" | "google") {
    this.loggingIn = true;
    this.error = null;
    try {
      const session = await invoke<StreamerSession>("streamer_login", {
        supabaseUrl: SUPABASE_URL,
        anonKey: SUPABASE_ANON_KEY,
        provider,
      });
      this.session = session;
      if (browser) localStorage.setItem(KEY, JSON.stringify(session));
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loggingIn = false;
    }
  }

  logout() {
    this.session = null;
    if (browser) localStorage.removeItem(KEY);
  }
}

export const streamerAuth = new StreamerAuth();
