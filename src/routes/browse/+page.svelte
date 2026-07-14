<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";

  const tabs = ["Modpacks", "Mods", "Resource Packs", "Shaders", "Datapacks"];
  let active = $state("Modpacks");
</script>

<div class="page">
  <header class="head">
    <h1>Browse</h1>
  </header>

  <div class="tabs">
    {#each tabs as t}
      <button class="tab" class:active={active === t} onclick={() => (active = t)}>
        {t}
      </button>
    {/each}
  </div>

  <div class="placeholder">
    <div class="mark"><Icon name="package" size={40} /></div>
    <h2>Content discovery coming soon</h2>
    <p>
      This tab will search Modrinth for {active.toLowerCase()} and let you install
      them into an instance with one click. Wired up in the Modrinth integration
      milestone.
    </p>
  </div>
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .head h1 {
    font-size: 24px;
    margin-bottom: 18px;
  }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 32px;
  }
  .tab {
    padding: 10px 14px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    font-size: 13.5px;
    font-weight: 500;
    margin-bottom: -1px;
    transition: color 0.12s, border-color 0.12s;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 72px 24px;
    color: var(--text-secondary);
    gap: 10px;
  }
  .mark {
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
  .placeholder h2 {
    font-size: 18px;
  }
  .placeholder p {
    max-width: 420px;
    margin: 0;
  }
</style>
