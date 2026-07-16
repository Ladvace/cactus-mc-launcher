<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { MOD_LOADERS, type ContentItem, type Source } from "$lib/types";
  import Icon from "$lib/components/Icon.svelte";
  import InstanceIcon from "$lib/components/InstanceIcon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import ServerProperties from "$lib/components/ServerProperties.svelte";
  import ServerAddress from "$lib/components/ServerAddress.svelte";
  import PlayersList from "$lib/components/PlayersList.svelte";
  import WorldsList from "$lib/components/WorldsList.svelte";

  const id = $derived($page.params.id ?? "");
  const instance = $derived(instancesStore.get(id));

  const isServer = $derived(instance?.kind === "server");
  const tabs = $derived(
    isServer
      ? ["Content", "Worlds", "Console", "Players", "Properties", "Settings"]
      : ["Content", "Worlds", "Screenshots", "Logs", "Settings"]
  );
  let activeTab = $state("Content");
  // Keep the active tab valid when switching between client/server instances.
  $effect(() => {
    if (!tabs.includes(activeTab)) activeTab = "Content";
  });

  // --- Server console input ---
  let command = $state("");
  let logEl = $state<HTMLPreElement>();

  async function sendCommand() {
    const c = command.trim();
    if (!c || !launchRunning) return;
    try {
      await api.sendServerCommand(id, c);
      command = "";
    } catch (e) {
      toast.error(String(e));
    }
  }

  // Auto-scroll the console to the newest line.
  $effect(() => {
    void runtime.logs.length;
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
  });

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

  // --- Installed content ---
  let content = $state<ContentItem[]>([]);
  let contentLoading = $state(false);

  async function loadContent() {
    if (!id) return;
    contentLoading = true;
    try {
      content = await api.listContent(id);
    } finally {
      contentLoading = false;
    }
  }

  $effect(() => {
    if (activeTab === "Content" && id) loadContent();
  });

  async function toggleContent(item: ContentItem) {
    await api.setContentEnabled(id, item.versionId, !item.enabled);
    await loadContent();
  }

  async function removeContentItem(item: ContentItem) {
    await api.removeContent(id, item.versionId);
    await loadContent();
  }

  // Update checking: map installed versionId -> available newer version.
  let updates = $state<Record<string, { versionId: string; number: string }>>({});
  let checkingUpdates = $state(false);
  let updatingId = $state<string | null>(null);

  async function checkUpdates() {
    if (!instance) return;
    checkingUpdates = true;
    updates = {};
    try {
      const found: Record<string, { versionId: string; number: string }> = {};
      for (const item of content) {
        if (!item.projectId) continue;
        const loaderFilter =
          item.projectType === "mod" && instance.loader !== "vanilla"
            ? instance.loader
            : null;
        const vs = await api.contentVersions(
          item.source as Source,
          item.projectId,
          loaderFilter,
          instance.mcVersion
        );
        if (vs.length > 0 && vs[0].id !== item.versionId) {
          found[item.versionId] = { versionId: vs[0].id, number: vs[0].versionNumber };
        }
      }
      updates = found;
    } finally {
      checkingUpdates = false;
    }
  }

  async function updateItem(item: ContentItem) {
    const upd = updates[item.versionId];
    if (!upd) return;
    updatingId = item.versionId;
    try {
      await api.installContent({
        instanceId: id,
        source: item.source as Source,
        versionId: upd.versionId,
        projectType: item.projectType,
        title: item.title,
        iconUrl: item.iconUrl,
      });
      const next = { ...updates };
      delete next[item.versionId];
      updates = next;
      await loadContent();
    } finally {
      updatingId = null;
    }
  }

  let renameOpen = $state(false);
  let deleteOpen = $state(false);
  let renameValue = $state("");
  let busy = $state(false);

  // --- Per-server memory (max heap, MB). null = use the global setting. ---
  let serverMem = $state<number | null>(null);
  let memInstanceId = "";
  let savingMem = $state(false);
  $effect(() => {
    if (instance && instance.id !== memInstanceId) {
      memInstanceId = instance.id;
      serverMem = instance.serverMemoryMb;
    }
  });

  async function saveServerMem(value: number | null) {
    if (!instance) return;
    savingMem = true;
    try {
      await instancesStore.update(instance.id, {
        serverMemoryMb: value && value > 0 ? value : 0,
      });
      serverMem = value && value > 0 ? value : null;
      toast.success(
        value && value > 0
          ? `Server memory set to ${(value / 1024).toFixed(value % 1024 ? 1 : 0)} GB.`
          : "Using the global memory setting."
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      savingMem = false;
    }
  }

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
      goto("/");
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
      <button class="btn ghost" onclick={() => goto("/")}>
        Back to home
      </button>
    {:else}
      <p class="muted">Loading…</p>
    {/if}
  </div>
{:else}
  <div class="detail">
    <div class="banner">
      <button class="back" onclick={() => goto("/")} aria-label="Back">
        ← Home
      </button>
      <div class="header">
        <InstanceIcon {instance} size={96} />
        <div class="titles">
          <h1>{instance.name}</h1>
          <div class="badges">
            {#if isServer}
              <span class="badge server-badge">Server</span>
            {/if}
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
              <Icon name="stop" size={16} /> Stop
            </button>
          {:else}
            <button
              class="btn primary big"
              disabled={launchBusy}
              onclick={() => launchStore.launch(id)}
            >
              <Icon name="play" size={16} />
              {launchBusy
                ? isServer
                  ? "Starting…"
                  : "Preparing…"
                : isServer
                  ? "Start server"
                  : "Play"}
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

      {#if isServer}
        <ServerAddress {id} />
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

          {#if isServer}
            <section class="card-block">
              <h3>Server memory</h3>
              <p class="block-hint">
                Max heap for this server. Leave on the global default unless the
                pack needs more.
              </p>
              <div class="mem-presets">
                {#each [2048, 4096, 6144, 8192] as mb (mb)}
                  <button
                    class="mem-chip"
                    class:active={serverMem === mb}
                    disabled={savingMem}
                    onclick={() => saveServerMem(mb)}
                  >
                    {mb / 1024} GB
                  </button>
                {/each}
                <button
                  class="mem-chip"
                  class:active={!serverMem}
                  disabled={savingMem}
                  onclick={() => saveServerMem(null)}
                >
                  Global default
                </button>
              </div>
              <div class="mem-custom">
                <input
                  class="input"
                  type="number"
                  min="512"
                  step="512"
                  placeholder="Custom (MB)"
                  bind:value={serverMem}
                />
                <button
                  class="btn ghost sm"
                  disabled={savingMem}
                  onclick={() => saveServerMem(serverMem)}
                >
                  Save
                </button>
              </div>
            </section>
          {/if}

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
      {:else if activeTab === "Content"}
        <div class="content-tab">
          <div class="content-head">
            <span class="muted">
              {content.length} item{content.length === 1 ? "" : "s"} installed
            </span>
            <div class="content-head-actions">
              {#if content.length > 0}
                <button
                  class="btn ghost sm"
                  onclick={checkUpdates}
                  disabled={checkingUpdates}
                >
                  {checkingUpdates ? "Checking…" : "Check for updates"}
                </button>
              {/if}
              <button class="btn ghost sm" onclick={() => goto("/browse")}>
                <Icon name="compass" size={14} /> Browse Modrinth
              </button>
            </div>
          </div>

          {#if contentLoading}
            <div class="content-list">
              {#each Array(4) as _, i (i)}
                <div class="content-row">
                  <span class="skeleton" style="width:36px;height:36px"></span>
                  <div class="content-info">
                    <span
                      class="skeleton"
                      style="width:38%;height:13px;margin-bottom:5px"
                    ></span>
                    <span class="skeleton" style="width:62%;height:10px"></span>
                  </div>
                </div>
              {/each}
            </div>
          {:else if content.length === 0}
            <div class="content-empty">
              <div class="mark"><Icon name="package" size={30} /></div>
              <p>No content installed yet.</p>
              <button class="btn primary" onclick={() => goto("/browse")}>
                <Icon name="compass" size={15} /> Find mods on Modrinth
              </button>
            </div>
          {:else}
            <div class="content-list">
              {#each content as item (item.versionId)}
                <div class="content-row" class:disabled={!item.enabled}>
                  {#if item.iconUrl}
                    <img class="content-icon" src={item.iconUrl} alt={item.title} />
                  {:else}
                    <div class="content-icon ph"><Icon name="package" size={16} /></div>
                  {/if}
                  <div class="content-info">
                    <span class="content-title">{item.title}</span>
                    <span class="content-sub">{item.projectType} · {item.fileName}</span>
                  </div>
                  {#if updates[item.versionId]}
                    <button
                      class="btn primary sm"
                      onclick={() => updateItem(item)}
                      disabled={updatingId === item.versionId}
                      title={`Update to ${updates[item.versionId].number}`}
                    >
                      {updatingId === item.versionId ? "Updating…" : "Update"}
                    </button>
                  {/if}
                  <button
                    class="btn ghost sm"
                    onclick={() => toggleContent(item)}
                    title={item.enabled ? "Disable" : "Enable"}
                  >
                    {item.enabled ? "Enabled" : "Disabled"}
                  </button>
                  <button
                    class="icon-remove"
                    onclick={() => removeContentItem(item)}
                    title="Remove"
                  >
                    <Icon name="trash" size={15} />
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {:else if activeTab === "Logs" || activeTab === "Console"}
        <div class="logs">
          <div class="logs-head">
            <span class="state-pill state-{runtime.state}">{runtime.state}</span>
            <span class="logs-count">{runtime.logs.length} lines</span>
          </div>
          {#if runtime.logs.length === 0}
            <p class="muted logs-empty">
              {isServer
                ? "No output yet. Start the server — console output streams here live."
                : "No output yet. Press Play to launch — game logs stream here live."}
            </p>
          {:else}
            <pre class="log-view" bind:this={logEl}>{runtime.logs.join("\n")}</pre>
          {/if}
          {#if isServer}
            <div class="console-input">
              <span class="prompt">&gt;</span>
              <input
                class="cmd"
                placeholder={launchRunning
                  ? "Type a command (e.g. say hello, op <player>, whitelist add <player>)…"
                  : "Start the server to send commands"}
                bind:value={command}
                disabled={!launchRunning}
                onkeydown={(e) => e.key === "Enter" && sendCommand()}
              />
              <button
                class="btn primary sm"
                disabled={!launchRunning || !command.trim()}
                onclick={sendCommand}
              >
                Send
              </button>
            </div>
          {/if}
        </div>
      {:else if activeTab === "Properties"}
        <ServerProperties {id} running={launchRunning} />
      {:else if activeTab === "Players"}
        <PlayersList {id} running={launchRunning} />
      {:else if activeTab === "Worlds"}
        <WorldsList {id} running={launchRunning} />
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
    padding: 4px 10px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: 0;
    font-family: var(--font-pixel);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .badge.subtle {
    color: var(--text-muted);
  }
  .badge.server-badge {
    color: var(--bg-app);
    background: var(--accent);
    border-color: var(--accent);
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
    border-bottom: 3px solid transparent;
    color: var(--text-secondary);
    font-family: var(--font-pixel);
    font-size: 14px;
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
    border-radius: 0;
    border: 2px solid var(--border);
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
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    padding: 18px 20px;
  }
  .card-block h3 {
    font-size: 14px;
    margin-bottom: 14px;
  }
  .block-hint {
    margin: -6px 0 12px;
    font-size: 12.5px;
    color: var(--text-muted);
    max-width: 60ch;
  }
  .mem-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }
  .mem-chip {
    padding: 7px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 600;
    transition: all 0.12s;
  }
  .mem-chip:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
  }
  .mem-chip.active {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .mem-custom {
    display: flex;
    gap: 8px;
    max-width: 320px;
  }
  .mem-custom .input {
    flex: 1;
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
    height: 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    overflow: hidden;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.3);
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    background-image: repeating-linear-gradient(
      90deg,
      rgba(0, 0, 0, 0.18) 0 2px,
      transparent 2px 8px
    );
    transition: width 0.2s steps(16);
  }
  .bar-fill.indeterminate {
    width: 35%;
    animation: slide 1.1s steps(8) infinite;
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
    padding: 3px 10px;
    border-radius: 0;
    font-family: var(--font-pixel);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--bg-card);
    color: var(--text-secondary);
    border: 2px solid var(--border);
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
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.3);
    padding: 12px 14px;
    font-family: "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
  /* Server console command bar */
  .console-input {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    padding: 0 10px;
  }
  .console-input .prompt {
    font-family: "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
    color: var(--accent);
    font-weight: 700;
  }
  .console-input .cmd {
    flex: 1;
    padding: 10px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-family: "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
    font-size: 12.5px;
  }
  .console-input .cmd:focus {
    outline: none;
  }
  .console-input .cmd:disabled {
    color: var(--text-muted);
  }

  /* Content tab */
  .content-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }
  .content-head-actions {
    display: flex;
    gap: 8px;
  }
  .content-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 48px;
    color: var(--text-secondary);
  }
  .content-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .content-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: 0;
  }
  .content-row.disabled {
    opacity: 0.55;
  }
  .content-icon {
    width: 36px;
    height: 36px;
    border-radius: 0;
    border: 2px solid rgba(0, 0, 0, 0.3);
    object-fit: cover;
    background: var(--bg-input);
    flex-shrink: 0;
    image-rendering: pixelated;
  }
  .content-icon.ph {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .content-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .content-title {
    font-weight: 600;
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .content-sub {
    font-size: 11.5px;
    color: var(--text-muted);
    text-transform: capitalize;
  }
  .icon-remove {
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 7px;
    border-radius: var(--radius-sm);
    display: flex;
  }
  .icon-remove:hover {
    background: rgba(255, 91, 110, 0.12);
    color: var(--danger);
  }
</style>
