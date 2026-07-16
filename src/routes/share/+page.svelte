<script lang="ts">
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "$lib/components/Icon.svelte";
  import { api } from "$lib/api";
  import { streamerApi } from "$lib/streamerApi";
  import { recordImport } from "$lib/importedFrom";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import type { ImportResult, StreamerCard } from "$lib/types";

  const online = streamerApi.configured();

  let fileInput = $state<HTMLInputElement>();
  let importing = $state(false);
  let progress = $state<{ phase: string; current: number; total: number } | null>(
    null
  );
  let result = $state<ImportResult | null>(null);
  let error = $state<string | null>(null);

  // --- Online: search + share code ---
  let query = $state("");
  let debounced = $state("");
  let results = $state<StreamerCard[]>([]);
  let searching = $state(false);
  let code = $state("");
  let codeBusy = $state(false);
  let codeError = $state<string | null>(null);

  $effect(() => {
    const q = query;
    const t = setTimeout(() => (debounced = q), 300);
    return () => clearTimeout(t);
  });
  $effect(() => {
    if (!online) return;
    const q = debounced.trim();
    if (!q) {
      results = [];
      return;
    }
    searching = true;
    streamerApi
      .search(q)
      .then((r) => (results = r))
      .catch(() => (results = []))
      .finally(() => (searching = false));
  });

  async function importByCode() {
    const c = code.trim();
    if (!c || codeBusy) return;
    codeBusy = true;
    codeError = null;
    error = null;
    result = null;
    try {
      const { snapshotId, streamerHandle } = await streamerApi.resolveCode(c);
      importing = true;
      result = await streamerApi.importSnapshot(snapshotId);
      recordImport(result.instance.id, {
        handle: streamerHandle,
        snapshotId,
        importedAt: Date.now(),
      });
      await instancesStore.refresh();
    } catch (e) {
      codeError = String(e);
    } finally {
      codeBusy = false;
      importing = false;
      progress = null;
    }
  }

  // Live import progress from the backend (file + snapshot imports).
  $effect(() => {
    let un: (() => void) | undefined;
    listen<{ phase: string; current: number; total: number }>(
      "snapshot-progress",
      (e) => (progress = e.payload)
    ).then((u) => (un = u));
    return () => un?.();
  });

  const pct = $derived(
    progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : null
  );

  async function onFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    importing = true;
    error = null;
    result = null;
    progress = null;
    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      result = await api.importSetup(bytes);
      await instancesStore.refresh();
    } catch (err) {
      error = String(err);
    } finally {
      importing = false;
      progress = null;
    }
  }
</script>

