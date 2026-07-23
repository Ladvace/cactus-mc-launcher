import { boardApi, onBoardSessionRefresh } from "$lib/boardApi";
import { readJson, removeJson, writeJson } from "$lib/storage";
import type { BoardSession } from "$lib/types";

const KEY = "cactus:boardSession";

function load(): BoardSession | null {
  return readJson<BoardSession | null>(KEY, null);
}

class BoardAuth {
  session = $state<BoardSession | null>(load());
  loggingIn = $state(false);
  error = $state<string | null>(null);

  constructor() {
    // Adopt any session the API layer silently re-mints on token expiry.
    onBoardSessionRefresh((session) => {
      this.session = session;
      writeJson(KEY, session);
    });
  }

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
    } catch (error) {
      this.error = String(error);
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
