<script lang="ts">
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import InstanceGrid from "$lib/components/InstanceGrid.svelte";
  import InstanceCardSkeleton from "$lib/components/InstanceCardSkeleton.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { MOD_LOADERS, type ModLoader } from "$lib/types";

  let query = $state("");
  let loaderFilter = $state<ModLoader | "all">("all");
  let arranging = $state(false);
  let grid = $state<InstanceGrid>();

  // Arranging reorders the whole collection, so it only makes sense on the
  // full, unfiltered list.
  const filtersActive = $derived(query.trim() !== "" || loaderFilter !== "all");
  $effect(() => {
    if (filtersActive && arranging) arranging = false;
  });

  const filtered = $derived(
    instancesStore.instances.filter((i) => {
      const matchesQuery =
        !query.trim() ||
        i.name.toLowerCase().includes(query.trim().toLowerCase()) ||
        i.mcVersion.includes(query.trim());
      const matchesLoader = loaderFilter === "all" || i.loader === loaderFilter;
      return matchesQuery && matchesLoader;
    })
  );
</script>

<div class="page">
  <header class="hero">
    <div>
      <h1>Welcome back</h1>
      <p>Arrange your instances into a home screen that's yours.</p>
    </div>
    <button class="btn primary" onclick={() => ui.openCreateInstance()}>
      <Icon name="plus" size={16} /> New instance
    </button>
  </header>

  {#if instancesStore.instances.length > 0}
    <div class="toolbar">
      <div class="search">
        <Icon name="search" size={16} />
        <input
          class="search-input"
          placeholder="Search instances…"
          bind:value={query}
        />
      </div>
      <select class="select loader-filter" bind:value={loaderFilter}>
        <option value="all">All loaders</option>
        {#each MOD_LOADERS as l}
          <option value={l.value}>{l.label}</option>
        {/each}
      </select>
      <button
        class="btn ghost arrange"
        class:on={arranging}
        disabled={filtersActive}
        title={filtersActive
          ? "Clear filters to rearrange"
          : "Drag & resize tiles into a custom layout"}
        onclick={() => (arranging = !arranging)}
      >
        <Icon name={arranging ? "check" : "expand"} size={15} />
        {arranging ? "Done" : "Arrange"}
      </button>
    </div>

    {#if arranging}
      <div class="arrange-hint">
        <span>
          Drag a tile to reorder · drag its right edge, bottom edge, or
          <Icon name="expand" size={12} /> corner to resize. Saved automatically.
        </span>
        <button class="reset" onclick={() => grid?.resetLayout()}>
          <Icon name="refresh" size={12} /> Reset layout
        </button>
      </div>
    {/if}
  {/if}

  {#if instancesStore.loading && !instancesStore.loaded}
    <div class="grid">
      {#each Array(6) as _, i (i)}
        <InstanceCardSkeleton />
      {/each}
    </div>
  {:else if instancesStore.instances.length === 0}
    <div class="empty">
      <div class="empty-mark"><Icon name="cube" size={40} /></div>
      <h2>No instances yet</h2>
      <p>Create your first instance to start playing Minecraft.</p>
      <button class="btn primary" onclick={() => ui.openCreateInstance()}>
        <Icon name="plus" size={16} /> Create instance
      </button>
    </div>
  {:else if filtered.length === 0}
    <p class="muted">No instances match your filters.</p>
  {:else}
    <InstanceGrid bind:this={grid} instances={filtered} {arranging} />
  {/if}
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .hero {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 24px;
  }
  .hero h1 {
    font-size: 26px;
  }
  .hero p {
    margin: 6px 0 0;
    color: var(--text-secondary);
  }
  .toolbar {
    display: flex;
    gap: 12px;
    margin-bottom: 18px;
  }
  .search {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  .search :global(.hn) {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 9px 12px 9px 36px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .loader-filter {
    width: auto;
    min-width: 140px;
  }
  .arrange {
    flex-shrink: 0;
  }
  .arrange.on {
    --face: var(--accent);
    --ink: var(--accent-contrast);
    --depth: #7c5a12;
  }
  .arrange-hint {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin: -6px 0 16px;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .arrange-hint span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex-wrap: wrap;
  }
  .reset {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    background: transparent;
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    transition: color 0.12s, border-color 0.12s;
  }
  .reset:hover {
    color: var(--danger);
    border-color: var(--danger);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 16px;
  }
  .muted {
    color: var(--text-muted);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    text-align: center;
    padding: 72px 24px;
    color: var(--text-secondary);
  }
  .empty-mark {
    color: var(--text-muted);
    background: var(--bg-card);
    width: 88px;
    height: 88px;
    border-radius: 0;
    border: 2px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 6px;
  }
  .empty h2 {
    font-size: 18px;
  }
  .empty p {
    margin: 0 0 8px;
    max-width: 340px;
  }
</style>
