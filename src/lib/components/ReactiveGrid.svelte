<script lang="ts">
  import { onMount } from "svelte";

  // A cursor-reactive overlay for the "Circuit" grid background: the cell under
  // the pointer lights up and fades, leaving a trail as you move. Drawn on a
  // canvas over the (CSS-drawn) grid, behind the app UI. CELL must match the
  // `.anim-circuit` grid size in app.css.
  const CELL = 30;
  const TRAIL_MS = 700; // how long a hovered cell stays lit before fading out

  let canvas = $state<HTMLCanvasElement>();

  onMount(() => {
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;

    type Lit = { col: number; row: number; born: number };
    let lit: Lit[] = [];
    let raf = 0;
    let running = false;

    // Accent as an "r, g, b" string, read once from the CSS variable.
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

    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      canvas!.width = Math.round(window.innerWidth * dpr);
      canvas!.height = Math.round(window.innerHeight * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();

    const tick = () => {
      const now = performance.now();
      lit = lit.filter((cell) => now - cell.born < TRAIL_MS);
      ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
      for (const cell of lit) {
        const life = 1 - (now - cell.born) / TRAIL_MS;
        ctx.fillStyle = `rgba(${rgb}, ${life * 0.45})`;
        ctx.fillRect(cell.col * CELL, cell.row * CELL, CELL, CELL);
      }
      // Keep animating only while cells are fading; go idle otherwise.
      if (lit.length) {
        raf = requestAnimationFrame(tick);
      } else {
        running = false;
      }
    };
    const ensureRunning = () => {
      if (!running) {
        running = true;
        raf = requestAnimationFrame(tick);
      }
    };

    const onMove = (event: MouseEvent) => {
      const col = Math.floor(event.clientX / CELL);
      const row = Math.floor(event.clientY / CELL);
      const last = lit[lit.length - 1];
      if (last && last.col === col && last.row === row) {
        last.born = performance.now(); // keep the current cell lit while hovering it
      } else {
        lit.push({ col, row, born: performance.now() });
      }
      ensureRunning();
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("resize", resize);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("resize", resize);
    };
  });
</script>

<canvas bind:this={canvas} class="reactive-grid" aria-hidden="true"></canvas>

<style>
  .reactive-grid {
    position: fixed;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
