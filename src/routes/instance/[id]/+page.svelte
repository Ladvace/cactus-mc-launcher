<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { formatDate } from "$lib/time";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { toPct } from "$lib/stores/install.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { MOD_LOADERS, type ContentItem, type Source } from "$lib/types";
  import Icon from "$lib/components/Icon.svelte";
  import InstanceIcon from "$lib/components/InstanceIcon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import ServerProperties from "$lib/components/ServerProperties.svelte";
  import ServerAddress from "$lib/components/ServerAddress.svelte";
  import ServerShare from "$lib/components/ServerShare.svelte";
  import PlayersList from "$lib/components/PlayersList.svelte";
  import WorldsList from "$lib/components/WorldsList.svelte";
  import InstanceJavaSettings from "$lib/components/InstanceJavaSettings.svelte";
  import TuneupModal from "$lib/components/TuneupModal.svelte";
  import ProgressBar from "$lib/components/ProgressBar.svelte";
  import { pickFolder } from "$lib/dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { t } from "$lib/i18n";

  function tabLabel(tab: string): string {
    const map: Record<string, string> = {
      Content: t("instance.tabContent"),
      Worlds: t("instance.tabWorlds"),
      Console: t("instance.tabConsole"),
      Players: t("instance.tabPlayers"),
      Properties: t("instance.tabProperties"),
      Settings: t("instance.tabSettings"),
      Screenshots: t("instance.tabScreenshots"),
      Logs: t("instance.tabLogs"),
    };
    return map[tab] ?? tab;
  }

  const id = $derived($page.params.id ?? "");
  const instance = $derived(instancesStore.get(id));

  const isServer = $derived(instance?.kind === "server");
  const tabs = $derived(
    isServer
      ? ["Content", "Worlds", "Console", "Players", "Properties", "Settings"]
      : ["Content", "Worlds", "Screenshots", "Logs", "Settings"]
  );
  let activeTab = $state("Content");
  $effect(() => {
    if (!tabs.includes(activeTab)) activeTab = "Content";
  });

  let command = $state("");
  let logEl = $state<HTMLPreElement>();

  async function sendCommand() {
    const trimmed = command.trim();
    if (!trimmed || !launchRunning) return;
    try {
      await api.sendServerCommand(id, trimmed);
      command = "";
    } catch (err) {
      toast.error(String(err));
    }
  }

  $effect(() => {
    void runtime.logs.length;
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
  });

  const runtime = $derived(launchStore.get(id));
  const launchBusy = $derived(launchStore.isBusy(id));
  const launchRunning = $derived(launchStore.isRunning(id));

  const stageLabels = $derived<Record<string, string>>({
    libraries: t("instance.stageLibraries"),
    assets: t("instance.stageAssets"),
    java: t("instance.stageJava"),
  });
  const progressLabel = $derived(
    runtime.message ?? stageLabels[runtime.stage] ?? t("instance.working")
  );
  const progressPct = $derived(toPct(runtime.current, runtime.total));

  let content = $state<ContentItem[]>([]);
  let contentLoading = $state(false);

  async function loadContent() {
    if (!id) return;
    contentLoading = true;
    try {
      content = await api.listContent(id);
    } catch (err) {
      toast.error(String(err));
    } finally {
      contentLoading = false;
    }
  }

  $effect(() => {
    if (activeTab === "Content" && id) loadContent();
  });

  async function toggleContent(item: ContentItem) {
    try {
      await api.setContentEnabled(id, item.versionId, !item.enabled);
      await loadContent();
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function removeContentItem(item: ContentItem) {
    try {
      await api.removeContent(id, item.versionId);
      await loadContent();
    } catch (err) {
      toast.error(String(err));
    }
  }

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
        const versions = await api.contentVersions(
          item.source as Source,
          item.projectId,
          loaderFilter,
          instance.mcVersion
        );
        if (versions.length > 0 && versions[0].id !== item.versionId) {
          found[item.versionId] = { versionId: versions[0].id, number: versions[0].versionNumber };
        }
      }
      updates = found;
    } catch (err) {
      toast.error(String(err));
    } finally {
      checkingUpdates = false;
    }
  }

  async function updateItem(item: ContentItem) {
    const update = updates[item.versionId];
    if (!update) return;
    updatingId = item.versionId;
    try {
      await api.installContent({
        instanceId: id,
        source: item.source as Source,
        versionId: update.versionId,
        projectType: item.projectType,
        title: item.title,
        iconUrl: item.iconUrl,
      });
      const next = { ...updates };
      delete next[item.versionId];
      updates = next;
      await loadContent();
    } catch (err) {
      toast.error(String(err));
    } finally {
      updatingId = null;
    }
  }

  let renameOpen = $state(false);
  let deleteOpen = $state(false);
  let tuneupOpen = $state(false);
  let renameValue = $state("");
  let busy = $state(false);

  const loaderLabel = $derived(
    instance
      ? MOD_LOADERS.find((loader) => loader.value === instance.loader)?.label ??
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

  const fmtDate = (iso: string | null) => (iso ? formatDate(iso) : t("instance.never"));

  let gameFolder = $state("");
  let movingFolder = $state(false);
  $effect(() => {
    if (id) api.instanceFolder(id).then((path) => (gameFolder = path)).catch(() => {});
  });

  async function moveGameFolder(path: string | null) {
    if (movingFolder || launchRunning) return;
    movingFolder = true;
    try {
      await api.setInstanceGameDir(id, path);
      await instancesStore.refresh();
      gameFolder = await api.instanceFolder(id);
      toast.success(path ? t("instance.filesMoved") : t("instance.movedToDefault"));
    } catch (err) {
      toast.error(String(err));
    } finally {
      movingFolder = false;
    }
  }

  async function changeGameFolder() {
    if (movingFolder || launchRunning) return;
    const folder = await pickFolder(t("instance.pickFolderTitle"));
    if (folder) moveGameFolder(folder);
  }

  async function openGameFolder() {
    try {
      await revealItemInDir(await api.instanceFolder(id));
    } catch (err) {
      toast.error(String(err));
    }
  }

  function fmtPlaytime(sec: number): string {
    if (sec < 60) return t("instance.playtimeLessThanMin");
    const hours = Math.floor(sec / 3600);
    const minutes = Math.floor((sec % 3600) / 60);
    return hours > 0
      ? t("instance.playtimeHoursMinutes", { hours, minutes })
      : t("instance.playtimeMinutes", { minutes });
  }
</script>

{#if !instance}
  <div class="missing">
    {#if instancesStore.loaded}
      <p>{t("instance.noLongerExists")}</p>
      <button class="btn ghost" onclick={() => goto("/")}>
        {t("instance.backToHome")}
      </button>
    {:else}
      <div class="loading-detail">
        <div class="loading-head">
          <span class="skeleton" style="width:64px;height:64px;border-radius:12px"></span>
          <div class="loading-lines">
            <span class="skeleton" style="width:180px;height:20px"></span>
            <span class="skeleton" style="width:120px;height:12px"></span>
          </div>
        </div>
        <span class="skeleton" style="width:100%;height:38px;border-radius:8px"></span>
        <span class="skeleton" style="width:100%;height:140px;border-radius:8px"></span>
      </div>
    {/if}
  </div>
{:else}
  <div class="detail">
    <div class="banner">
      <div class="col">
      <button class="back" onclick={() => goto("/")} aria-label={t("common.back")}>
        {t("instance.home")}
      </button>
      <div class="header">
        <InstanceIcon {instance} size={96} />
        <div class="titles">
          <h1>{instance.name}</h1>
          <div class="badges">
            {#if isServer}
              <span class="badge server-badge">{t("instance.serverBadge")}</span>
            {/if}
            <span class="badge">{loaderLabel}</span>
            <span class="badge">{instance.mcVersion}</span>
            {#if instance.loaderVersion}
              <span class="badge subtle">{instance.loaderVersion}</span>
            {/if}
          </div>
          <div class="stats">
            <span><Icon name="clock" size={13} /> {t("instance.played", { time: fmtPlaytime(instance.totalPlaytimeSeconds) })}</span>
            <span>{t("instance.lastPlayed", { date: fmtDate(instance.lastPlayed) })}</span>
          </div>
        </div>
        <div class="actions">
          {#if launchRunning}
            <button class="btn danger big" onclick={() => launchStore.stop(id)}>
              <Icon name="stop" size={16} /> {t("instance.stop")}
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
                  ? t("instance.starting")
                  : t("instance.preparing")
                : isServer
                  ? t("instance.startServer")
                  : t("instance.play")}
            </button>
          {/if}
          {#if !isServer}
            <button
              class="btn ghost"
              onclick={() => (tuneupOpen = true)}
              title={t("instance.tuneupTitle")}
              aria-label={t("instance.tuneupLabel")}
            >
              <Icon name="sparkles" size={16} />
            </button>
          {/if}
          <button class="btn ghost" onclick={openRename} aria-label={t("instance.rename")}>
            <Icon name="edit" size={16} />
          </button>
        </div>
      </div>

      {#if launchBusy}
        <div class="progress">
          <ProgressBar label={progressLabel} pct={progressPct} />
        </div>
      {:else if runtime.state === "error"}
        <div class="launch-error">
          <strong>{t("instance.launchFailed")}</strong>
          {runtime.message}
        </div>
      {/if}

      {#if isServer}
        <ServerAddress {id} />
        <ServerShare {id} />
      {/if}
      </div>
    </div>

    <div class="tabs">
      <div class="col tabs-inner">
        {#each tabs as tab}
          <button
            class="tab"
            class:active={activeTab === tab}
            onclick={() => (activeTab = tab)}
          >
            {tabLabel(tab)}
          </button>
        {/each}
      </div>
    </div>

    <div class="tab-body">
      {#if activeTab === "Settings"}
        <div class="settings-tab">
          <section class="card-block">
            <h3>{t("instance.instanceHeading")}</h3>
            <div class="row">
              <span>{t("instance.name")}</span>
              <button class="btn ghost sm" onclick={openRename}>
                <Icon name="edit" size={14} /> {t("instance.rename")}
              </button>
            </div>
            <div class="row">
              <span>{t("instance.minecraftVersion")}</span>
              <strong>{instance.mcVersion}</strong>
            </div>
            <div class="row">
              <span>{t("instance.modLoader")}</span>
              <strong>{loaderLabel}</strong>
            </div>
            <div class="row">
              <span>{t("instance.created")}</span>
              <strong>{fmtDate(instance.created)}</strong>
            </div>
          </section>

          <InstanceJavaSettings {instance} {isServer} />

          <section class="card-block">
            <h3>{t("instance.storage")}</h3>
            <div class="row col">
              <span>{t("instance.gameFolder")} <small class="muted">{t("instance.gameFolderHint")}</small></span>
              <code class="folder-path">{gameFolder || "…"}</code>
            </div>
            <div class="row">
              <span>{instance.gameDir ? t("instance.customLocation") : t("instance.defaultLocation")}</span>
              <div class="folder-actions">
                <button class="btn ghost sm" onclick={openGameFolder}>{t("instance.open")}</button>
                {#if instance.gameDir}
                  <button
                    class="btn ghost sm"
                    onclick={() => moveGameFolder(null)}
                    disabled={movingFolder || launchRunning}
                  >
                    {t("instance.reset")}
                  </button>
                {/if}
                <button
                  class="btn ghost sm"
                  onclick={changeGameFolder}
                  disabled={movingFolder || launchRunning}
                >
                  {movingFolder ? t("instance.moving") : t("instance.change")}
                </button>
              </div>
            </div>
            {#if launchRunning}
              <p class="muted running-note">{t("instance.stopToMove")}</p>
            {/if}
          </section>

          <section class="card-block danger-zone">
            <h3>{t("instance.dangerZone")}</h3>
            <div class="row">
              <span>{t("instance.deleteDescription")}</span>
              <button class="btn danger sm" onclick={() => (deleteOpen = true)}>
                <Icon name="trash" size={14} /> {t("instance.delete")}
              </button>
            </div>
          </section>
        </div>
      {:else if activeTab === "Content"}
        <div class="content-tab">
          <div class="content-head">
            <span class="muted">
              {content.length === 1
                ? t("instance.itemInstalled", { count: content.length })
                : t("instance.itemsInstalled", { count: content.length })}
            </span>
            <div class="content-head-actions">
              {#if content.length > 0}
                <button
                  class="btn ghost sm"
                  onclick={checkUpdates}
                  disabled={checkingUpdates}
                >
                  {checkingUpdates ? t("instance.checking") : t("instance.checkForUpdates")}
                </button>
              {/if}
              <button class="btn ghost sm" onclick={() => goto("/browse")}>
                <Icon name="compass" size={14} /> {t("instance.browseModrinth")}
              </button>
            </div>
          </div>

          {#if contentLoading}
            <div class="content-list">
              {#each Array(4) as _, index (index)}
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
              <img class="empty-art" src="/empty-cactus.png" alt="" />
              <p>{t("instance.noContent")}</p>
              <button class="btn primary" onclick={() => goto("/browse")}>
                <Icon name="compass" size={15} /> {t("instance.findMods")}
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
                      title={t("instance.updateTo", { version: updates[item.versionId].number })}
                    >
                      {updatingId === item.versionId ? t("instance.updating") : t("instance.update")}
                    </button>
                  {/if}
                  <button
                    class="btn ghost sm"
                    onclick={() => toggleContent(item)}
                    title={item.enabled ? t("instance.disable") : t("instance.enable")}
                  >
                    {item.enabled ? t("instance.enabled") : t("instance.disabled")}
                  </button>
                  <button
                    class="icon-remove"
                    onclick={() => removeContentItem(item)}
                    title={t("common.remove")}
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
            <span class="logs-count">{t("instance.lines", { count: runtime.logs.length })}</span>
          </div>
          {#if runtime.logs.length === 0}
            <p class="muted logs-empty">
              {isServer
                ? t("instance.noOutputServer")
                : t("instance.noOutputGame")}
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
                  ? t("instance.commandPlaceholder")
                  : t("instance.commandPlaceholderStopped")}
                bind:value={command}
                disabled={!launchRunning}
                onkeydown={(event) => event.key === "Enter" && sendCommand()}
              />
              <button
                class="btn primary sm"
                disabled={!launchRunning || !command.trim()}
                onclick={sendCommand}
              >
                {t("instance.send")}
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
          <p>{t("instance.tabPlaceholder", { tab: tabLabel(activeTab) })}</p>
        </div>
      {/if}
    </div>
  </div>
{/if}

<TuneupModal
  instanceId={id}
  open={tuneupOpen}
  onClose={() => (tuneupOpen = false)}
  onApplied={loadContent}
/>

<Modal title={t("instance.renameInstance")} open={renameOpen} onClose={() => (renameOpen = false)} width={420}>
  <label class="field-label" for="rename-input">{t("instance.name")}</label>
  <input
    id="rename-input"
    class="input"
    bind:value={renameValue}
    onkeydown={(event) => event.key === "Enter" && confirmRename()}
  />
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (renameOpen = false)}>{t("common.cancel")}</button>
    <button class="btn primary" disabled={busy || !renameValue.trim()} onclick={confirmRename}>
      {t("common.save")}
    </button>
  {/snippet}
</Modal>

<Modal title={t("instance.deleteInstance")} open={deleteOpen} onClose={() => (deleteOpen = false)} width={420}>
  <p class="confirm-text">
    {t("instance.deleteConfirmPrefix")} <strong>{instance?.name}</strong>{t("instance.deleteConfirmSuffix")}
  </p>
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (deleteOpen = false)}>{t("common.cancel")}</button>
    <button class="btn danger" disabled={busy} onclick={confirmDelete}>
      {busy ? t("instance.deleting") : t("instance.delete")}
    </button>
  {/snippet}
</Modal>

<style>
  .loading-detail {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-width: 900px;
    margin: 1.5rem auto;
    width: 100%;
  }
  .loading-head {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .loading-lines {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
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
  .col {
    max-width: 960px;
    margin: 0 auto;
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
  .tabs {
    padding: 0 32px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .tabs-inner {
    display: flex;
    gap: 2px;
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
    /* 960 column + the 32px horizontal padding on each side, so the content
       edge lines up with the header/tabs columns above. */
    max-width: 1024px;
    margin: 0 auto;
  }
  .tab-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 56px;
    color: var(--text-muted);
  }
  .content-empty .empty-art {
    width: 200px;
    max-width: 55%;
    height: auto;
    image-rendering: pixelated;
    -webkit-user-drag: none;
    user-select: none;
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
  .row.col {
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .folder-path {
    font-family: var(--font-pixel);
    font-size: 11px;
    color: var(--accent);
    word-break: break-all;
  }
  .folder-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .running-note {
    margin: 8px 0 0;
    font-size: 12px;
  }
  .danger-zone {
    border-color: rgba(255, 91, 91, 0.25);
  }
  .confirm-text {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  .progress {
    margin-top: 18px;
    max-width: 520px;
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
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
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
    font-family: var(--font-mono);
    color: var(--accent);
    font-weight: 700;
  }
  .console-input .cmd {
    flex: 1;
    padding: 10px 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .console-input .cmd:focus {
    outline: none;
  }
  .console-input .cmd:disabled {
    color: var(--text-muted);
  }

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