<div class="page">
  <header class="hero">
    <div>
      <h1>Streamers</h1>
      <p>Import a creator's exact setup — mods, packs, and configs — in one click.</p>
    </div>
    <button class="btn ghost" onclick={() => goto("/share/creator")}>
      <Icon name="video" size={15} /> I'm a streamer
    </button>
  </header>

  <section class="discover">
    <div class="disc-row">
      <div class="search" class:disabled={!online}>
        <Icon name="search" size={16} />
        <input
          placeholder={online ? "Search streamers…" : "Search streamers…"}
          bind:value={query}
          disabled={!online}
        />
        {#if searching}<span class="mini-spin"></span>{/if}
      </div>
      <div class="code" class:disabled={!online}>
        <input
          placeholder="Paste a share code…"
          bind:value={code}
          disabled={!online || codeBusy}
          onkeydown={(e) => e.key === "Enter" && importByCode()}
        />
        {#if online}
          <button class="btn primary sm" onclick={importByCode} disabled={codeBusy || !code.trim()}>
            {codeBusy ? "…" : "Go"}
          </button>
        {:else}
          <span class="badge">Soon</span>
        {/if}
      </div>
    </div>

    {#if !online}
      <p class="soon-note">
        <Icon name="video" size={13} /> Streamer profiles, live status, and share codes
        arrive with the hosted service. For now you can import a setup file below.
      </p>
    {:else if codeError}
      <p class="error">{codeError}</p>
    {/if}

    {#if online && results.length}
      <ul class="results">
        {#each results as s (s.handle)}
          <li>
            <button class="scard" onclick={() => goto(`/share/${s.handle}`)}>
              <span class="avatar"><Icon name="user" size={18} /></span>
              <span class="scard-body">
                <span class="name">{s.displayName}</span>
                <span class="handle">@{s.handle} · {s.platform}</span>
              </span>
              {#if s.isLive}<span class="live">LIVE</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <!-- Works today: import a snapshot file. -->
  <section class="import-card">
    {#if result}
      <div class="result">
        <div class="result-head">
          <Icon name="check" size={18} />
          <h3>Imported “{result.instance.name}”</h3>
        </div>
        <p>{result.installed} item{result.installed === 1 ? "" : "s"} installed{result.skipped.length ? `, ${result.skipped.length} skipped` : ""}.</p>
        {#if result.skipped.length}
          <details class="skipped">
            <summary>{result.skipped.length} couldn't be downloaded</summary>
            <ul>
              {#each result.skipped as s}
                <li>{s}</li>
              {/each}
            </ul>
            <small>Usually CurseForge mods whose authors opted out of API downloads.</small>
          </details>
        {/if}
        <div class="result-actions">
          <button class="btn primary" onclick={() => goto(`/instance/${result?.instance.id}`)}>
            Open instance
          </button>
          <button class="btn ghost" onclick={() => (result = null)}>Import another</button>
        </div>
      </div>
    {:else if importing}
      <div class="importing">
        <span class="spinner"></span>
        <div>
          <strong>Importing setup…</strong>
          <div class="prog-label">
            {progress?.phase === "installing"
              ? `Downloading content ${progress.current}/${progress.total}`
              : "Reading pack…"}
          </div>
        </div>
        {#if pct !== null}
          <div class="bar"><div class="fill" style="width:{pct}%"></div></div>
        {/if}
      </div>
    {:else}
      <button class="drop" onclick={() => fileInput?.click()}>
        <Icon name="download" size={28} />
        <strong>Import a setup file</strong>
        <span>Open a <code>.drakepack</code> or Modrinth <code>.mrpack</code></span>
      </button>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
    <input
      bind:this={fileInput}
      type="file"
      accept=".drakepack,.mrpack,application/zip"
      style="display:none"
      onchange={onFile}
    />
  </section>

  <p class="tip">
    <Icon name="share" size={13} /> To export your own setup, right-click any instance
    on the Home page and choose <strong>Export setup…</strong>
  </p>
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 900px;
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
  .discover {
    margin-bottom: 22px;
  }
  .disc-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .search,
  .code {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    padding: 0 12px;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
  }
  .search {
    flex: 1;
    min-width: 220px;
  }
  .code {
    position: relative;
  }
  .search :global(.hn) {
    color: var(--text-muted);
  }
  .search input,
  .code input {
    flex: 1;
    padding: 11px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 13px;
  }
  .search input:focus,
  .code input:focus {
    outline: none;
  }
  .disabled {
    opacity: 0.55;
  }
  .btn.sm {
    padding: 5px 12px;
    font-size: 12px;
  }
  .badge {
    font-family: var(--font-pixel);
    font-size: 10px;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 1px 5px;
    text-transform: uppercase;
  }
  .soon-note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 2px 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .results {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .scard {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    text-align: left;
    transition: border-color 0.1s, background 0.1s;
  }
  .scard:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .avatar {
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-muted);
  }
  .scard-body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .scard .name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .scard .handle {
    font-size: 12px;
    color: var(--text-muted);
  }
  .live {
    font-family: var(--font-pixel);
    font-size: 10px;
    color: #fff;
    background: var(--danger);
    padding: 2px 6px;
  }
  .mini-spin {
    width: 13px;
    height: 13px;
    border: 2px solid rgba(255, 255, 255, 0.25);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  .import-card {
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    padding: 22px;
  }
  .drop {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 34px 18px;
    background: var(--bg-input);
    border: 2px dashed var(--border);
    color: var(--text-secondary);
    transition: border-color 0.12s, color 0.12s;
  }
  .drop:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .drop :global(.hn) {
    color: var(--accent);
  }
  .drop strong {
    font-size: 15px;
    color: var(--text);
  }
  .drop span {
    font-size: 12.5px;
  }
  .drop code {
    background: var(--bg-app);
    padding: 1px 5px;
    color: var(--accent);
  }
  .importing {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 14px;
  }
  .importing .bar {
    grid-column: 1 / -1;
  }
  .prog-label {
    font-size: 12.5px;
    color: var(--text-muted);
    margin-top: 2px;
  }
  .bar {
    height: 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s steps(16);
  }
  .spinner {
    width: 22px;
    height: 22px;
    border: 3px solid rgba(255, 255, 255, 0.25);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .result-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent);
  }
  .result-head h3 {
    font-size: 16px;
  }
  .result p {
    color: var(--text-secondary);
    margin: 8px 0;
  }
  .skipped {
    margin: 8px 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .skipped summary {
    cursor: pointer;
    color: var(--warning);
  }
  .skipped ul {
    margin: 8px 0 4px;
    padding-left: 18px;
  }
  .skipped small {
    color: var(--text-muted);
  }
  .result-actions {
    display: flex;
    gap: 10px;
    margin-top: 14px;
  }
  .error {
    margin: 12px 0 0;
    color: var(--danger);
    font-size: 13px;
  }
  .tip {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 18px 2px 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }
</style>
