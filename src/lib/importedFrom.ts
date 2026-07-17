// Remembers which hosted snapshot an instance was imported from, so the viewer
// can be told "the streamer updated their setup" instead of silently drifting.
import { browser } from "$app/environment";

const KEY = "cactus:importedFrom";

export interface ImportRecord {
  handle: string | null;
  snapshotId: string;
  importedAt: number; // ms
}

function all(): Record<string, ImportRecord> {
  if (!browser) return {};
  try {
    return JSON.parse(localStorage.getItem(KEY) || "{}") ?? {};
  } catch {
    return {};
  }
}

export function recordImport(instanceId: string, rec: ImportRecord) {
  if (!browser) return;
  const map = all();
  map[instanceId] = rec;
  localStorage.setItem(KEY, JSON.stringify(map));
}

/** The instance (if any) this viewer imported from a given streamer handle. */
export function importForHandle(
  handle: string
): { instanceId: string; rec: ImportRecord } | null {
  for (const [instanceId, rec] of Object.entries(all())) {
    if (rec.handle === handle) return { instanceId, rec };
  }
  return null;
}
