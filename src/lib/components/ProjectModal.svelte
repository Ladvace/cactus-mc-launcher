<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import InstancePicker from "./InstancePicker.svelte";
  import { api } from "$lib/api";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { installStore, toPct } from "$lib/stores/install.svelte";
  import { formatCount } from "$lib/format";
  import { t } from "$lib/i18n";
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { SearchHit, ModrinthVersion, ContentItem, Source } from "$lib/types";

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch {
    }
  }

  interface Props {
    hit: SearchHit | null;
    open: boolean;
    onClose: () => void;
  }
  let { hit, open, onClose }: Props = $props();

  const isModpack = $derived(hit?.projectType === "modpack");

  let selectedInstanceId = $state("");
  let versions = $state<ModrinthVersion[]>([]);
  let loadingVersions = $state(false);
  let versionError = $state<string | null>(null);
  let installing = $state(false);
  let done = $state(false);
  let error = $state<string | null>(null);
  let installedItem = $state<ContentItem | null>(null);

  // Top file of the latest compatible version, and whether it's downloadable
  // (CurseForge returns no URL for mods whose author opted out).
  const topFile = $derived(
    versions.length > 0
      ? versions[0].files.find((file) => file.primary) ?? versions[0].files[0]
      : null
  );
  const canDownload = $derived(!!topFile && topFile.url.length > 0);

  const isInstalled = $derived(!!installedItem);
  const isUpToDate = $derived(
    !!installedItem && versions.length > 0 && installedItem.versionId === versions[0].id
  );
  const installLabel = $derived(
    installing
      ? t("browse.installing")
      : isUpToDate
        ? t("browse.reinstall")
        : isInstalled
          ? t("browse.update")
          : t("browse.install")
  );

  let mpCurrent = $state(0);
  let mpTotal = $state(0);
  let mpMessage = $state("");

  const instances = $derived(instancesStore.instances);
  const selectedInstance = $derived(
    instances.find((instance) => instance.id === selectedInstanceId)
  );

  const mpPct = $derived(toPct(mpCurrent, mpTotal));

  // Reset transient install/progress state when a different project is shown, so
  // a still-running install from a previously-opened modpack doesn't leak its
  // progress bar into this one (the modal is a single reused instance).
  let shownProjectId: string | null = null;
  $effect(() => {
    const id = hit?.projectId ?? null;
    if (id === shownProjectId) return;
    shownProjectId = id;
    installing = false;
    done = false;
    error = null;
    installedItem = null;
    mpCurrent = 0;
    mpTotal = 0;
    mpMessage = "";
  });

  $effect(() => {
    if (open && !isModpack && !selectedInstanceId && instances.length > 0) {
      selectedInstanceId = instances[0].id;
    }
  });

  $effect(() => {
    const currentHit = hit;
    if (!open || !currentHit) return;
    if (isModpack) {
      loadModpackVersion(currentHit);
    } else if (selectedInstance) {
      loadVersions(currentHit, selectedInstance.mcVersion, selectedInstance.loader);
    }
  });

  $effect(() => {
    if (!open || !isModpack) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen<{ current: number; total: number; message: string }>(
      "modpack-progress",
      (event) => {
        mpCurrent = event.payload.current;
        mpTotal = event.payload.total;
        mpMessage = event.payload.message;
      }
    )
      .then((unlistenFn) => {
        if (cancelled) unlistenFn();
        else unlisten = unlistenFn;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  async function loadModpackVersion(searchHit: SearchHit) {
    loadingVersions = true;
    versionError = null;
    done = false;
    error = null;
    try {
      versions = await api.contentVersions(searchHit.source as Source, searchHit.projectId, null, null);
      if (versions.length === 0) versionError = t("browse.noDownloadableVersion");
    } catch (err) {
      versions = [];
      versionError = String(err);
    } finally {
      loadingVersions = false;
    }
  }

  async function loadVersions(searchHit: SearchHit, mcVersion: string, loader: string) {
    loadingVersions = true;
    versionError = null;
    done = false;
    error = null;
    installedItem = null;
    try {
      const loaderFilter =
        searchHit.projectType === "mod" && loader !== "vanilla" ? loader : null;
      versions = await api.contentVersions(searchHit.source as Source, searchHit.projectId, loaderFilter, mcVersion);
      if (versions.length === 0) {
        versionError = t("browse.noCompatibleVersion", { loader, mcVersion });
      }
      if (selectedInstance) {
        const content = await api.listContent(selectedInstance.id);
        installedItem = content.find((item) => item.projectId === searchHit.projectId) ?? null;
      }
    } catch (err) {
      versions = [];
      versionError = String(err);
    } finally {
      loadingVersions = false;
    }
  }

  async function install() {
    if (!hit || !selectedInstance || versions.length === 0) return;
    installing = true;
    error = null;
    try {
      installedItem = await api.installContent({
        instanceId: selectedInstance.id,
        source: hit.source as Source,
        versionId: versions[0].id,
        projectType: hit.projectType,
        title: hit.title,
        iconUrl: hit.iconUrl,
      });
      done = true;
    } catch (err) {
      error = String(err);
    } finally {
      installing = false;
    }
  }

  async function installModpack() {
    if (!hit || versions.length === 0) return;
    installing = true;
    error = null;
    mpCurrent = 0;
    mpTotal = 0;
    mpMessage = t("browse.starting");
    try {
      const instance = await api.installModpack(
        hit.source as Source,
        versions[0].id,
        hit.iconUrl
      );
      await instancesStore.refresh();
      close();
      goto(`/instance/${instance.id}`);
    } catch (err) {
      error = String(err);
      installStore.clearPending();
    } finally {
      installing = false;
    }
  }

  function close() {
    versions = [];
    versionError = null;
    error = null;
    done = false;
    mpCurrent = 0;
    mpTotal = 0;
    onClose();
  }
</script>

<Modal title={hit?.title ?? t("browse.projectFallback")} open={open && !!hit} onClose={close} width={640}>
  {#if hit}
    <div class="head">
      {#if hit.iconUrl}
        <img class="icon" src={hit.iconUrl} alt={hit.title} />
      {:else}
        <div class="icon placeholder"><Icon name="package" size={28} /></div>
      {/if}
      <div class="meta">
        <h3>{hit.title}</h3>
        <p class="by">{t("browse.byAuthor", { author: hit.author })}</p>
        <div class="stats">
          <span><Icon name="package" size={13} /> {t("browse.downloads", { count: formatCount(hit.downloads) })}</span>
          <span class="type">{hit.projectType}</span>
        </div>
      </div>
    </div>

    <p class="desc">{hit.description}</p>

    <div class="install-box">
      {#if isModpack}
        <p class="mp-note">
          {t("browse.installsAsA")}<strong>{t("browse.newInstance")}</strong>{#if versions.length > 0}{versions[0].gameVersions[0] ? t("browse.mpVersionMc", { version: versions[0].versionNumber, mc: versions[0].gameVersions[0] }) : t("browse.mpVersion", { version: versions[0].versionNumber })}{/if}.
        </p>
        {#if loadingVersions}
          <span class="muted">{t("browse.findingLatestVersion")}</span>
        {:else if versionError}
          <span class="warn">{versionError}</span>
        {/if}
        {#if installing}
          <div class="mp-progress">
            <div class="progress-head">
              <span>{mpMessage || t("browse.installing")}</span>
              {#if mpPct !== null}<span>{mpPct}%</span>{/if}
            </div>
            <div class="bar">
              <div
                class="bar-fill"
                class:indeterminate={mpPct === null}
                style={mpPct !== null ? `width:${mpPct}%` : ""}
              ></div>
            </div>
          </div>
        {/if}
      {:else if instances.length === 0}
        <p class="muted">{t("browse.createInstanceFirst")}</p>
      {:else}
        <span class="field-label">{t("browse.installTo")}</span>
        <InstancePicker bind:value={selectedInstanceId} />
        <div class="version-line">
          {#if loadingVersions}
            <span class="muted">{t("browse.findingCompatibleVersion")}</span>
          {:else if versionError}
            <span class="warn">{versionError}</span>
          {:else if versions.length > 0}
            <span class="muted">
              {t("browse.latestMatch")}<strong>{versions[0].versionNumber}</strong>
            </span>
            {#if isUpToDate}
              <span class="pill ok">✓ {t("browse.installed")}</span>
            {:else if isInstalled}
              <span class="pill upd">{t("browse.updateAvailable")}</span>
            {/if}
          {/if}
        </div>
        {#if versions.length > 0 && !canDownload}
          <p class="cf-optout">
            {t("browse.cfOptout")}<button
              class="linklike"
              onclick={() =>
                openLink(
                  `https://www.curseforge.com/minecraft/search?search=${encodeURIComponent(hit.title)}`
                )}
            >{t("browse.curseforgePage")}</button>.
          </p>
        {/if}
      {/if}
    </div>

    {#if error}<p class="err">{error}</p>{/if}
  {/if}

  {#snippet footer()}
    <button class="btn ghost" onclick={close} disabled={installing}>{t("common.close")}</button>
    {#if isModpack}
      <button
        class="btn primary"
        disabled={installing || loadingVersions || versions.length === 0}
        onclick={installModpack}
      >
        {installing ? t("browse.installing") : t("browse.installModpack")}
      </button>
    {:else if done}
      <span class="ok-pill">✓ {isUpToDate ? t("browse.installed") : t("common.done")}</span>
    {:else}
      <button
        class="btn primary"
        disabled={installing || loadingVersions || versions.length === 0 || !canDownload}
        onclick={install}
      >
        {installLabel}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .head {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .icon {
    width: 64px;
    height: 64px;
    border-radius: 0;
    border: 2px solid rgba(0, 0, 0, 0.3);
    object-fit: cover;
    background: var(--bg-card);
    flex-shrink: 0;
    image-rendering: pixelated;
  }
  .icon.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .meta h3 {
    font-size: 17px;
  }
  .by {
    margin: 3px 0 8px;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .stats {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .stats span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .type {
    text-transform: capitalize;
    background: var(--bg-card);
    padding: 2px 8px;
    border-radius: 20px;
    color: var(--text-muted);
  }
  .desc {
    margin: 16px 0;
    color: var(--text-secondary);
    line-height: 1.6;
    font-size: 13.5px;
  }
  .install-box {
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
    padding: 14px;
  }
  .mp-note {
    margin: 0 0 8px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .version-line {
    margin-top: 10px;
    font-size: 12.5px;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .pill {
    padding: 3px 9px;
    border-radius: 0;
    font-family: var(--font-pixel);
    font-size: 11px;
    font-weight: 600;
    border: 2px solid;
  }
  .pill.ok {
    color: var(--accent);
    border-color: var(--accent);
  }
  .pill.upd {
    color: var(--warning);
    border-color: var(--warning);
  }
  .muted {
    color: var(--text-muted);
  }
  .warn {
    color: var(--warning);
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
  }
  .cf-optout {
    margin: 10px 0 0;
    font-size: 12.5px;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .linklike {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }
  .err {
    color: var(--danger);
    font-size: 13px;
    margin: 10px 0 0;
  }
  .ok-pill {
    color: var(--accent);
    font-weight: 600;
    font-size: 13px;
    align-self: center;
  }
  .mp-progress {
    margin-top: 12px;
  }
  .progress-head {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
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
</style>
