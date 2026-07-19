export async function fileToIconDataUri(file: File, size = 256): Promise<string> {
  const raw = await readAsDataURL(file);
  if (file.type === "image/gif") return raw;
  return await downscaleSquare(raw, size);
}

function readAsDataURL(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error ?? new Error("read failed"));
    reader.readAsDataURL(file);
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
      const scale = Math.max(size / img.width, size / img.height);
      const width = img.width * scale;
      const height = img.height * scale;
      ctx.imageSmoothingQuality = "high";
      ctx.drawImage(img, (size - width) / 2, (size - height) / 2, width, height);
      resolve(canvas.toDataURL("image/webp", 0.85));
    };
    img.onerror = () => reject(new Error("could not load image"));
    img.src = src;
  });
}

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
      const width = Math.round(img.width * scale);
      const height = Math.round(img.height * scale);
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const ctx = canvas.getContext("2d");
      if (!ctx) return reject(new Error("no 2d context"));
      ctx.imageSmoothingQuality = "high";
      ctx.drawImage(img, 0, 0, width, height);
      resolve(canvas.toDataURL("image/webp", 0.85));
    };
    img.onerror = () => reject(new Error("could not load image"));
    img.src = src;
  });
}

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
