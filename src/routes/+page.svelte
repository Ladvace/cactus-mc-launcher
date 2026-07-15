<script lang="ts">
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import InstanceGrid from "$lib/components/InstanceGrid.svelte";
  import InstanceCardSkeleton from "$lib/components/InstanceCardSkeleton.svelte";
  import Icon from "$lib/components/Icon.svelte";

  let arranging = $state(false);
  let grid = $state<InstanceGrid>();

  const instances = $derived(instancesStore.instances);
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

  {#if instancesStore.loading && !instancesStore.loaded}
    <section>
      <h3 class="section-title"><Icon name="cube" size={16} /> Your instances</h3>
      <div class="grid">
        {#each Array(6) as _, i (i)}
          <InstanceCardSkeleton />
        {/each}
      </div>
    </section>
  {:else if instances.length === 0}
    <div class="empty">
      <div class="empty-mark"><Icon name="cube" size={40} /></div>
      <h2>No instances yet</h2>
      <p>Create your first instance to start playing Minecraft.</p>
      <button class="btn primary" onclick={() => ui.openCreateInstance()}>
        <Icon name="plus" size={16} /> Create instance
      </button>
    </div>
  {:else}
    <section>
      <div class="section-head">
        <h3 class="section-title">
          <Icon name="cube" size={16} /> Your instances
        </h3>
        <button
          class="btn ghost arrange"
          class:on={arranging}
          title="Drag & resize tiles into a custom layout"
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

      <InstanceGrid bind:this={grid} {instances} {arranging} />
    </section>
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
    margin-bottom: 28px;
  }
  .hero h1 {
    font-size: 26px;
  }
  .hero p {
    margin: 6px 0 0;
    color: var(--text-secondary);
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--text-secondary);
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
    margin: 0 0 16px;
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
