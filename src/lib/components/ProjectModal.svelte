<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { formatCount } from "$lib/format";
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { SearchHit, ModrinthVersion, ContentItem, Source } from "$lib/types";

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch {
      /* ignore */
    }
  }

  interface Props {
    hit: SearchHit | null;
    open: boolean;
    onClose: () => void;
  }
  let { hit, open, onClose }: Props = $props();

  const isModpack = $derived(hit?.projectType === "modpack");
  // CurseForge modpacks use a different (non-.mrpack) format we don't handle yet.
  const modpackUnsupported = $derived(isModpack && hit?.source === "curseforge");

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
      ? versions[0].files.find((f) => f.primary) ?? versions[0].files[0]
      : null
  );
  const canDownload = $derived(!!topFile && topFile.url.length > 0);

  const isInstalled = $derived(!!installedItem);
  const isUpToDate = $derived(
    !!installedItem && versions.length > 0 && installedItem.versionId === versions[0].id
  );
  const installLabel = $derived(
    installing
      ? "Installing…"
      : isUpToDate
        ? "Reinstall"
        : isInstalled
          ? "Update"
          : "Install"
  );

  // Modpack install progress.
  let mpCurrent = $state(0);
  let mpTotal = $state(0);
  let mpMessage = $state("");

  const instances = $derived(instancesStore.instances);
  const selectedInstance = $derived(
    instances.find((i) => i.id === selectedInstanceId)
  );

  const mpPct = $derived(
    mpTotal > 0 ? Math.round((mpCurrent / mpTotal) * 100) : null
  );

  // Default the target instance (content installs only).
  $effect(() => {
    if (open && !isModpack && !selectedInstanceId && instances.length > 0) {
      selectedInstanceId = instances[0].id;
    }
  });

  // Load versions: unfiltered for modpacks, instance-compatible for content.
  $effect(() => {
    const h = hit;
    if (!open || !h) return;
    if (isModpack) {
      loadModpackVersion(h);
    } else if (selectedInstance) {
      loadVersions(h, selectedInstance.mcVersion, selectedInstance.loader);
    }
  });

  // Listen for modpack install progress while open.
  $effect(() => {
    if (!open || !isModpack) return;
    let unlisten: (() => void) | undefined;
    listen<{ current: number; total: number; message: string }>(
      "modpack-progress",
      (e) => {
        mpCurrent = e.payload.current;
        mpTotal = e.payload.total;
        mpMessage = e.payload.message;
      }
    ).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  async function loadModpackVersion(h: SearchHit) {
    loadingVersions = true;
    versionError = null;
    done = false;
    error = null;
    try {
      versions = await api.contentVersions(h.source as Source, h.projectId, null, null);
      if (versions.length === 0) versionError = "No downloadable version found.";
    } catch (e) {
      versions = [];
      versionError = String(e);
    } finally {
      loadingVersions = false;
    }
  }

  async function loadVersions(h: SearchHit, mc: string, loader: string) {
    loadingVersions = true;
    versionError = null;
    done = false;
    error = null;
    installedItem = null;
    try {
      const loaderFilter =
        h.projectType === "mod" && loader !== "vanilla" ? loader : null;
      versions = await api.contentVersions(h.source as Source, h.projectId, loaderFilter, mc);
      if (versions.length === 0) {
        versionError = `No version compatible with ${loader} ${mc}.`;
      }
      // Is this project already installed in the selected instance?
      if (selectedInstance) {
        const content = await api.listContent(selectedInstance.id);
        installedItem = content.find((c) => c.projectId === h.projectId) ?? null;
      }
    } catch (e) {
      versions = [];
      versionError = String(e);
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
    } catch (e) {
      error = String(e);
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
    mpMessage = "Starting…";
    try {
      const inst = await api.installModpack(
        hit.source as Source,
        versions[0].id,
        hit.iconUrl
      );
      await instancesStore.refresh();
      close();
      goto(`/instance/${inst.id}`);
    } catch (e) {
      error = String(e);
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

<Modal title={hit?.title ?? "Project"} open={open && !!hit} onClose={close} width={560}>
  {#if hit}
    <div class="head">
      {#if hit.iconUrl}
        <img class="icon" src={hit.iconUrl} alt={hit.title} />
      {:else}
        <div class="icon placeholder"><Icon name="package" size={28} /></div>
      {/if}
      <div class="meta">
        <h3>{hit.title}</h3>
        <p class="by">by {hit.author}</p>
        <div class="stats">
          <span><Icon name="package" size={13} /> {formatCount(hit.downloads)} downloads</span>
          <span class="type">{hit.projectType}</span>
        </div>
      </div>
    </div>

    <p class="desc">{hit.description}</p>

    <div class="install-box">
      {#if modpackUnsupported}
        <p class="warn">
          CurseForge modpacks use a different format that isn't supported yet —
          Modrinth modpacks work. You can still install individual CurseForge
          mods into an instance.
        </p>
      {:else if isModpack}
        <p class="mp-note">
          Installs as a <strong>new instance</strong>{#if versions.length > 0}
            (latest {versions[0].versionNumber}{#if versions[0].gameVersions[0]}, MC
              {versions[0].gameVersions[0]}{/if}){/if}.
        </p>
        {#if loadingVersions}
          <span class="muted">Finding latest version…</span>
        {:else if versionError}
          <span class="warn">{versionError}</span>
        {/if}
        {#if installing}
          <div class="mp-progress">
            <div class="progress-head">
              <span>{mpMessage || "Installing…"}</span>
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
        <p class="muted">Create an instance first, then install content into it.</p>
      {:else}
        <label class="field-label" for="pm-instance">Install to</label>
        <select id="pm-instance" class="select" bind:value={selectedInstanceId}>
          {#each instances as inst (inst.id)}
            <option value={inst.id}>
              {inst.name} ({inst.loader} {inst.mcVersion})
            </option>
          {/each}
        </select>
        <div class="version-line">
          {#if loadingVersions}
            <span class="muted">Finding compatible version…</span>
          {:else if versionError}
            <span class="warn">{versionError}</span>
          {:else if versions.length > 0}
            <span class="muted">
              Latest match: <strong>{versions[0].versionNumber}</strong>
            </span>
            {#if isUpToDate}
              <span class="pill ok">✓ Installed</span>
            {:else if isInstalled}
              <span class="pill upd">Update available</span>
            {/if}
          {/if}
        </div>
        {#if versions.length > 0 && !canDownload}
          <p class="cf-optout">
            The author opted out of third-party downloads on CurseForge, so this
            can't be auto-installed. Get it from the
            <button
              class="linklike"
              onclick={() =>
                openLink(
                  `https://www.curseforge.com/minecraft/search?search=${encodeURIComponent(hit.title)}`
                )}
            >CurseForge page</button>.
          </p>
        {/if}
      {/if}
    </div>

    {#if error}<p class="err">{error}</p>{/if}
  {/if}

  {#snippet footer()}
    <button class="btn ghost" onclick={close} disabled={installing}>Close</button>
    {#if isModpack}
      <button
        class="btn primary"
        disabled={installing ||
          loadingVersions ||
          versions.length === 0 ||
          modpackUnsupported}
        onclick={installModpack}
      >
        {installing ? "Installing…" : "Install modpack"}
      </button>
    {:else if done}
      <span class="ok-pill">✓ {isUpToDate ? "Installed" : "Done"}</span>
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
