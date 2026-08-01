<script lang="ts">
  import type { Instance } from "$lib/types";
  import { LOADER_COLORS } from "$lib/themes";

  interface Props {
    instance: Instance;
    size?: number;
  }
  let { instance, size = 48 }: Props = $props();

  const initials = $derived(
    instance.name
      .split(/\s+/)
      .slice(0, 2)
      .map((word) => word[0]?.toUpperCase() ?? "")
      .join("") || "?"
  );
  const backgroundColor = $derived(LOADER_COLORS[instance.loader] ?? LOADER_COLORS.forge);
</script>

{#if instance.icon}
  <img
    src={instance.icon}
    alt={instance.name}
    style="width:{size}px;height:{size}px;"
    class="icon-img"
  />
{:else}
  <div
    class="icon-fallback"
    style="width:{size}px;height:{size}px;background:{backgroundColor};font-size:{Math.round(
      size * 0.38
    )}px;"
  >
    {initials}
  </div>
{/if}

<style>
  .icon-img {
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bg-card);
    border: 2px solid rgba(0, 0, 0, 0.35);
    image-rendering: pixelated;
  }
  .icon-fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-pixel);
    font-weight: 700;
    color: rgba(255, 255, 255, 0.92);
    flex-shrink: 0;
    border: 2px solid rgba(0, 0, 0, 0.3);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.18),
      inset -2px -2px 0 rgba(0, 0, 0, 0.25);
    text-shadow: 0 1px 0 rgba(0, 0, 0, 0.4);
  }
</style>
