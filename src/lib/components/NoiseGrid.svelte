<script lang="ts">
  import { onMount } from "svelte";

  // Ambient overlay for the "Lattice" background: small grid cells fade in/out
  // following animated 3D value noise (Perlin-like), giving an organic drifting
  // field. Drawn on a canvas over the (CSS-drawn) fine grid, behind the app UI.
  // CELL must match the `.anim-lattice` grid size in app.css.
  const CELL = 16;
  const SCALE = 0.14; // noise frequency across cells
  const TIME = 0.00028; // noise evolution speed (per ms)
  const THRESHOLD = 0.52; // a cell lights only where noise exceeds this
  const MAX_ALPHA = 0.5;
  const FRAME_MS = 33; // ~30fps is plenty for a soft ambient field

  let canvas = $state<HTMLCanvasElement>();

  onMount(() => {
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;

    const accentRgb = (): string => {
      const raw = getComputedStyle(document.documentElement)
        .getPropertyValue("--accent")
        .trim();
      const match = /^#?([0-9a-f]{6})$/i.exec(raw);
      if (!match) return "232, 178, 58";
      const n = parseInt(match[1], 16);
      return `${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}`;
    };
    const rgb = accentRgb();

    // Cheap integer hash → [0,1), and 3D value noise (trilinear, smoothstep).
    const hash = (x: number, y: number, z: number): number => {
      let h = Math.imul(x, 374761393) + Math.imul(y, 668265263) + Math.imul(z, 1274126177);
      h = Math.imul(h ^ (h >>> 13), 1274126177);
      h ^= h >>> 16;
      return (h >>> 0) / 4294967295;
    };
    const fade = (t: number) => t * t * (3 - 2 * t);
    const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
    const noise = (x: number, y: number, z: number): number => {
      const xi = Math.floor(x), yi = Math.floor(y), zi = Math.floor(z);
      const xf = x - xi, yf = y - yi, zf = z - zi;
      const u = fade(xf), v = fade(yf), w = fade(zf);
      const c = (i: number, j: number, k: number) => hash(xi + i, yi + j, zi + k);
      return lerp(
        lerp(lerp(c(0, 0, 0), c(1, 0, 0), u), lerp(c(0, 1, 0), c(1, 1, 0), u), v),
        lerp(lerp(c(0, 0, 1), c(1, 0, 1), u), lerp(c(0, 1, 1), c(1, 1, 1), u), v),
        w,
      );
    };

    let w = 0, h = 0;
    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      w = window.innerWidth;
      h = window.innerHeight;
      canvas!.width = Math.round(w * dpr);
      canvas!.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();

    let raf = 0;
    let last = 0;
    const draw = (now: number) => {
      raf = requestAnimationFrame(draw);
      if (now - last < FRAME_MS) return;
      last = now;
      const z = now * TIME;
      ctx.clearRect(0, 0, w, h);
      const cols = Math.ceil(w / CELL);
      const rows = Math.ceil(h / CELL);
      for (let row = 0; row < rows; row++) {
        for (let col = 0; col < cols; col++) {
          const n = noise(col * SCALE, row * SCALE, z);
          if (n <= THRESHOLD) continue;
          const alpha = ((n - THRESHOLD) / (1 - THRESHOLD)) * MAX_ALPHA;
          ctx.fillStyle = `rgba(${rgb}, ${alpha})`;
          ctx.fillRect(col * CELL, row * CELL, CELL - 1, CELL - 1);
        }
      }
    };
    raf = requestAnimationFrame(draw);

    window.addEventListener("resize", resize);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
    };
  });
</script>

<canvas bind:this={canvas} class="noise-grid" aria-hidden="true"></canvas>

<style>
  .noise-grid {
    position: fixed;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
