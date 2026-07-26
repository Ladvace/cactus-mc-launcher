<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { t } from "$lib/i18n";
  import type { DetectedLauncher, ExternalInstance } from "$lib/types";

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();

  let scanning = $state(false);
  let launchers = $state<DetectedLauncher[]>([]);
  let selected = $state<DetectedLauncher | null>(null);
  let instances = $state<ExternalInstance[]>([]);
  let loadingInstances = $state(false);
  let deleteSource = $state(false);
  let status = $state<Record<number, "importing" | "done" | "error">>({});

  const loaderLabel = (loader: string) =>
    ({ fabric: "Fabric", quilt: "Quilt", forge: "Forge", neoforge: "NeoForge" })[loader] ?? "";

  const BRAND: Record<string, { color: string; label: string }> = {
    prism: { color: "#4f8cc9", label: "Pr" },
    multimc: { color: "#5fa85f", label: "MM" },
    polymc: { color: "#8b6cf0", label: "Po" },
    curseforge: { color: "#f16436", label: "CF" },
    modrinth: { color: "#1bd96a", label: "Mo" },
    vanilla: { color: "#7aa64f", label: "MC" },
  };
  const brand = (id: string) => BRAND[id] ?? { color: "var(--accent)", label: "?" };

  $effect(() => {
    if (open) {
      void scan();
    } else {
      selected = null;
      instances = [];
      status = {};
      deleteSource = false;
    }
  });

  async function scan() {
    scanning = true;
    launchers = [];
    try {
      launchers = await api.detectLaunchers();
    } catch (err) {
      toast.error(String(err));
    } finally {
      scanning = false;
    }
  }

  async function openLauncher(launcher: DetectedLauncher) {
    selected = launcher;
    instances = [];
    status = {};
    deleteSource = false;
    loadingInstances = true;
    try {
      instances = await api.listExternalInstances(launcher.id);
    } catch (err) {
      toast.error(String(err));
    } finally {
      loadingInstances = false;
    }
  }

  function back() {
    selected = null;
    instances = [];
    status = {};
  }

  async function importOne(instance: ExternalInstance, index: number) {
    if (status[index] === "importing" || status[index] === "done") return;
    status = { ...status, [index]: "importing" };
    const wipe = deleteSource && !!instance.sourceDir;
    try {
      const created = await api.importExternalInstance(instance, wipe);
      status = { ...status, [index]: "done" };
      toast.success(t("launcherImport.imported", { name: created.name }));
      instancesStore.refresh().catch(() => {});
    } catch (err) {
      status = { ...status, [index]: "error" };
      toast.error(String(err));
    }
  }
</script>

<Modal title={t("launcherImport.title")} {open} {onClose} width={480}>
  {#if selected}
    <div class="crumbs">
      <button class="link" onclick={back}>
        <Icon name="chevron-left" size={14} /> {t("launcherImport.allLaunchers")}
      </button>
      <span class="sep">/</span>
      <strong>{selected.name}</strong>
    </div>

    {#if selected.canDelete}
      <label class="toggle">
        <input type="checkbox" bind:checked={deleteSource} />
        <span>{t("launcherImport.deleteAfter")}</span>
      </label>
    {:else}
      <p class="hint"><Icon name="globe" size={12} /> {t("launcherImport.sharedDirHint")}</p>
    {/if}

    {#if loadingInstances}
      <div class="rows">
        {#each Array(3) as _}
          <div class="skel"></div>
        {/each}
      </div>
    {:else if instances.length === 0}
      <p class="empty">{t("launcherImport.noInstances")}</p>
    {:else}
      <ul class="rows">
        {#each instances as instance, i (instance.name + i)}
          <li class="row">
            <div class="meta">
              <span class="name">{instance.name}</span>
              <span class="sub">
                {instance.mcVersion}
                {#if loaderLabel(instance.loader)}<span class="badge">{loaderLabel(instance.loader)}</span>{/if}
              </span>
            </div>
            {#if status[i] === "done"}
              <span class="ok"><Icon name="check" size={14} /> {t("launcherImport.done")}</span>
            {:else}
              <button
                class="btn primary sm"
                disabled={status[i] === "importing"}
                onclick={() => importOne(instance, i)}
              >
                {status[i] === "importing"
                  ? t("common.importing")
                  : status[i] === "error"
                    ? t("launcherImport.retry")
                    : t("common.import")}
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {:else if scanning}
    <div class="rows">
      {#each Array(3) as _}
        <div class="skel"></div>
      {/each}
    </div>
  {:else if launchers.length === 0}
    <div class="empty-state">
      <Icon name="search" size={22} />
      <p>{t("launcherImport.noLaunchers")}</p>
    </div>
  {:else}
    <ul class="rows">
      {#each launchers as launcher (launcher.id)}
        <li>
          <button class="launcher" onclick={() => openLauncher(launcher)}>
            {#if launcher.icon}
              <img class="brand-img" src={launcher.icon} alt="" />
            {:else}
              <span class="brand" style="--c:{brand(launcher.id).color}">{brand(launcher.id).label}</span>
            {/if}
            <span class="meta">
              <span class="name">{launcher.name}</span>
              <span class="sub">{t("launcherImport.instanceCount", { count: launcher.instanceCount })}</span>
            </span>
            <Icon name="chevron-right" size={16} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#snippet footer()}
    <button class="btn ghost" onclick={onClose}>{t("common.done")}</button>
  {/snippet}
</Modal>

<style>
  .rows {
    display: flex;
    flex-direction: column;
    gap: 8px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .launcher {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    text-align: left;
    transition: border-color 0.12s;
  }
  .launcher:hover {
    border-color: var(--accent);
  }
  .brand-img {
    flex-shrink: 0;
    width: 34px;
    height: 34px;
    object-fit: contain;
    image-rendering: auto;
  }
  .brand {
    flex-shrink: 0;
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-raised);
    border: 2px solid var(--c);
    color: var(--c);
    font-family: var(--font-pixel);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .badge {
    padding: 1px 6px;
    background: var(--bg-raised);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: 10px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--accent);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    font-size: 13px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 0;
  }
  .link:hover {
    color: var(--accent);
  }
  .crumbs .sep {
    color: var(--text-muted);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 12px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .empty,
  .empty-state p {
    color: var(--text-muted);
    font-size: 13px;
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 28px 0;
    color: var(--text-muted);
    text-align: center;
  }
  .skel {
    height: 46px;
    background: linear-gradient(90deg, var(--bg-input) 25%, var(--bg-hover) 50%, var(--bg-input) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.2s infinite;
  }
  @keyframes shimmer {
    from {
      background-position: 200% 0;
    }
    to {
      background-position: -200% 0;
    }
  }
</style>
