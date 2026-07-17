// Remembers which hosted snapshot an instance was imported from, so the viewer
// can be told "the streamer updated their setup" instead of silently drifting.
import { readJson, writeJson } from "$lib/storage";

const KEY = "cactus:importedFrom";

export interface ImportRecord {
  handle: string | null;
  snapshotId: string;
  importedAt: number; // ms
}

function all(): Record<string, ImportRecord> {
  return readJson<Record<string, ImportRecord>>(KEY, {});
}

export function recordImport(instanceId: string, rec: ImportRecord) {
  const map = all();
  map[instanceId] = rec;
  writeJson(KEY, map);
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
