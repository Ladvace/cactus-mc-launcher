<script lang="ts">
  // A small pixel-art flag. Sources are low-res PNGs rendered crisp (pixelated).
  let { code, width = 22 }: { code: string; width?: number } = $props();
</script>

<img
  class="flag"
  src={`/flags/${code}.png`}
  alt=""
  aria-hidden="true"
  style={`width:${width}px`}
  draggable="false"
/>

<style>
  .flag {
    display: block;
    height: auto;
    aspect-ratio: 3 / 2;
    image-rendering: pixelated;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
    flex-shrink: 0;
  }
</style>
