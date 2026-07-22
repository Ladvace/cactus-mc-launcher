<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { skinCache } from "$lib/stores/skins.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { t, type MessageKey } from "$lib/i18n";

  interface Props {
    open: boolean;
    onClose: () => void;
    uuid: string;
    variant: "classic" | "slim";

    onApplied?: (dataUri: string) => void;
  }
  let { open, onClose, uuid, variant, onApplied }: Props = $props();

  const SIZE = 64;
  const SCALE = 8;
  const VIEW = SIZE * SCALE;

  const GUIDES: [number, number, number, number][] = [
    [0, 0, 32, 16], [32, 0, 32, 16],
    [16, 16, 24, 16], [16, 32, 24, 16],
    [0, 16, 16, 16], [0, 32, 16, 16],
    [40, 16, 16, 16], [40, 32, 16, 16],
    [16, 48, 16, 16], [0, 48, 16, 16],
    [32, 48, 16, 16], [48, 48, 16, 16],
  ];

  const FRONT: [number, number, number, number, number, number][] = [
    [8, 8, 8, 8, 4, 0], [40, 8, 8, 8, 4, 0],
    [20, 20, 8, 12, 4, 8], [20, 36, 8, 12, 4, 8],
    [44, 20, 4, 12, 0, 8], [44, 36, 4, 12, 0, 8],
    [36, 52, 4, 12, 12, 8], [52, 52, 4, 12, 12, 8],
    [4, 20, 4, 12, 4, 20], [4, 36, 4, 12, 4, 20],
    [20, 52, 4, 12, 8, 20], [4, 52, 4, 12, 8, 20],
  ];
  // Back faces (arms/legs mirrored horizontally since we view from behind).
  const BACK: [number, number, number, number, number, number][] = [
    [24, 8, 8, 8, 4, 0], [56, 8, 8, 8, 4, 0],
    [32, 20, 8, 12, 4, 8], [32, 36, 8, 12, 4, 8],
    [52, 20, 4, 12, 12, 8], [52, 36, 4, 12, 12, 8],
    [44, 52, 4, 12, 0, 8], [60, 52, 4, 12, 0, 8],
    [12, 20, 4, 12, 8, 20], [12, 36, 4, 12, 8, 20],
    [28, 52, 4, 12, 4, 20], [12, 52, 4, 12, 4, 20],
  ];

  const PALETTE = [
    "#000000", "#3d3a34", "#7f7970", "#c9c4ba", "#ffffff",
    "#b1836a", "#8a5a3b", "#e8b23a", "#d9e746", "#57c84a",
    "#3aa0e8", "#3a4ae8", "#9b4ae8", "#e84a8a", "#e84a4a",
  ];

  type Tool = "draw" | "erase" | "fill" | "pick";
  const TOOLS: [Tool, MessageKey][] = [
    ["draw", "skinEditor.draw"],
    ["erase", "skinEditor.erase"],
    ["fill", "skinEditor.fill"],
    ["pick", "skinEditor.pick"],
  ];

  let tool = $state<Tool>("draw");
  let color = $state("#57c84a");
  let showGrid = $state(true);
  let loading = $state(true);
  let applying = $state(false);

  let editCanvas: HTMLCanvasElement;
  let viewCanvas = $state<HTMLCanvasElement>();
  let previewCanvas = $state<HTMLCanvasElement>();
  let previewBackCanvas = $state<HTMLCanvasElement>();
  let ectx: CanvasRenderingContext2D;

  let undoStack: ImageData[] = [];
  let redoStack: ImageData[] = [];
  let canUndo = $state(false);
  let canRedo = $state(false);

  let painting = false;
  let last: { x: number; y: number } | null = null;

  $effect(() => {
    if (!open || !viewCanvas) return;
    editCanvas = document.createElement("canvas");
    editCanvas.width = SIZE;
    editCanvas.height = SIZE;
    ectx = editCanvas.getContext("2d", { willReadFrequently: true })!;
    ectx.imageSmoothingEnabled = false;
    undoStack = [];
    redoStack = [];
    canUndo = canRedo = false;
    loadSkin();
  });

  async function loadSkin() {
    loading = true;
    try {

      const dataUri = skinCache.getFull(uuid) ?? (await api.downloadImage(`https://minotar.net/skin/${uuid}`));
      await drawSourceImage(dataUri);
    } catch {
      ectx.clearRect(0, 0, SIZE, SIZE);
    } finally {
      loading = false;
      render();
    }
  }

  function drawSourceImage(src: string): Promise<void> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        ectx.clearRect(0, 0, SIZE, SIZE);

        ectx.drawImage(img, 0, 0);
        resolve();
      };
      img.onerror = () => resolve();
      img.src = src;
    });
  }

  function render() {
    const ctx = viewCanvas?.getContext("2d");
    if (!ctx || !editCanvas) return;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, VIEW, VIEW);

    for (let y = 0; y < SIZE; y++)
      for (let x = 0; x < SIZE; x++) {
        if ((x + y) % 2 === 0) continue;
        ctx.fillStyle = "rgba(255,255,255,0.04)";
        ctx.fillRect(x * SCALE, y * SCALE, SCALE, SCALE);
      }
    ctx.drawImage(editCanvas, 0, 0, SIZE, SIZE, 0, 0, VIEW, VIEW);

    if (showGrid) {
      ctx.strokeStyle = "rgba(0,0,0,0.18)";
      ctx.lineWidth = 1;
      for (let i = 1; i < SIZE; i++) {
        ctx.beginPath();
        ctx.moveTo(i * SCALE + 0.5, 0);
        ctx.lineTo(i * SCALE + 0.5, VIEW);
        ctx.moveTo(0, i * SCALE + 0.5);
        ctx.lineTo(VIEW, i * SCALE + 0.5);
        ctx.stroke();
      }
    }

    ctx.strokeStyle = "rgba(232,178,58,0.85)";
    ctx.lineWidth = 2;
    for (const [gx, gy, gw, gh] of GUIDES) {
      ctx.strokeRect(gx * SCALE + 1, gy * SCALE + 1, gw * SCALE - 2, gh * SCALE - 2);
    }
    renderPreview();
  }

  function renderPreview() {
    drawFaces(previewCanvas, FRONT);
    drawFaces(previewBackCanvas, BACK);
  }

  function drawFaces(
    canvas: HTMLCanvasElement | undefined,
    faces: [number, number, number, number, number, number][]
  ) {
    const ctx = canvas?.getContext("2d");
    if (!ctx || !editCanvas) return;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, 16, 32);
    for (const [sx, sy, w, h, dx, dy] of faces) {
      ctx.drawImage(editCanvas, sx, sy, w, h, dx, dy, w, h);
    }
  }

  function texelAt(event: PointerEvent): { x: number; y: number } | null {
    if (!viewCanvas) return null;
    const rect = viewCanvas.getBoundingClientRect();
    const x = Math.floor(((event.clientX - rect.left) / rect.width) * SIZE);
    const y = Math.floor(((event.clientY - rect.top) / rect.height) * SIZE);
    if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return null;
    return { x, y };
  }

  function pushUndo() {
    undoStack.push(ectx.getImageData(0, 0, SIZE, SIZE));
    if (undoStack.length > 40) undoStack.shift();
    redoStack = [];
    canUndo = true;
    canRedo = false;
  }

  function paint(x: number, y: number) {
    if (tool === "erase") {
      ectx.clearRect(x, y, 1, 1);
    } else {
      ectx.fillStyle = color;
      ectx.clearRect(x, y, 1, 1);
      ectx.fillRect(x, y, 1, 1);
    }
  }

  function line(x0: number, y0: number, x1: number, y1: number) {

    const dx = Math.abs(x1 - x0), dy = Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
    let err = dx - dy;
    for (;;) {
      paint(x0, y0);
      if (x0 === x1 && y0 === y1) break;
      const e2 = 2 * err;
      if (e2 > -dy) { err -= dy; x0 += sx; }
      if (e2 < dx) { err += dx; y0 += sy; }
    }
  }

  function floodFill(sx: number, sy: number) {
    const img = ectx.getImageData(0, 0, SIZE, SIZE);
    const d = img.data;
    const idx = (x: number, y: number) => (y * SIZE + x) * 4;
    const target = [d[idx(sx, sy)], d[idx(sx, sy) + 1], d[idx(sx, sy) + 2], d[idx(sx, sy) + 3]];
    const rgb = hexToRgb(color);
    const repl = [rgb[0], rgb[1], rgb[2], 255];
    if (target.every((v, i) => v === repl[i])) return;
    const stack = [[sx, sy]];
    while (stack.length) {
      const [x, y] = stack.pop()!;
      if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) continue;
      const i = idx(x, y);
      if (d[i] !== target[0] || d[i + 1] !== target[1] || d[i + 2] !== target[2] || d[i + 3] !== target[3])
        continue;
      d[i] = repl[0]; d[i + 1] = repl[1]; d[i + 2] = repl[2]; d[i + 3] = repl[3];
      stack.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]);
    }
    ectx.putImageData(img, 0, 0);
  }

  function pickColor(x: number, y: number) {
    const p = ectx.getImageData(x, y, 1, 1).data;
    if (p[3] === 0) return;
    color = "#" + [p[0], p[1], p[2]].map((v) => v.toString(16).padStart(2, "0")).join("");
    tool = "draw";
  }

  function hexToRgb(hex: string): [number, number, number] {
    return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];
  }

  function onDown(event: PointerEvent) {
    const t = texelAt(event);
    if (!t) return;
    viewCanvas?.setPointerCapture(event.pointerId);
    if (tool === "pick") { pickColor(t.x, t.y); return; }
    pushUndo();
    if (tool === "fill") { floodFill(t.x, t.y); render(); return; }
    painting = true;
    last = t;
    paint(t.x, t.y);
    render();
  }

  function onMove(event: PointerEvent) {
    if (!painting) return;
    const t = texelAt(event);
    if (!t || !last) return;
    if (t.x === last.x && t.y === last.y) return;
    line(last.x, last.y, t.x, t.y);
    last = t;
    render();
  }

  function onUp() {
    painting = false;
    last = null;
  }

  function undo() {
    if (!undoStack.length) return;
    redoStack.push(ectx.getImageData(0, 0, SIZE, SIZE));
    ectx.putImageData(undoStack.pop()!, 0, 0);
    canUndo = undoStack.length > 0;
    canRedo = true;
    render();
  }

  function redo() {
    if (!redoStack.length) return;
    undoStack.push(ectx.getImageData(0, 0, SIZE, SIZE));
    ectx.putImageData(redoStack.pop()!, 0, 0);
    canRedo = redoStack.length > 0;
    canUndo = true;
    render();
  }

  function reset() {
    pushUndo();
    loadSkin();
  }

  $effect(() => {
    showGrid;
    if (!loading) render();
  });

  async function apply() {
    applying = true;
    try {
      const blob: Blob = await new Promise((resolve, reject) =>
        editCanvas.toBlob((b) => (b ? resolve(b) : reject(new Error("encode failed"))), "image/png")
      );
      const buf = new Uint8Array(await blob.arrayBuffer());
      await api.setSkin(Array.from(buf), variant);
      onApplied?.(editCanvas.toDataURL("image/png"));
      toast.success(t("skinEditor.applied"));
      onClose();
    } catch (err) {
      toast.error(String(err));
    } finally {
      applying = false;
    }
  }
