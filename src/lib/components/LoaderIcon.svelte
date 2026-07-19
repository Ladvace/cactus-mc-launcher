<script lang="ts">
  import fabric from "$lib/loaders/fabric.svg?raw";
  import quilt from "$lib/loaders/quilt.svg?raw";
  import forge from "$lib/loaders/forge.svg?raw";
  import neoforge from "$lib/loaders/neoforge.svg?raw";
  import vanilla from "$lib/loaders/vanilla.svg?raw";
  import type { ModLoader } from "$lib/types";

  interface Props {
    loader: ModLoader;
    size?: number;
  }
  let { loader, size = 18 }: Props = $props();

  const svgs: Record<ModLoader, string> = {
    fabric,
    quilt,
    forge,
    neoforge,
    vanilla,
  };
</script>

<span
  class="loader-icon"
  style="width:{size}px;height:{size}px"
  aria-hidden="true">{@html svgs[loader]}</span>

<style>
  .loader-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .loader-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
