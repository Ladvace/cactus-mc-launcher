<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { SkinViewer, IdleAnimation } from "skinview3d";

  interface Props {
    /** Skin texture as a URL or data URI. */
    skin: string;
    /** Arm model — changes the model live. */
    model?: "classic" | "slim";
    /** Optional cape texture (data URI). Empty = no cape. */
    cape?: string;
    width?: number;
    height?: number;
  }
  let { skin, model = "classic", cape = "", width = 200, height = 280 }: Props =
    $props();

  let canvas = $state<HTMLCanvasElement>();
  let viewer: SkinViewer | undefined;

  function applySkin() {
    if (!viewer || !skin) return;
    viewer
      .loadSkin(skin, { model: model === "slim" ? "slim" : "default" })
      .catch(() => {});
  }
  function applyCape() {
    if (!viewer) return;
    if (cape) viewer.loadCape(cape).catch(() => {});
    else viewer.resetCape();
  }

  onMount(() => {
    if (!canvas) return;
    viewer = new SkinViewer({ canvas, width, height });
    viewer.autoRotate = true;
    viewer.autoRotateSpeed = 0.5;
    viewer.animation = new IdleAnimation();
    viewer.controls.enableZoom = false;
    viewer.controls.enablePan = false;
    applySkin();
    applyCape();
  });

  onDestroy(() => viewer?.dispose());

  $effect(() => {
    void skin;
    void model;
    applySkin();
  });
  $effect(() => {
    void cape;
    applyCape();
  });
</script>

<canvas bind:this={canvas} class="skin-canvas"></canvas>

<style>
  .skin-canvas {
    display: block;
    cursor: grab;
  }
  .skin-canvas:active {
    cursor: grabbing;
  }
</style>
