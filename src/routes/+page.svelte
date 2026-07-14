<script lang="ts">
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import InstanceCard from "$lib/components/InstanceCard.svelte";
  import Icon from "$lib/components/Icon.svelte";

  const recent = $derived(
    instancesStore.instances
      .filter((i) => i.lastPlayed)
      .slice(0, 5)
  );
  const jumpBackIn = $derived(instancesStore.instances.slice(0, 5));
</script>

<div class="page">
  <header class="hero">
    <div>
      <h1>Welcome back</h1>
      <p>Jump into an instance or create a new one to get started.</p>
    </div>
    <button class="btn primary" onclick={() => ui.openCreateInstance()}>
      <Icon name="plus" size={16} /> New instance
    </button>
  </header>

  {#if instancesStore.loading && !instancesStore.loaded}
    <p class="muted">Loading…</p>
  {:else if instancesStore.instances.length === 0}
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
      <h3 class="section-title">
        <Icon name="clock" size={16} /> Jump back in
      </h3>
      <div class="grid">
        {#each (recent.length ? recent : jumpBackIn) as inst (inst.id)}
          <InstanceCard instance={inst} />
        {/each}
      </div>
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
  .section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--text-secondary);
    margin-bottom: 14px;
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
    border-radius: 24px;
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