</script>

<Modal title={t("skinEditor.title")} {open} {onClose} width={720}>
  <div class="editor">
    <div class="canvas-wrap">
      {#if loading}
        <div class="loading"><span class="spinner"></span> {t("skinEditor.loading")}</div>
      {/if}
      <canvas
        bind:this={viewCanvas}
        class="view"
        width={VIEW}
        height={VIEW}
        onpointerdown={onDown}
        onpointermove={onMove}
        onpointerup={onUp}
        onpointerleave={onUp}
      ></canvas>
    </div>

    <div class="side">
      <div class="preview-box">
        <span class="side-label">{t("skinEditor.preview")}</span>
        <div class="previews">
          <div class="pv">
            <canvas bind:this={previewCanvas} class="preview" width={16} height={32}></canvas>
            <span class="pv-label">{t("skinEditor.front")}</span>
          </div>
          <div class="pv">
            <canvas bind:this={previewBackCanvas} class="preview" width={16} height={32}></canvas>
            <span class="pv-label">{t("skinEditor.back")}</span>
          </div>
        </div>
      </div>

      <div class="tools">
        {#each TOOLS as [id, key] (id)}
          <button class="tool" class:on={tool === id} onclick={() => (tool = id)}>
            {t(key)}
          </button>
        {/each}
      </div>

      <div class="color-row">
        <input class="color-input" type="color" bind:value={color} aria-label={t("skinEditor.color")} />
        <div class="swatches">
          {#each PALETTE as swatch (swatch)}
            <button
              class="swatch"
              class:on={color.toLowerCase() === swatch}
              style={`background:${swatch}`}
              aria-label={swatch}
              onclick={() => (color = swatch)}
            ></button>
          {/each}
        </div>
      </div>

      <div class="actions">
        <button class="mini" disabled={!canUndo} title={t("skinEditor.undo")} onclick={undo}>↶</button>
        <button class="mini" disabled={!canRedo} title={t("skinEditor.redo")} onclick={redo}>↷</button>
        <button class="mini" class:on={showGrid} title={t("skinEditor.grid")} onclick={() => (showGrid = !showGrid)}>#</button>
        <button class="mini wide" onclick={reset}>
          <Icon name="refresh" size={13} /> {t("skinEditor.reset")}
        </button>
      </div>
    </div>
  </div>

  {#snippet footer()}
    <button class="btn ghost" onclick={onClose}>{t("common.cancel")}</button>
    <button class="btn primary" disabled={applying || loading} onclick={apply}>
      {applying ? t("skinEditor.applying") : t("skinEditor.apply")}
    </button>
  {/snippet}
</Modal>

<style>
  .editor {
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }
  .canvas-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .view {
    width: 100%;
    height: auto;
    aspect-ratio: 1;
    image-rendering: pixelated;
    background: var(--bg-app);
    border: 2px solid var(--border);
    touch-action: none;
    cursor: crosshair;
  }
  .loading {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: rgba(0, 0, 0, 0.5);
    color: var(--text-secondary);
    font-size: 13px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .side {
    width: 190px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .side-label {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
    margin-bottom: 6px;
  }
  .preview-box {
    text-align: center;
  }
  .previews {
    display: flex;
    justify-content: center;
    gap: 10px;
  }
  .pv {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .pv-label {
    font-size: 10px;
    color: var(--text-muted);
  }
  .preview {
    width: 74px;
    height: 148px;
    image-rendering: pixelated;
    background: var(--bg-app);
    border: 2px solid var(--border);
  }
  .tools {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .tool {
    padding: 8px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
  }
  .tool:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .tool.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .color-row {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .color-input {
    width: 34px;
    height: 34px;
    padding: 0;
    border: 2px solid var(--border);
    background: none;
    cursor: pointer;
    flex-shrink: 0;
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    flex: 1;
  }
  .swatch {
    width: 100%;
    aspect-ratio: 1;
    border: 2px solid var(--border);
    cursor: pointer;
    padding: 0;
  }
  .swatch.on {
    border-color: var(--text);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .mini {
    min-width: 34px;
    height: 32px;
    padding: 0 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 15px;
    cursor: pointer;
  }
  .mini.wide {
    flex: 1;
    font-size: 12.5px;
  }
  .mini:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
  }
  .mini.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .mini:disabled {
    opacity: 0.4;
    cursor: default;
  }
  @media (max-width: 640px) {
    .editor {
      flex-direction: column;
    }
    .side {
      width: 100%;
    }
  }
</style>
