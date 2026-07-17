// Shared localStorage helpers used by the runes stores. Every access is guarded
// by `browser` so SSR/prerender (where localStorage is absent) is a no-op.
import { browser } from "$app/environment";

/** Parse a JSON value from localStorage, falling back on missing/corrupt data. */
export function readJson<T>(key: string, fallback: T): T {
  if (!browser) return fallback;
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? fallback : ((JSON.parse(raw) ?? fallback) as T);
  } catch {
    return fallback;
  }
}

export function writeJson(key: string, value: unknown): void {
  if (browser) localStorage.setItem(key, JSON.stringify(value));
}

export function removeJson(key: string): void {
  if (browser) localStorage.removeItem(key);
}
