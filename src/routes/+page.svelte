<script lang="ts">
  import { goto } from "$app/navigation";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { instanceLayout } from "$lib/stores/instanceLayout.svelte";
  import HomeGrid, { type Entry } from "$lib/components/HomeGrid.svelte";
  import FolderOverlay from "$lib/components/FolderOverlay.svelte";
  import GroupContextMenu from "$lib/components/GroupContextMenu.svelte";
  import InstanceCardSkeleton from "$lib/components/InstanceCardSkeleton.svelte";
  import NewsSection from "$lib/components/NewsSection.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import ContextMenu, { type MenuItem } from "$lib/components/ContextMenu.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { api } from "$lib/api";
  import { boardApi } from "$lib/boardApi";
  import { toast } from "$lib/stores/toast.svelte";
  import { MOD_LOADERS, type Instance, type ModLoader } from "$lib/types";
  import { CMD_K } from "$lib/platform";
  import { t } from "$lib/i18n";

  let query = $state("");
  let loaderFilter = $state<ModLoader | "all">("all");
  let arranging = $state(false);
  let openFolder = $state<string | null>(null);

  const entries = $derived.by<Entry[]>(() => {
    const map = new Map<string, Instance[]>();
    const list: Entry[] = [];
    for (const instance of filtered) {
      if (instance.group) {
        (map.get(instance.group) ?? map.set(instance.group, []).get(instance.group)!).push(instance);
      } else {
        list.push({ kind: "instance", id: instance.id, instance });
      }
    }
    for (const [name, instances] of [...map.entries()].sort((a, b) =>
      a[0].localeCompare(b[0])
    )) {
      list.push({ kind: "folder", id: `folder:${name}`, name, instances });
    }
    return list;
  });

  const online = boardApi.configured();
  let fileInput = $state<HTMLInputElement>();
  let homeMenu = $state<{ x: number; y: number } | null>(null);
  let codeOpen = $state(false);
  let code = $state("");
  let importing = $state(false);

  const hasPlayable = $derived(
    instancesStore.instances.some((instance) => instance.kind !== "server")
  );

  const menuItems = $derived<MenuItem[]>([
    ...(hasPlayable
      ? [
          {
            label: t("home.playRandom"),
            icon: "shuffle",
            onSelect: playRandom,
          } as MenuItem,
          { separator: true } as MenuItem,
        ]
      : []),
    { label: t("home.newInstance"), icon: "plus", onSelect: () => ui.openCreateInstance() },
    { separator: true },
    { label: t("home.importSetupFile"), icon: "download", onSelect: () => fileInput?.click() },
    ...(online
      ? [{ label: t("home.importFromCode"), icon: "share", onSelect: () => (codeOpen = true) }]
      : []),
  ]);

  function playRandom() {
    const clients = instancesStore.instances.filter((instance) => instance.kind !== "server");
    if (clients.length === 0) return;
    const idle = clients.filter(
      (instance) => !launchStore.isRunning(instance.id) && !launchStore.isBusy(instance.id)
    );
    const pool = idle.length > 0 ? idle : clients;
    const pick = pool[Math.floor(Math.random() * pool.length)];
    goto(`/instance/${pick.id}`);
    launchStore.launch(pick.id);
  }

  function openMenu(event: MouseEvent) {
    event.preventDefault();
    homeMenu = { x: event.clientX, y: event.clientY };
  }

  async function onImportFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    importing = true;
    try {
      const buf = await file.arrayBuffer();
      const res = await api.importSetup(Array.from(new Uint8Array(buf)));
      await instancesStore.refresh();
      goto(`/instance/${res.instance.id}`);
    } catch (err) {
      toast.error(String(err));
    } finally {
      importing = false;
    }
  }

  async function importFromCode() {
    const trimmedCode = code.trim();
    if (!trimmedCode || importing) return;
    importing = true;
    try {
      const { snapshotId } = await boardApi.resolveCode(trimmedCode);
      const res = await boardApi.importSnapshot(snapshotId);
      await instancesStore.refresh();
      codeOpen = false;
      code = "";
      goto(`/instance/${res.instance.id}`);
    } catch (err) {
      toast.error(String(err));
    } finally {
      importing = false;
    }
  }

  const filtersActive = $derived(query.trim() !== "" || loaderFilter !== "all");
  $effect(() => {
    if (filtersActive && arranging) arranging = false;
  });

  const filtered = $derived(
    instancesStore.instances.filter((instance) => {
      const matchesQuery =
        !query.trim() ||
        instance.name.toLowerCase().includes(query.trim().toLowerCase()) ||
        instance.mcVersion.includes(query.trim());
      const matchesLoader = loaderFilter === "all" || instance.loader === loaderFilter;
      return matchesQuery && matchesLoader;
    })
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="page" oncontextmenu={openMenu}>
  <header class="hero">
    <div>
      <h1>{t("home.welcomeBack")}</h1>
      <p>{t("home.subtitle")}</p>
    </div>
    <div class="hero-actions">
      <button class="cmdk" title={t("home.commandPalette")} onclick={() => ui.toggleCommandPalette()}>
        <Icon name="search" size={14} />
        <span class="cmdk-label">{t("home.search")}</span>
        <kbd>{CMD_K}</kbd>
      </button>
      <button class="btn ghost" title={t("home.importShared")} onclick={openMenu}>
        <Icon name="download" size={15} /> {t("common.import")}
      </button>
      <button class="btn primary" onclick={() => ui.openCreateInstance()}>
        <Icon name="plus" size={16} /> {t("home.newInstance")}
      </button>
    </div>
  </header>

  <NewsSection />

  {#if instancesStore.instances.length > 0}
    <div class="toolbar">
      <div class="search">
        <Icon name="search" size={16} />
        <input
          class="search-input"
          placeholder={t("home.searchInstances")}
          bind:value={query}
        />
      </div>
      <select class="select loader-filter" bind:value={loaderFilter}>
        <option value="all">{t("home.allLoaders")}</option>
        {#each MOD_LOADERS as loader}
          <option value={loader.value}>{loader.label}</option>
        {/each}
      </select>
      <button
        class="btn ghost arrange"
        class:on={arranging}
        disabled={filtersActive}
        title={filtersActive
          ? t("home.clearFiltersToRearrange")
          : t("home.dragResizeTiles")}
        onclick={() => (arranging = !arranging)}
      >
        <Icon name={arranging ? "check" : "expand"} size={15} />
        {arranging ? t("common.done") : t("home.arrange")}
      </button>
    </div>

    {#if arranging}
      <div class="arrange-hint">
        <span>
          {t("home.arrangeHint")}
          <Icon name="expand" size={12} /> {t("home.arrangeHintResize")}
        </span>
        <button class="reset" onclick={() => instanceLayout.reset()}>
          <Icon name="refresh" size={12} /> {t("home.resetLayout")}
        </button>
      </div>
    {/if}
  {/if}

  {#if instancesStore.loading && !instancesStore.loaded}
    <div class="grid">
      {#each Array(6) as _, index (index)}
        <InstanceCardSkeleton />
      {/each}
    </div>
  {:else if instancesStore.instances.length === 0}
    <div class="empty">
      <img class="empty-art" src="/empty-cactus.png" alt="" />
      <h2>{t("home.noInstancesYet")}</h2>
      <p>{t("home.createFirst")}</p>
      <button class="btn primary" onclick={() => ui.openCreateInstance()}>
        <Icon name="plus" size={16} /> {t("home.createInstance")}
      </button>
    </div>
  {:else if filtered.length === 0}
    <p class="muted">{t("home.noMatchFilters")}</p>
  {:else}
    <HomeGrid {entries} {arranging} onOpenFolder={(name) => (openFolder = name)} />
  {/if}
</div>

<FolderOverlay name={openFolder} onClose={() => (openFolder = null)} />
<GroupContextMenu onOpenFolder={(name) => (openFolder = name)} />

<input
  bind:this={fileInput}
  type="file"
  accept=".cactuspack,.drakepack,.mrpack,application/zip"
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
  <div class="toast" role="status">{t("common.importing")}</div>
{/if}

<Modal title={t("home.importFromCodeTitle")} open={codeOpen} onClose={() => (codeOpen = false)} width={380}>
  <input
    class="input"
    placeholder={t("home.pasteShareCode")}
    bind:value={code}
    onkeydown={(event) => event.key === "Enter" && importFromCode()}
  />
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (codeOpen = false)}>{t("common.cancel")}</button>
    <button class="btn primary" disabled={importing || !code.trim()} onclick={importFromCode}>
      {importing ? t("common.importing") : t("common.import")}
    </button>
  {/snippet}
</Modal>

<style>
  .page {
    min-height: 100%;
  }
  .hero-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .cmdk {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    min-width: 190px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
  }
  .cmdk:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .cmdk-label {
    flex: 1;
    text-align: left;
    font-size: 13px;
  }
  .cmdk kbd {
    font-family: var(--font-pixel), monospace;
    font-size: 10.5px;
    padding: 2px 6px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    white-space: nowrap;
  }
  @media (max-width: 620px) {
    .cmdk-label {
      display: none;
    }
    .cmdk {
      min-width: 0;
    }
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
  .empty-art {
    width: 240px;
    max-width: 60%;
    height: auto;
    margin-bottom: 4px;
    image-rendering: pixelated;
    -webkit-user-drag: none;
    user-select: none;
  }
  .empty h2 {
    font-size: 18px;
  }
  .empty p {
    margin: 0 0 8px;
    max-width: 340px;
  }
</style>
