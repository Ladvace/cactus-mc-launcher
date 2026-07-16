// Helpers for turning user input into compact instance-icon data URIs.

/** Read a picked image file into a data URI. GIFs are kept raw so they keep
    animating; everything else is cover-cropped to a small square thumbnail to
    keep the stored instance JSON light. */
export async function fileToIconDataUri(file: File, size = 256): Promise<string> {
  const raw = await readAsDataURL(file);
  if (file.type === "image/gif") return raw; // preserve animation
  return await downscaleSquare(raw, size);
}

function readAsDataURL(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result as string);
    r.onerror = () => reject(r.error ?? new Error("read failed"));
    r.readAsDataURL(file);
  });
}

function downscaleSquare(src: string, size: number): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext("2d");
      if (!ctx) return reject(new Error("no 2d context"));
      // Cover-fit: scale so the image fills the square, centered.
      const scale = Math.max(size / img.width, size / img.height);
      const w = img.width * scale;
      const h = img.height * scale;
      ctx.imageSmoothingQuality = "high";
      ctx.drawImage(img, (size - w) / 2, (size - h) / 2, w, h);
      resolve(canvas.toDataURL("image/webp", 0.85));
    };
    img.onerror = () => reject(new Error("could not load image"));
    img.src = src;
  });
}

/** Read a picked image into a data URI for use as an app background: GIFs kept
    raw (to animate), other formats scaled to fit within `max` on the long edge,
    preserving aspect ratio. */
export async function fileToBackgroundDataUri(file: File, max = 1366): Promise<string> {
  const raw = await readAsDataURL(file);
  if (file.type === "image/gif") return raw;
  return await downscaleFit(raw, max);
}

function downscaleFit(src: string, max: number): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const scale = Math.min(1, max / Math.max(img.width, img.height));
      const w = Math.round(img.width * scale);
      const h = Math.round(img.height * scale);
      const canvas = document.createElement("canvas");
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext("2d");
      if (!ctx) return reject(new Error("no 2d context"));
      ctx.imageSmoothingQuality = "high";
      ctx.drawImage(img, 0, 0, w, h);
      resolve(canvas.toDataURL("image/webp", 0.85));
    };
    img.onerror = () => reject(new Error("could not load image"));
    img.src = src;
  });
}

/** Render an emoji to a PNG data URI. Canvas text uses the system colour-emoji
    font, so this works reliably as an <img> source. */
export function emojiToDataUri(emoji: string, size = 128): string {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) return "";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.font = `${Math.floor(size * 0.7)}px "Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", sans-serif`;
  ctx.fillText(emoji, size / 2, size / 2 + size * 0.04);
  return canvas.toDataURL("image/png");
}
