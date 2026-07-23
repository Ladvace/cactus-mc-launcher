<script lang="ts">
  import { MATERIALS, MATERIAL_TEX, RECIPES, RECIPE_COUNT, matchRecipe, texUrl, type MaterialId } from "$lib/crafting";
  import { craftingStore } from "$lib/stores/crafting.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { playClick } from "$lib/sound";
  import { t } from "$lib/i18n";

  let { open, onClose }: { open: boolean; onClose: () => void } = $props();

  let grid = $state<(MaterialId | null)[]>(Array(9).fill(null));
  let popped = $state(false);
  let showRecipes = $state(true);

  // The item currently held on the cursor (Minecraft-style drag), and its position.
  let carry = $state<MaterialId | null>(null);
  let cursor = $state({ x: 0, y: 0 });

  const result = $derived(matchRecipe(grid));

  // Vanilla inventory grid; our materials fill the first slots.
  const MAIN = Array.from({ length: 27 }, (_, i) => MATERIALS[i]?.id ?? null);

  function pickUp(mat: MaterialId, e: PointerEvent) {
    e.preventDefault();
    carry = mat;
    cursor = { x: e.clientX, y: e.clientY };
  }
  function pickFromGrid(i: number, e: PointerEvent) {
    if (!grid[i]) return;
    e.preventDefault();
    carry = grid[i];
    grid[i] = null; // lift it off the grid; dropping outside discards it
    cursor = { x: e.clientX, y: e.clientY };
  }
  function onMove(e: PointerEvent) {
    if (carry) cursor = { x: e.clientX, y: e.clientY };
  }
  function onUp(e: PointerEvent) {
    if (!carry) return;
    const slot = document.elementFromPoint(e.clientX, e.clientY)?.closest("[data-slot]");
    if (slot) grid[Number((slot as HTMLElement).dataset.slot)] = carry;
    carry = null;
  }
  function craft() {
    if (!result) return;
    craftingStore.discover(result.id);
    if (result.reward.accent) {
      settingsStore.save({ ...settingsStore.settings, accent: result.reward.accent });
    }
    playClick(0.18);
    toast.success(`${t("craft.crafted", { name: result.name })} ${result.reward.message}`);
    popped = true;
    setTimeout(() => (popped = false), 350);
    grid = Array(9).fill(null);
  }

  $effect(() => {
    if (!open) {
      grid = Array(9).fill(null);
      carry = null;
    }
  });
</script>

<svelte:window
  onkeydown={(e) => open && e.key === "Escape" && onClose()}
  onpointermove={onMove}
  onpointerup={onUp}
/>

