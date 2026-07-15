<script lang="ts">
  import Modal from "./Modal.svelte";
  import LoaderIcon from "./LoaderIcon.svelte";
  import Icon from "./Icon.svelte";
  import { randomInstanceName } from "$lib/funnyNames";
  import { api } from "$lib/api";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import {
    MOD_LOADERS,
    SUPPORTED_LOADERS,
    type ModLoader,
    type MinecraftVersion,
    type LoaderVersion,
  } from "$lib/types";
  import { goto } from "$app/navigation";

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();

  let name = $state("");
  let loader = $state<ModLoader>("vanilla");
  let selectedVersion = $state("");
  let showSnapshots = $state(false);

  let versions = $state<MinecraftVersion[]>([]);
  let versionsLoading = $state(false);
  let versionError = $state<string | null>(null);
  let creating = $state(false);

  // Loader version state (Fabric/Quilt).
  let loaderVersions = $state<LoaderVersion[]>([]);
  let loaderVersionsLoading = $state(false);
  let loaderVersionError = $state<string | null>(null);
  let selectedLoaderVersion = $state(""); // "" = latest stable

  const supportedLoader = $derived(
    loader === "vanilla" || SUPPORTED_LOADERS.includes(loader)
  );
  const needsLoaderVersion = $derived(
    loader !== "vanilla" && SUPPORTED_LOADERS.includes(loader)
  );

  // Fetch the version manifest the first time the modal opens.
  $effect(() => {
    if (open && versions.length === 0 && !versionsLoading) {
      loadVersions();
    }
  });

  // Fetch loader builds whenever the loader or MC version changes.
  $effect(() => {
    const l = loader;
    const mc = selectedVersion;
    if (open && needsLoaderVersion && mc) {
      loadLoaderVersions(l, mc);
    } else {
      loaderVersions = [];
      loaderVersionError = null;
    }
  });

  async function loadVersions() {
    versionsLoading = true;
    versionError = null;
    try {
      const list = await api.getMinecraftVersions();
      versions = list.versions;
      selectedVersion = list.latestRelease;
    } catch (e) {
      versionError = String(e);
    } finally {
      versionsLoading = false;
    }
  }

  async function loadLoaderVersions(l: ModLoader, mc: string) {
    loaderVersionsLoading = true;
    loaderVersionError = null;
    selectedLoaderVersion = "";
    try {
      loaderVersions = await api.getLoaderVersions(l, mc);
      if (loaderVersions.length === 0) {
        loaderVersionError = `No ${l} builds for Minecraft ${mc}.`;
      }
    } catch (e) {
      loaderVersions = [];
      loaderVersionError = String(e);
    } finally {
      loaderVersionsLoading = false;
    }
  }

  const visibleVersions = $derived(
    versions.filter((v) =>
      showSnapshots ? true : v.type === "release"
    )
  );

  const canCreate = $derived(
    name.trim().length > 0 &&
      selectedVersion.length > 0 &&
      supportedLoader &&
      // a modded loader needs at least one compatible build
      (!needsLoaderVersion || (!loaderVersionsLoading && loaderVersions.length > 0)) &&
      !creating
  );

  async function create() {
    if (!canCreate) return;
    creating = true;
    try {
      const inst = await instancesStore.create({
        name: name.trim(),
        mcVersion: selectedVersion,
        loader,
        loaderVersion:
          loader === "vanilla" ? null : selectedLoaderVersion || null,
      });
      reset();
      onClose();
      goto(`/instance/${inst.id}`);
    } catch (e) {
      versionError = String(e);
    } finally {
      creating = false;
    }
  }

  function reset() {
    name = "";
    loader = "vanilla";
    showSnapshots = false;
    loaderVersions = [];
    selectedLoaderVersion = "";
    loaderVersionError = null;
  }

  function close() {
    reset();
    onClose();
  }
</script>

