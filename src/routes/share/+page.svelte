<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "$lib/components/Icon.svelte";
  import BoardView from "$lib/components/BoardView.svelte";
  import PresencePanel from "$lib/components/PresencePanel.svelte";
  import { api } from "$lib/api";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { followedBoards } from "$lib/stores/followedBoards.svelte";
  import { recordImport } from "$lib/importedFrom";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import type { BoardCard, ImportResult } from "$lib/types";

  const online = boardApi.configured();

  let hasBoard = $state(false);
  $effect(() => {
    const token = boardAuth.token;
    if (!online || !token) return;
    boardApi
      .myBoards(token)
      .then((boards) => (hasBoard = boards.length > 0))
      .catch(() => {});
  });

  let active = $state<"discover" | string>("discover");
  const BUILTIN = ["discover", "play"];
  const tabs = $derived(
    !BUILTIN.includes(active) && !followedBoards.handles.includes(active)
      ? [...followedBoards.handles, active]
      : followedBoards.handles
  );

  let query = $state("");
  let debounced = $state("");
  let results = $state<BoardCard[]>([]);
  let searching = $state(false);

  $effect(() => {
    const currentQuery = query;
    const timer = setTimeout(() => (debounced = currentQuery), 300);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    if (!online) return;
    const trimmed = debounced.trim();
    if (!trimmed) {
      results = [];
      return;
    }
    searching = true;
    boardApi
      .search(trimmed)
      .then((found) => (results = found))
      .catch(() => (results = []))
      .finally(() => (searching = false));
  });

  let fileInput = $state<HTMLInputElement>();
  let importing = $state(false);
  let result = $state<ImportResult | null>(null);
  let code = $state("");
  let codeBusy = $state(false);

  async function onFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    importing = true;
    result = null;
    try {
      const buf = await file.arrayBuffer();
      result = await api.importSetup(Array.from(new Uint8Array(buf)));
      await instancesStore.refresh();
    } catch (err) {
      toast.error(String(err));
    } finally {
      importing = false;
    }
  }

  async function importByCode() {
    const trimmedCode = code.trim();
    if (!trimmedCode || codeBusy) return;
    codeBusy = true;
    result = null;
    try {
      const { snapshotId } = await boardApi.resolveCode(trimmedCode);
      importing = true;
      result = await boardApi.importSnapshot(snapshotId);
      recordImport(result.instance.id, {
        handle: null,
        snapshotId,
        importedAt: Date.now(),
      });
      await instancesStore.refresh();
    } catch (err) {
      toast.error(String(err));
    } finally {
      codeBusy = false;
      importing = false;
    }
  }
</script>

