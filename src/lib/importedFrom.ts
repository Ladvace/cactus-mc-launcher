// Remembers which hosted snapshot an instance was imported from, so the viewer
// can be told "the streamer updated their setup" instead of silently drifting.
import { readJson, writeJson } from "$lib/storage";

const KEY = "cactus:importedFrom";

interface ImportRecord {
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
