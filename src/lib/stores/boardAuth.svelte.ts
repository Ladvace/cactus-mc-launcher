import { browser } from "$app/environment";
import { boardApi } from "$lib/boardApi";
import type { BoardSession } from "$lib/types";

const KEY = "cactus:boardSession";

function load(): BoardSession | null {
  if (!browser) return null;
  try {
    return JSON.parse(localStorage.getItem(KEY) || "null");
  } catch {
    return null;
  }
}

/// Boards-service session, authenticated with the player's Minecraft account.
class BoardAuth {
  session = $state<BoardSession | null>(load());
  loggingIn = $state(false);
  error = $state<string | null>(null);

  get token(): string | null {
    return this.session?.token ?? null;
  }
  get signedIn(): boolean {
    return !!this.session;
  }

  async login() {
    this.loggingIn = true;
    this.error = null;
    try {
      this.session = await boardApi.login();
      if (browser) localStorage.setItem(KEY, JSON.stringify(this.session));
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

export const boardAuth = new BoardAuth();