<Modal title="Create instance" {open} onClose={close}>
  <div class="form">
    <div>
      <label class="field-label" for="ci-name">Name</label>
      <div class="name-row">
        <input
          id="ci-name"
          class="input"
          placeholder="My awesome instance"
          bind:value={name}
          autocomplete="off"
        />
        <button
          type="button"
          class="dice"
          title="Roll a random name"
          onclick={() => (name = randomInstanceName())}
        >
          <Icon name="shuffle" size={16} />
        </button>
      </div>
    </div>

    <div>
      <label class="field-label" for="ci-loader">Mod loader</label>
      <div class="loader-grid">
        {#each MOD_LOADERS as l}
          <button
            type="button"
            class="loader-chip"
            class:active={loader === l.value}
            onclick={() => (loader = l.value)}
          >
            <LoaderIcon loader={l.value} size={16} />
            {l.label}
          </button>
        {/each}
      </div>
      {#if needsLoaderVersion}
        <div class="loader-version">
          <label class="field-label" for="ci-loader-version">Loader version</label>
          {#if loaderVersionsLoading}
            <div class="status">Loading loader builds…</div>
          {:else if loaderVersionError}
            <div class="status error">{loaderVersionError}</div>
          {:else}
            <select
              id="ci-loader-version"
              class="select"
              bind:value={selectedLoaderVersion}
            >
              <option value="">Latest stable</option>
              {#each loaderVersions as lv (lv.version)}
                <option value={lv.version}>
                  {lv.version}{lv.stable ? "" : " (beta)"}
                </option>
              {/each}
            </select>
          {/if}
          {#if loader === "forge" || loader === "neoforge"}
            <p class="hint">
              First launch runs the official installer, which can take a minute.
            </p>
          {/if}
        </div>
      {:else if loader !== "vanilla"}
        <p class="hint warn">
          {MOD_LOADERS.find((l) => l.value === loader)?.label} support is coming
          soon. Pick Vanilla, Fabric, or Quilt for now.
        </p>
      {/if}
    </div>

    <div>
      <div class="version-head">
        <label class="field-label" for="ci-version">Minecraft version</label>
        <label class="snap-toggle">
          <input type="checkbox" bind:checked={showSnapshots} />
          Show snapshots
        </label>
      </div>

      {#if versionsLoading}
        <div class="status">Loading versions…</div>
      {:else if versionError}
        <div class="status error">{versionError}</div>
        <button class="btn ghost" onclick={loadVersions}>Retry</button>
      {:else}
        <select id="ci-version" class="select" bind:value={selectedVersion}>
          {#each visibleVersions as v (v.id)}
            <option value={v.id}>
              {v.id}{v.type !== "release" ? ` (${v.type})` : ""}
            </option>
          {/each}
        </select>
      {/if}
    </div>
  </div>

  {#snippet footer()}
    <button class="btn ghost" onclick={close}>Cancel</button>
    <button class="btn primary" disabled={!canCreate} onclick={create}>
      {creating ? "Creating…" : "Create"}
    </button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .name-row {
    display: flex;
    gap: 8px;
  }
  .name-row .input {
    flex: 1;
  }
  /* Square dice button matched to the input height. */
  .dice {
    flex-shrink: 0;
    width: 42px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
    transition: color 0.12s, border-color 0.12s, transform 0.08s;
  }
  .dice:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .dice:active {
    transform: scale(0.9);
  }
  .loader-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(84px, 1fr));
    gap: 8px;
  }
  .loader-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 9px 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.22);
    transition: all 0.12s;
  }
  .loader-chip:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .loader-chip.active {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .hint {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }
  .hint.warn {
    color: var(--warning);
  }
  .loader-version {
    margin-top: 14px;
  }
  .version-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .snap-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    margin-bottom: 6px;
  }
  .status {
    padding: 10px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .status.error {
    color: var(--danger);
    margin-bottom: 8px;
  }
</style>
