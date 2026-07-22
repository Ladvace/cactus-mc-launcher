<script lang="ts">
  import Modal from "./Modal.svelte";
  import LoaderIcon from "./LoaderIcon.svelte";
  import Icon from "./Icon.svelte";
  import Select from "$lib/components/Select.svelte";
  import { randomInstanceName } from "$lib/funnyNames";
  import { t } from "$lib/i18n";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { api } from "$lib/api";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    MOD_LOADERS,
    SUPPORTED_LOADERS,
    type InstanceKind,
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

  const initialLoader = (): ModLoader => {
    const pref = settingsStore.settings.defaultLoader as ModLoader;
    return MOD_LOADERS.some((option) => option.value === pref) ? pref : "vanilla";
  };

  let name = $state("");
  let kind = $state<InstanceKind>("client");
  let eulaAccepted = $state(false);
  let loader = $state<ModLoader>(initialLoader());
  let selectedVersion = $state("");
  let showSnapshots = $state(false);

  let versions = $state<MinecraftVersion[]>([]);
  let versionsLoading = $state(false);
  let versionError = $state<string | null>(null);
  let creating = $state(false);

  let loaderVersions = $state<LoaderVersion[]>([]);
  let loaderVersionsLoading = $state(false);
  let loaderVersionError = $state<string | null>(null);
  let selectedLoaderVersion = $state("");

  const supportedLoader = $derived(
    loader === "vanilla" || SUPPORTED_LOADERS.includes(loader)
  );
  const needsLoaderVersion = $derived(
    loader !== "vanilla" && SUPPORTED_LOADERS.includes(loader)
  );

  $effect(() => {
    if (open && versions.length === 0 && !versionsLoading) {
      loadVersions();
    }
  });

  $effect(() => {
    const currentLoader = loader;
    const mcVersion = selectedVersion;
    if (open && needsLoaderVersion && mcVersion) {
      loadLoaderVersions(currentLoader, mcVersion);
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
    } catch (error) {
      versionError = String(error);
    } finally {
      versionsLoading = false;
    }
  }

  async function loadLoaderVersions(loaderName: ModLoader, mcVersion: string) {
    loaderVersionsLoading = true;
    loaderVersionError = null;
    selectedLoaderVersion = "";
    try {
      loaderVersions = await api.getLoaderVersions(loaderName, mcVersion);
      if (loaderVersions.length === 0) {
        loaderVersionError = t("create.noBuilds", { loader: loaderName, version: mcVersion });
      }
    } catch (error) {
      loaderVersions = [];
      loaderVersionError = String(error);
    } finally {
      loaderVersionsLoading = false;
    }
  }

  const visibleVersions = $derived(
    versions.filter((version) =>
      showSnapshots ? true : version.type === "release"
    )
  );

  const loaderVersionOptions = $derived([
    { value: "", label: t("create.latestStable") },
    ...loaderVersions.map((loaderVersion) => ({
      value: loaderVersion.version,
      label: `${loaderVersion.version}${loaderVersion.stable ? "" : " (beta)"}`,
    })),
  ]);

  const versionOptions = $derived(
    visibleVersions.map((version) => ({
      value: version.id,
      label: `${version.id}${version.type !== "release" ? ` (${version.type})` : ""}`,
    }))
  );

  const canCreate = $derived(
    name.trim().length > 0 &&
      selectedVersion.length > 0 &&
      supportedLoader &&
      (!needsLoaderVersion || (!loaderVersionsLoading && loaderVersions.length > 0)) &&
      (kind === "client" || eulaAccepted) &&
      !creating
  );

  async function create() {
    if (!canCreate) return;
    creating = true;
    try {
      const inst = await instancesStore.create({
        name: name.trim(),
        kind,
        mcVersion: selectedVersion,
        loader,
        loaderVersion:
          loader === "vanilla" ? null : selectedLoaderVersion || null,
      });
      reset();
      onClose();
      goto(`/instance/${inst.id}`);
    } catch (error) {
      versionError = String(error);
    } finally {
      creating = false;
    }
  }

  function reset() {
    name = "";
    kind = "client";
    eulaAccepted = false;
    loader = initialLoader();
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

<Modal title={t("create.title")} {open} onClose={close}>
  <div class="form">
    <div class="kind-toggle" role="tablist" aria-label={t("create.instanceType")}>
      <button
        type="button"
        role="tab"
        aria-selected={kind === "client"}
        class="kind-btn"
        class:active={kind === "client"}
        onclick={() => (kind = "client")}
      >
        <Icon name="cube" size={16} /> {t("create.client")}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={kind === "server"}
        class="kind-btn"
        class:active={kind === "server"}
        onclick={() => (kind = "server")}
      >
        <Icon name="globe" size={16} /> {t("create.server")}
      </button>
    </div>

    <div>
      <label class="field-label" for="ci-name">{t("create.name")}</label>
      <div class="name-row">
        <input
          id="ci-name"
          class="input"
          placeholder={t("create.namePlaceholder")}
          bind:value={name}
          autocomplete="off"
        />
        <button
          type="button"
          class="dice"
          title={t("create.rollRandomName")}
          onclick={() => (name = randomInstanceName())}
        >
          <Icon name="shuffle" size={16} />
        </button>
      </div>
    </div>

    <div>
      <label class="field-label" for="ci-loader">{t("create.modLoader")}</label>
      <div class="loader-grid">
        {#each MOD_LOADERS as loaderOption}
          <button
            type="button"
            class="loader-chip"
            class:active={loader === loaderOption.value}
            onclick={() => (loader = loaderOption.value)}
          >
            <LoaderIcon loader={loaderOption.value} size={16} />
            {loaderOption.label}
          </button>
        {/each}
      </div>
      {#if needsLoaderVersion}
        <div class="loader-version">
          <label class="field-label" for="ci-loader-version">{t("create.loaderVersion")}</label>
          {#if loaderVersionsLoading}
            <div class="status">{t("create.loadingLoaderBuilds")}</div>
          {:else if loaderVersionError}
            <div class="status error">{loaderVersionError}</div>
          {:else}
            <Select
              id="ci-loader-version"
              bind:value={selectedLoaderVersion}
              options={loaderVersionOptions}
            />
          {/if}
          {#if loader === "forge" || loader === "neoforge"}
            <p class="hint">
              {t("create.installerNote")}
            </p>
          {/if}
        </div>
      {:else if loader !== "vanilla"}
        <p class="hint warn">
          {t("create.loaderComingSoon", {
            loader: MOD_LOADERS.find((option) => option.value === loader)?.label ?? loader,
          })}
        </p>
      {/if}
    </div>

    <div>
      <div class="version-head">
        <label class="field-label" for="ci-version">{t("create.minecraftVersion")}</label>
        <label class="snap-toggle">
          <input type="checkbox" bind:checked={showSnapshots} />
          {t("create.showSnapshots")}
        </label>
      </div>

      {#if versionsLoading}
        <div class="status">{t("create.loadingVersions")}</div>
      {:else if versionError}
        <div class="status error">{versionError}</div>
        <button class="btn ghost" onclick={loadVersions}>{t("common.retry")}</button>
      {:else}
        <Select id="ci-version" bind:value={selectedVersion} options={versionOptions} />
      {/if}
    </div>

    {#if kind === "server"}
      <div class="eula">
        <label class="eula-check">
          <input type="checkbox" bind:checked={eulaAccepted} />
          <span>
            {t("create.eulaAgree")}
            <button
              type="button"
              class="link"
              onclick={() => openUrl("https://www.minecraft.net/eula")}
              >{t("create.minecraftEula")}</button
            >.
          </span>
        </label>
        <p class="hint">
          {t("create.serverHeadless")}
        </p>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <button class="btn ghost" onclick={close}>{t("common.cancel")}</button>
    <button class="btn primary" disabled={!canCreate} onclick={create}>
      {creating ? t("common.creating") : t("common.create")}
    </button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .kind-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .kind-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 600;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.22);
    transition: all 0.12s;
  }
  .kind-btn:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .kind-btn.active {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .eula {
    padding: 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .eula-check {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  .eula .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
  .eula .hint {
    margin: 8px 0 0;
  }
  .name-row {
    display: flex;
    gap: 8px;
  }
  .name-row .input {
    flex: 1;
  }
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
