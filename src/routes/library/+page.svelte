<script lang="ts">
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import InstanceCard from "$lib/components/InstanceCard.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { MOD_LOADERS, type ModLoader } from "$lib/types";

  let query = $state("");
  let loaderFilter = $state<ModLoader | "all">("all");

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
  <header class="head">
    <h1>Library</h1>
    <button class="btn primary" onclick={() => ui.openCreateInstance()}>
      <Icon name="plus" size={16} /> New instance
    </button>
  </header>

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
  </div>

  {#if instancesStore.instances.length === 0}
    <div class="empty">
      <p>Your library is empty.</p>
      <button class="btn primary" onclick={() => ui.openCreateInstance()}>
        <Icon name="plus" size={16} /> Create instance
      </button>
    </div>
  {:else if filtered.length === 0}
    <p class="muted">No instances match your filters.</p>
  {:else}
    <div class="grid">
      {#each filtered as inst (inst.id)}
        <InstanceCard instance={inst} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }
  .head h1 {
    font-size: 24px;
  }
  .toolbar {
    display: flex;
    gap: 12px;
    margin-bottom: 22px;
  }
  .search {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  .search :global(svg) {
    position: absolute;
    left: 12px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 9px 12px 9px 36px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 13px;
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .loader-filter {
    width: auto;
    min-width: 140px;
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
    padding: 64px;
    color: var(--text-secondary);
  }
</style>
