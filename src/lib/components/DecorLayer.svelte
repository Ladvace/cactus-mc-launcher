<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { DECOR_THEMES } from "$lib/themes";

  // Sprites placed at fixed spots around the page for the active decor theme.
  const theme = $derived(
    DECOR_THEMES.find((decorTheme) => decorTheme.id === (settingsStore.settings.decorTheme ?? ""))
  );
</script>

{#if theme}
  <div class="decor-layer" aria-hidden="true">
    {#each theme.placements as placement, index (index)}
      <img
        src={placement.sprite}
        alt=""
        style="{placement.at} width:{placement.size}px; opacity:{placement.opacity ?? 1}; transform: rotate({placement.rotate ?? 0}deg){placement.flip ? ' scaleX(-1)' : ''};"
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