<div class="page">
  <header class="hero">
    <div>
      <h1>Community</h1>
      <p>Follow creators & servers, or share and import a setup.</p>
    </div>
    <button class="btn ghost" onclick={() => goto("/share/creator")}>
      <Icon name={hasBoard ? "edit" : "plus"} size={15} />
      {hasBoard ? "Edit board" : "Create a board"}
    </button>
  </header>

  <div class="tabbar">
    <button class="tab" class:on={active === "discover"} onclick={() => (active = "discover")}>
      Discover
    </button>
    {#each tabs as handle (handle)}
      <button class="tab" class:on={active === handle} onclick={() => (active = handle)}>
        @{handle}
      </button>
    {/each}
    <button
      class="tab play"
      class:on={active === "play"}
      onclick={() => (active = "play")}
    >
      <Icon name="users" size={14} /> Play together
    </button>
  </div>

  {#if active === "play"}
    <PresencePanel />
  {:else if active !== "discover"}
    <BoardView handle={active} />
  {:else}
    {#if online}
      <div class="search">
        <Icon name="search" size={16} />
        <input placeholder="Search creators, servers…" bind:value={query} />
        {#if searching}<span class="mini-spin"></span>{/if}
      </div>
      {#if results.length}
        <ul class="results">
          {#each results as board (board.handle)}
            <li>
              <button class="bcard" onclick={() => (active = board.handle)}>
                <span class="kind">{board.kind}</span>
                <span class="bbody">
                  <span class="name">{board.displayName}</span>
                  <span class="handle">@{board.handle} · {board.ownerName}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <p class="offline"><Icon name="globe" size={13} /> Board discovery is offline in this build. You can still import a setup file below.</p>
    {/if}

    <section class="share-card">
      <h3>Share an instance</h3>
      {#if result}
        <div class="result">
          <p><Icon name="check" size={15} /> Imported “{result.instance.name}” — {result.installed} items{result.skipped.length ? `, ${result.skipped.length} skipped` : ""}.</p>
          <div class="ractions">
            <button class="btn primary sm" onclick={() => goto(`/instance/${result?.instance.id}`)}>Open</button>
            <button class="btn ghost sm" onclick={() => (result = null)}>Import another</button>
          </div>
        </div>
      {:else if importing}
        <p class="muted">Importing…</p>
      {:else}
        <div class="share-row">
          <button class="btn ghost" onclick={() => fileInput?.click()}>
            <Icon name="download" size={15} /> Import a file (.cactuspack / .mrpack)
          </button>
          {#if online}
            <div class="code">
              <input placeholder="Paste a share code…" bind:value={code} onkeydown={(event) => event.key === "Enter" && importByCode()} />
              <button class="btn primary sm" disabled={codeBusy || !code.trim()} onclick={importByCode}>Go</button>
            </div>
          {/if}
        </div>
      {/if}
      <input bind:this={fileInput} type="file" accept=".cactuspack,.drakepack,.mrpack,application/zip" style="display:none" onchange={onFile} />
    </section>

    <p class="tip"><Icon name="share" size={13} /> Export a setup from any instance's right-click menu → <strong>Export setup…</strong></p>
  {/if}
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 940px;
    margin: 0 auto;
  }
  .hero {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 18px;
  }
  .hero h1 {
    font-size: 26px;
  }
  .hero p {
    margin: 6px 0 0;
    color: var(--text-secondary);
  }
  .tabbar {
    display: flex;
    gap: 4px;
    border-bottom: 2px solid var(--border-subtle);
    margin-bottom: 18px;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .tab {
    padding: 8px 14px;
    background: transparent;
    border: none;
    border-bottom: 3px solid transparent;
    color: var(--text-secondary);
    font-family: var(--font-pixel);
    font-size: 13px;
    white-space: nowrap;
    margin-bottom: -2px;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.on {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .tab.play {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--accent-contrast);
    background: var(--accent);
    border-bottom-color: transparent;
    align-self: center;
    margin-bottom: 0;
  }
  .tab.play :global(.hn) {
    color: var(--accent-contrast);
  }
  .tab.play:hover {
    color: var(--accent-contrast);
    background: var(--accent-hover);
  }
  .tab.play.on {
    color: var(--accent-contrast);
    border-bottom-color: transparent;
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    padding: 0 12px;
    margin-bottom: 12px;
  }
  .search :global(.hn) {
    color: var(--text-muted);
  }
  .search input {
    flex: 1;
    padding: 11px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 13px;
  }
  .search input:focus {
    outline: none;
  }
  .mini-spin {
    width: 13px;
    height: 13px;
    border: 2px solid rgba(255, 255, 255, 0.25);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .results {
    list-style: none;
    margin: 0 0 18px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .bcard {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    text-align: left;
  }
  .bcard:hover {
    border-color: var(--accent);
  }
  .kind {
    font-family: var(--font-pixel);
    font-size: 9px;
    text-transform: uppercase;
    color: var(--text-muted);
    border: 1px solid var(--border);
    padding: 2px 5px;
  }
  .bbody {
    display: flex;
    flex-direction: column;
  }
  .bcard .name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .bcard .handle {
    font-size: 12px;
    color: var(--text-muted);
  }
  .offline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 12.5px;
    margin-bottom: 14px;
  }
  .share-card {
    background: var(--bg-card);
    border: 2px solid var(--border);
    padding: 18px 20px;
  }
  .share-card h3 {
    font-size: 14px;
    margin-bottom: 12px;
  }
  .share-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    align-items: center;
  }
  .code {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    padding: 0 0 0 12px;
  }
  .code input {
    padding: 9px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 13px;
  }
  .code input:focus {
    outline: none;
  }
  .result p {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--accent);
    font-size: 13px;
  }
  .ractions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .muted {
    color: var(--text-muted);
  }
  .tip {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 16px 2px 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
</style>
