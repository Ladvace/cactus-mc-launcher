import { readJson, writeJson } from "$lib/storage";

const KEY = "cactus:skinCache";

type Cached = { full: string; face: string; body: string };

class SkinCache {
  #map = $state<Record<string, Cached>>(readJson(KEY, {}));

  getFull(uuid: string): string | null {
    return this.#map[uuid]?.full ?? null;
  }
  getFace(uuid: string): string | null {
    return this.#map[uuid]?.face ?? null;
  }
  getBody(uuid: string): string | null {
    return this.#map[uuid]?.body ?? null;
  }

  setSkin(uuid: string, fullDataUri: string) {
    renderViews(fullDataUri).then((views) => {
      this.#map = {
        ...this.#map,
        [uuid]: { full: fullDataUri, face: views?.face ?? fullDataUri, body: views?.body ?? fullDataUri },
      };
      writeJson(KEY, this.#map);
    });
  }

  clear(uuid: string) {
    if (!this.#map[uuid]) return;
    const next = { ...this.#map };
    delete next[uuid];
    this.#map = next;
    writeJson(KEY, next);
  }
}

const FRONT: [number, number, number, number, number, number][] = [
  [8, 8, 8, 8, 4, 0], [40, 8, 8, 8, 4, 0],
  [20, 20, 8, 12, 4, 8], [20, 36, 8, 12, 4, 8],
  [44, 20, 4, 12, 0, 8], [44, 36, 4, 12, 0, 8],
  [36, 52, 4, 12, 12, 8], [52, 52, 4, 12, 12, 8],
  [4, 20, 4, 12, 4, 20], [4, 36, 4, 12, 4, 20],
  [20, 52, 4, 12, 8, 20], [4, 52, 4, 12, 8, 20],
];

function renderViews(fullDataUri: string): Promise<{ face: string; body: string } | null> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      try {
        resolve({ face: renderFace(img), body: renderBody(img) });
      } catch {
        resolve(null);
      }
    };
    img.onerror = () => resolve(null);
    img.src = fullDataUri;
  });
}

function renderFace(img: HTMLImageElement): string {
  const scale = 8;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 8 * scale;
  const ctx = canvas.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 8 * scale, 8 * scale);
  ctx.drawImage(img, 40, 8, 8, 8, 0, 0, 8 * scale, 8 * scale);
  return canvas.toDataURL("image/png");
}

function renderBody(img: HTMLImageElement): string {
  const scale = 8;
  const canvas = document.createElement("canvas");
  canvas.width = 16 * scale;
  canvas.height = 32 * scale;
  const ctx = canvas.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  for (const [sx, sy, w, h, dx, dy] of FRONT) {
    ctx.drawImage(img, sx, sy, w, h, dx * scale, dy * scale, w * scale, h * scale);
  }
  return canvas.toDataURL("image/png");
}

export const skinCache = new SkinCache();
