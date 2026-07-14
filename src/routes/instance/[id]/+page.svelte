<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { MOD_LOADERS } from "$lib/types";
  import Icon from "$lib/components/Icon.svelte";
  import InstanceIcon from "$lib/components/InstanceIcon.svelte";
  import Modal from "$lib/components/Modal.svelte";

  const id = $derived($page.params.id ?? "");
  const instance = $derived(instancesStore.get(id));

  const tabs = ["Content", "Worlds", "Screenshots", "Logs", "Settings"];
  let activeTab = $state("Content");

  // --- Launch state ---
  const runtime = $derived(launchStore.get(id));
  const launchBusy = $derived(launchStore.isBusy(id));
  const launchRunning = $derived(launchStore.isRunning(id));

  const stageLabels: Record<string, string> = {
    libraries: "Downloading libraries",
    assets: "Downloading assets",
    java: "Downloading Java runtime",
  };
  const progressLabel = $derived(
    runtime.message ?? stageLabels[runtime.stage] ?? "Working…"
  );
  const progressPct = $derived(
    runtime.total > 0 ? Math.round((runtime.current / runtime.total) * 100) : null
  );

  let renameOpen = $state(false);
  let deleteOpen = $state(false);
  let renameValue = $state("");
  let busy = $state(false);

  const loaderLabel = $derived(
    instance
      ? MOD_LOADERS.find((l) => l.value === instance.loader)?.label ??
          instance.loader
      : ""
  );

  function openRename() {
    if (!instance) return;
    renameValue = instance.name;
    renameOpen = true;
  }

  async function confirmRename() {
    if (!instance || !renameValue.trim()) return;
    busy = true;
    try {
      await instancesStore.update(instance.id, { name: renameValue.trim() });
      renameOpen = false;
    } finally {
      busy = false;
    }
  }

  async function confirmDelete() {
    if (!instance) return;
    busy = true;
    try {
      await instancesStore.remove(instance.id);
      deleteOpen = false;
      goto("/library");
    } finally {
      busy = false;
    }
  }

  function fmtDate(iso: string | null): string {
    if (!iso) return "Never";
    return new Date(iso).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  function fmtPlaytime(sec: number): string {
    if (sec < 60) return "< 1 min";
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}h ${m}m` : `${m}m`;
  }
</script>

{#if !instance}
  <div class="missing">
    {#if instancesStore.loaded}
      <p>This instance no longer exists.</p>
      <button class="btn ghost" onclick={() => goto("/library")}>
        Back to library
      </button>
    {:else}
      <p class="muted">Loading…</p>
    {/if}
  </div>
{:else}
  <div class="detail">
    <div class="banner">
      <button class="back" onclick={() => goto("/library")} aria-label="Back">
        ← Library
      </button>
      <div class="header">
        <InstanceIcon {instance} size={96} />
        <div class="titles">
          <h1>{instance.name}</h1>
          <div class="badges">
            <span class="badge">{loaderLabel}</span>
            <span class="badge">{instance.mcVersion}</span>
            {#if instance.loaderVersion}
              <span class="badge subtle">{instance.loaderVersion}</span>
            {/if}
          </div>
          <div class="stats">
            <span><Icon name="clock" size={13} /> {fmtPlaytime(instance.totalPlaytimeSeconds)} played</span>
            <span>Last played {fmtDate(instance.lastPlayed)}</span>
          </div>
        </div>
        <div class="actions">
          {#if launchRunning}
            <button class="btn danger big" onclick={() => launchStore.stop(id)}>
              <Icon name="trash" size={16} /> Stop
            </button>
          {:else}
            <button
              class="btn primary big"
              disabled={launchBusy}
              onclick={() => launchStore.launch(id)}
            >
              <Icon name="play" size={16} />
              {launchBusy ? "Preparing…" : "Play"}
            </button>
          {/if}
          <button class="btn ghost" onclick={openRename} aria-label="Rename">
            <Icon name="edit" size={16} />
          </button>
        </div>
      </div>

      {#if launchBusy}
        <div class="progress">
          <div class="progress-head">
            <span>{progressLabel}</span>
            {#if progressPct !== null}<span>{progressPct}%</span>{/if}
          </div>
          <div class="bar">
            <div
              class="bar-fill"
              class:indeterminate={progressPct === null}
              style={progressPct !== null ? `width:${progressPct}%` : ""}
            ></div>
          </div>
        </div>
      {:else if runtime.state === "error"}
        <div class="launch-error">
          <strong>Launch failed.</strong>
          {runtime.message}
        </div>
      {/if}
    </div>

    <div class="tabs">
      {#each tabs as t}
        <button
          class="tab"
          class:active={activeTab === t}
          onclick={() => (activeTab = t)}
        >
          {t}
        </button>
      {/each}
    </div>

    <div class="tab-body">
      {#if activeTab === "Settings"}
        <div class="settings-tab">
          <section class="card-block">
            <h3>Instance</h3>
            <div class="row">
              <span>Name</span>
              <button class="btn ghost sm" onclick={openRename}>
                <Icon name="edit" size={14} /> Rename
              </button>
            </div>
            <div class="row">
              <span>Minecraft version</span>
              <strong>{instance.mcVersion}</strong>
            </div>
            <div class="row">
              <span>Mod loader</span>
              <strong>{loaderLabel}</strong>
            </div>
            <div class="row">
              <span>Created</span>
              <strong>{fmtDate(instance.created)}</strong>
            </div>
          </section>

          <section class="card-block danger-zone">
            <h3>Danger zone</h3>
            <div class="row">
              <span>Delete this instance and all its files.</span>
              <button class="btn danger sm" onclick={() => (deleteOpen = true)}>
                <Icon name="trash" size={14} /> Delete
              </button>
            </div>
          </section>
        </div>
      {:else if activeTab === "Logs"}
        <div class="logs">
          <div class="logs-head">
            <span class="state-pill state-{runtime.state}">{runtime.state}</span>
            <span class="logs-count">{runtime.logs.length} lines</span>
          </div>
          {#if runtime.logs.length === 0}
            <p class="muted logs-empty">
              No output yet. Press Play to launch — game logs stream here live.
            </p>
          {:else}
            <pre class="log-view">{runtime.logs.join("\n")}</pre>
          {/if}
        </div>
      {:else}
        <div class="tab-placeholder">
          <div class="mark"><Icon name="package" size={34} /></div>
          <p>{activeTab} management arrives in a later milestone.</p>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- Rename -->
<Modal title="Rename instance" open={renameOpen} onClose={() => (renameOpen = false)} width={420}>
  <label class="field-label" for="rename-input">Name</label>
  <input
    id="rename-input"
    class="input"
    bind:value={renameValue}
    onkeydown={(e) => e.key === "Enter" && confirmRename()}
  />
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (renameOpen = false)}>Cancel</button>
    <button class="btn primary" disabled={busy || !renameValue.trim()} onclick={confirmRename}>
      Save
    </button>
  {/snippet}
</Modal>

<!-- Delete -->
<Modal title="Delete instance" open={deleteOpen} onClose={() => (deleteOpen = false)} width={420}>
  <p class="confirm-text">
    Are you sure you want to delete <strong>{instance?.name}</strong>? This removes
    all of its files and cannot be undone.
  </p>
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (deleteOpen = false)}>Cancel</button>
    <button class="btn danger" disabled={busy} onclick={confirmDelete}>
      {busy ? "Deleting…" : "Delete"}
    </button>
  {/snippet}
</Modal>

<style>
  .detail {
    display: flex;
    flex-direction: column;
  }
  .missing {
    padding: 80px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .muted {
    color: var(--text-muted);
  }
  .banner {
    padding: 24px 32px 20px;
    background: linear-gradient(180deg, var(--bg-raised), var(--bg-app));
    border-bottom: 1px solid var(--border-subtle);
  }
  .back {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    padding: 4px 0;
    margin-bottom: 18px;
  }
  .back:hover {
    color: var(--text);
  }
  .header {
    display: flex;
    align-items: center;
    gap: 20px;
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .titles h1 {
    font-size: 26px;
    margin-bottom: 8px;
  }
  .badges {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-bottom: 10px;
  }
  .badge {
    padding: 3px 9px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .badge.subtle {
    color: var(--text-muted);
  }
  .stats {
    display: flex;
    gap: 16px;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .stats span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .btn.big {
    padding: 11px 24px;
    font-size: 14px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 32px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .tab {
    padding: 12px 14px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    font-size: 13.5px;
    font-weight: 500;
    margin-bottom: -1px;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .tab-body {
    padding: 28px 32px;
    max-width: 900px;
  }
  .tab-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 56px;
    color: var(--text-muted);
  }
  .mark {
    background: var(--bg-card);
    width: 72px;
    height: 72px;
    border-radius: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .settings-tab {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .card-block {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: 18px 20px;
  }
  .card-block h3 {
    font-size: 14px;
    margin-bottom: 14px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
    border-top: 1px solid var(--border-subtle);
    font-size: 13px;
    color: var(--text-secondary);
  }
  .row:first-of-type {
    border-top: none;
  }
  .danger-zone {
    border-color: rgba(255, 91, 91, 0.25);
  }
  .btn.sm {
    padding: 6px 12px;
    font-size: 12px;
  }
  .confirm-text {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  /* Launch progress */
  .progress {
    margin-top: 18px;
    max-width: 520px;
  }
  .progress-head {
    display: flex;
    justify-content: space-between;
    font-size: 12.5px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .bar {
    height: 8px;
    background: var(--bg-input);
    border-radius: 6px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 6px;
    transition: width 0.2s ease;
  }
  .bar-fill.indeterminate {
    width: 35%;
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    0% {
      margin-left: -35%;
    }
    100% {
      margin-left: 100%;
    }
  }
  .launch-error {
    margin-top: 16px;
    max-width: 640px;
    padding: 10px 14px;
    background: rgba(255, 91, 110, 0.1);
    border: 1px solid rgba(255, 91, 110, 0.3);
    border-radius: var(--radius-sm);
    color: var(--danger);
    font-size: 13px;
  }

  /* Logs */
  .logs-head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  .state-pill {
    padding: 2px 10px;
    border-radius: 20px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--bg-card);
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }
  .state-running {
    color: var(--accent);
    border-color: var(--accent);
  }
  .state-error {
    color: var(--danger);
    border-color: var(--danger);
  }
  .logs-count {
    font-size: 12px;
    color: var(--text-muted);
  }
  .logs-empty {
    padding: 24px 0;
  }
  .log-view {
    margin: 0;
    max-height: 60vh;
    overflow: auto;
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 12px 14px;
    font-family: "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
