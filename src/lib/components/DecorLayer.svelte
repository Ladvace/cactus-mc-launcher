<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { DECOR_THEMES } from "$lib/themes";

  // Sprites placed at fixed spots around the page for the active decor theme.
  const theme = $derived(
    DECOR_THEMES.find((t) => t.id === (settingsStore.settings.decorTheme ?? ""))
  );
</script>

{#if theme}
  <div class="decor-layer" aria-hidden="true">
    {#each theme.placements as p, i (i)}
      <img
        src={p.sprite}
        alt=""
        style="{p.at} width:{p.size}px; opacity:{p.opacity ?? 1}; transform: rotate({p.rotate ?? 0}deg){p.flip ? ' scaleX(-1)' : ''};"
      />
    {/each}
  </div>
{/if}

<style>
  /* Above the background and content edges, below the dock, non-interactive. */
  .decor-layer {
    position: fixed;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    overflow: hidden;
  }
  .decor-layer img {
    position: fixed;
    height: auto;
    image-rendering: auto;
    filter: drop-shadow(0 2px 5px rgba(0, 0, 0, 0.45));
  }
</style>
