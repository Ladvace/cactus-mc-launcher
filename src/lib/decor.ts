// Sprites cropped from static/decor/desert-sheet.png by scripts/extract_sprites.py;
// re-run it to regenerate static/decor/sprites/NN.png after swapping the sheet.
const DECOR_IDS = [
  0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23,
  26, 30, 31, 32, 33, 34, 56, 57, 64,
];

export const DECOR_SPRITES: string[] = DECOR_IDS.map(
  (id) => `/decor/sprites/${String(id).padStart(2, "0")}.png`
);

export async function spriteToSquareIcon(url: string, size = 128): Promise<string> {
  const img = await loadImage(url);
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) return url;
  ctx.imageSmoothingEnabled = true;
  const scale = Math.min(size / img.width, size / img.height) * 0.92;
  const width = img.width * scale;
  const height = img.height * scale;
  ctx.drawImage(img, (size - width) / 2, (size - height) / 2, width, height);
  return canvas.toDataURL("image/png");
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = url;
  });
}
