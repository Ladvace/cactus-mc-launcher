import { boardApi } from "$lib/boardApi";
import { readJson, removeJson, writeJson } from "$lib/storage";
import type { BoardSession } from "$lib/types";

const KEY = "cactus:boardSession";

function load(): BoardSession | null {
  return readJson<BoardSession | null>(KEY, null);
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
      writeJson(KEY, this.session);
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loggingIn = false;
    }
  }

  logout() {
    this.session = null;
    removeJson(KEY);
  }
}

export const boardAuth = new BoardAuth();