{#if open}
  <div class="overlay" role="button" tabindex="-1" onclick={onClose} onkeydown={() => {}}>
    <div
      class="gui"
      role="dialog"
      aria-modal="true"
      aria-label={t("craft.title")}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={() => {}}
    >
      <span class="title">{t("craft.title")}</span>

      <div class="top">
        <button
          class="book"
          class:on={showRecipes}
          title={t("craft.discovered", { n: craftingStore.discovered.length, total: RECIPE_COUNT })}
          onclick={() => (showRecipes = !showRecipes)}
        >
          <img src={texUrl("item/knowledge_book.png")} alt="" />
        </button>

        <div class="grid">
          {#each grid as cell, i (i)}
            <button
              class="slot"
              data-slot={i}
              draggable="false"
              onpointerdown={(e) => pickFromGrid(i, e)}
            >
              {#if cell}<img src={texUrl(MATERIAL_TEX[cell])} alt="" />{/if}
            </button>
          {/each}
        </div>

        <svg class="arrow" viewBox="0 0 22 16" aria-hidden="true">
          <path
            d="M0 5 H12 V1 L21 8 L12 15 V11 H0 Z"
            fill="#8b8b8b"
            stroke="#5a5a5a"
            stroke-width="1"
            stroke-linejoin="round"
          />
        </svg>

        <button class="slot output" class:on={!!result} class:pop={popped} disabled={!result} title={result?.name} onclick={craft}>
          {#if result}<img src={texUrl(result.resultTex)} alt={result.name} />{/if}
        </button>
      </div>

      <span class="title inv-title">{t("craft.inventory")}</span>
      <div class="inv">
        {#each MAIN as mat, i (i)}
          {#if mat}
            <button
              class="slot item"
              draggable="false"
              onpointerdown={(e) => pickUp(mat, e)}
              title={MATERIALS.find((m) => m.id === mat)?.name}
            >
              <img src={texUrl(MATERIAL_TEX[mat])} alt="" />
            </button>
          {:else}
            <span class="slot"></span>
          {/if}
        {/each}
      </div>

      {#if showRecipes}
        <div class="recipes">
          <div class="rec-row">
            {#each RECIPES as recipe (recipe.id)}
              {@const found = craftingStore.has(recipe.id)}
              <span class="slot rec" title={found ? recipe.name : ""}>
                {#if found}<img src={texUrl(recipe.resultTex)} alt="" />{/if}
              </span>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    {#if carry}
      <img
        class="carry"
        src={texUrl(MATERIAL_TEX[carry])}
        style="left: {cursor.x}px; top: {cursor.y}px"
        alt=""
      />
    {/if}
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.55);
    animation: fade 0.12s ease;
  }
  .gui {
    --panel: #c6c6c6;
    --slot: #8b8b8b;
    --dark: #373737;
    --light: #ffffff;
    --frame: #555555;
    --ink: #404040;
    background: var(--panel);
    padding: 12px;
    box-shadow: inset 2px 2px 0 var(--light), inset -2px -2px 0 var(--frame);
    animation: pop 0.14s ease;
    user-select: none;
    touch-action: none;
  }
  .title {
    display: block;
    color: var(--ink);
    font-family: var(--font-pixel);
    font-size: 15px;
    margin: 2px 2px 8px;
  }
  .inv-title {
    margin-top: 12px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 44px);
    grid-template-rows: repeat(3, 44px);
    gap: 2px;
  }
  /* Recessed vanilla slot: dark top-left, light bottom-right. */
  .slot {
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    background: var(--slot);
    box-shadow: inset 2px 2px 0 var(--dark), inset -2px -2px 0 var(--light);
  }
  .slot img {
    width: 30px;
    height: 30px;
    image-rendering: pixelated;
    pointer-events: none;
  }
  button.slot {
    cursor: pointer;
    position: relative;
  }
  button.slot:not(:disabled):hover::after {
    content: "";
    position: absolute;
    inset: 2px;
    background: rgba(255, 255, 255, 0.55);
    pointer-events: none;
  }
  .arrow {
    width: 30px;
    height: 22px;
    flex-shrink: 0;
  }
  .output {
    width: 52px;
    height: 52px;
  }
  .output:disabled {
    cursor: default;
  }
  .output img {
    width: 36px;
    height: 36px;
  }
  .output.on::before {
    content: "";
    position: absolute;
    inset: 2px;
    background: rgba(255, 255, 255, 0.35);
    animation: glow 1.3s ease-in-out infinite;
    pointer-events: none;
  }
  .output.pop {
    animation: pop 0.35s ease;
  }
  .carry {
    position: fixed;
    width: 32px;
    height: 32px;
    image-rendering: pixelated;
    transform: translate(-50%, -50%);
    pointer-events: none;
    z-index: 200;
  }
  .inv {
    display: grid;
    grid-template-columns: repeat(9, 40px);
    gap: 2px;
  }
  .inv .slot {
    width: 40px;
    height: 40px;
  }
  .inv .slot img {
    width: 28px;
    height: 28px;
  }
  .recipes {
    margin-top: 8px;
  }
  .rec-row {
    display: grid;
    grid-template-columns: repeat(9, 40px);
    gap: 2px;
  }
  .rec {
    width: 40px;
    height: 40px;
  }
  .rec img {
    width: 28px;
    height: 28px;
  }
  .book {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: 0;
    border: 0;
    color: #e9ffe0;
    background: #4a8f3c;
    box-shadow: inset 2px 2px 0 #7bc25f, inset -2px -2px 0 #2f5f26;
    cursor: pointer;
  }
  .book.on,
  .book:hover {
    background: #57a844;
  }
  .book img {
    width: 26px;
    height: 26px;
    image-rendering: pixelated;
    pointer-events: none;
  }
  @keyframes glow {
    50% {
      opacity: 0.35;
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
    40% {
      transform: scale(1.06);
    }
  }
</style>
