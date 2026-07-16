<script lang="ts">
  import { goto } from "$app/navigation";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import InstanceGrid from "$lib/components/InstanceGrid.svelte";
  import InstanceCardSkeleton from "$lib/components/InstanceCardSkeleton.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import ContextMenu, { type MenuItem } from "$lib/components/ContextMenu.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { api } from "$lib/api";
  import { boardApi } from "$lib/boardApi";
  import { MOD_LOADERS, type ModLoader } from "$lib/types";

  let query = $state("");
  let loaderFilter = $state<ModLoader | "all">("all");
  let arranging = $state(false);
  let grid = $state<InstanceGrid>();

  // --- Import / share context menu ---
  const online = boardApi.configured();
  let fileInput = $state<HTMLInputElement>();
  let homeMenu = $state<{ x: number; y: number } | null>(null);
  let codeOpen = $state(false);
  let code = $state("");
  let importing = $state(false);
  let importError = $state<string | null>(null);

  const menuItems = $derived<MenuItem[]>([
    { label: "New instance", icon: "plus", onSelect: () => ui.openCreateInstance() },
    { separator: true },
    { label: "Import setup file…", icon: "download", onSelect: () => fileInput?.click() },
    ...(online
      ? [{ label: "Import from a code…", icon: "share", onSelect: () => (codeOpen = true) }]
      : []),
  ]);

  function openMenu(e: MouseEvent) {
    e.preventDefault();
    homeMenu = { x: e.clientX, y: e.clientY };
  }

  async function onImportFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    importing = true;
    importError = null;
    try {
      const buf = await file.arrayBuffer();
      const res = await api.importSetup(Array.from(new Uint8Array(buf)));
      await instancesStore.refresh();
      goto(`/instance/${res.instance.id}`);
    } catch (err) {
      importError = String(err);
      setTimeout(() => (importError = null), 4000);
    } finally {
      importing = false;
    }
  }

  async function importFromCode() {
    const c = code.trim();
    if (!c || importing) return;
    importing = true;
    importError = null;
    try {
      const { snapshotId } = await boardApi.resolveCode(c);
      const res = await boardApi.importSnapshot(snapshotId);
      await instancesStore.refresh();
      codeOpen = false;
      code = "";
      goto(`/instance/${res.instance.id}`);
    } catch (e) {
      importError = String(e);
    } finally {
      importing = false;
    }
  }

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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="page" oncontextmenu={openMenu}>
  <header class="hero">
    <div>
      <h1>Welcome back</h1>
      <p>Arrange your instances into a home screen that's yours.</p>
    </div>
    <div class="hero-actions">
      <button class="btn ghost" title="Import a shared setup" onclick={openMenu}>
        <Icon name="download" size={15} /> Import
      </button>
      <button class="btn primary" onclick={() => ui.openCreateInstance()}>
        <Icon name="plus" size={16} /> New instance
      </button>
    </div>
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

<input
  bind:this={fileInput}
  type="file"
  accept=".drakepack,.mrpack,application/zip"
  style="display:none"
  onchange={onImportFile}
/>

{#if homeMenu}
  <ContextMenu
    x={homeMenu.x}
    y={homeMenu.y}
    items={menuItems}
    onClose={() => (homeMenu = null)}
  />
{/if}

{#if importing}
  <div class="toast" role="status">Importing…</div>
{:else if importError}
  <div class="toast err" role="alert">{importError}</div>
{/if}

<Modal title="Import from a code" open={codeOpen} onClose={() => (codeOpen = false)} width={380}>
  <input
    class="input"
    placeholder="Paste a share code…"
    bind:value={code}
    onkeydown={(e) => e.key === "Enter" && importFromCode()}
  />
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (codeOpen = false)}>Cancel</button>
    <button class="btn primary" disabled={importing || !code.trim()} onclick={importFromCode}>
      {importing ? "Importing…" : "Import"}
    </button>
  {/snippet}
</Modal>

<style>
  .page {
    /* Fill the viewport so right-clicking the empty area still opens the menu. */
    min-height: 100%;
  }
  .hero-actions {
    display: flex;
    gap: 10px;
    flex-shrink: 0;
  }
  .toast {
    position: fixed;
    bottom: 100px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 300;
    padding: 10px 16px;
    background: var(--bg-raised);
    border: 2px solid var(--accent);
    color: var(--accent);
    font-size: 13px;
    box-shadow: var(--shadow-md);
  }
  .toast.err {
    border-color: var(--danger);
    color: var(--danger);
  }
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
