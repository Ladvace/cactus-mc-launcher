<script lang="ts">
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { timeAgo } from "$lib/time";
  import { t } from "$lib/i18n";
  import type { WorldInfo } from "$lib/types";

  let { id, running = false }: { id: string; running?: boolean } = $props();

  let worlds = $state<WorldInfo[]>([]);
  let loading = $state(false);
  let busyFolder = $state<string | null>(null);
  let confirmFolder = $state<string | null>(null);

  let lastId = "";
  $effect(() => {
    if (id && id !== lastId) {
      lastId = id;
      load();
    }
  });

  async function load() {
    loading = true;
    try {
      worlds = await api.listWorlds(id);
    } catch (error) {
      toast.error(String(error));
    } finally {
      loading = false;
    }
  }

  async function reveal(world: WorldInfo) {
    try {
      await revealItemInDir(world.path);
    } catch (error) {
      toast.error(String(error));
    }
  }

  async function backup(world: WorldInfo) {
    busyFolder = world.folder;
    try {
      const path = await api.backupWorld(id, world.folder);
      toast.success(t("server.backedUp", { name: world.name }));
      try {
        await revealItemInDir(path);
      } catch {
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      busyFolder = null;
    }
  }

  async function remove(world: WorldInfo) {
    busyFolder = world.folder;
    try {
      await api.deleteWorld(id, world.folder);
      confirmFolder = null;
      toast.success(t("server.deleted", { name: world.name }));
      await load();
    } catch (error) {
      toast.error(String(error));
    } finally {
      busyFolder = null;
    }
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const kb = bytes / 1024;
    if (kb < 1024) return `${kb.toFixed(0)} KB`;
    const mb = kb / 1024;
    if (mb < 1024) return `${mb.toFixed(1)} MB`;
    return `${(mb / 1024).toFixed(2)} GB`;
  }
</script>

<div class="worlds">
  <div class="head">
    <span class="muted">
      {worlds.length === 1
        ? t("server.worldCountOne", { count: worlds.length })
        : t("server.worldCountMany", { count: worlds.length })}
      {#if running}· <strong>{t("server.stopBeforeDelete")}</strong>{/if}
    </span>
    <button class="btn ghost sm" onclick={load} disabled={loading}>{t("server.reload")}</button>
  </div>

  {#if loading && worlds.length === 0}
    <p class="muted">{t("common.loading")}</p>
  {:else if worlds.length === 0}
    <div class="empty">
      <div class="mark"><Icon name="globe" size={30} /></div>
      <p>{t("server.noWorlds")}</p>
    </div>
  {:else}
    <ul class="list">
      {#each worlds as world (world.folder)}
        <li class="world">
          <div class="info">
            <div class="name-row">
              <span class="name" title={world.folder}>{world.name}</span>
              <span class="loc">{world.location === "server" ? t("server.locServer") : t("server.locSave")}</span>
            </div>
            <span class="sub">{t("server.worldMeta", { size: fmtSize(world.sizeBytes), ago: timeAgo(world.lastModified) })}</span>
          </div>
          <div class="acts">
            <button class="btn ghost sm" title={t("server.showInFileManager")} onclick={() => reveal(world)}>
              <Icon name="folder" size={13} />
            </button>
            <button
              class="btn ghost sm"
              disabled={busyFolder === world.folder}
              onclick={() => backup(world)}
            >
              {busyFolder === world.folder && confirmFolder !== world.folder ? t("server.zipping") : t("server.backup")}
            </button>
            {#if confirmFolder === world.folder}
              <button
                class="btn danger sm"
                disabled={busyFolder === world.folder}
                onclick={() => remove(world)}
              >
                {busyFolder === world.folder ? t("server.deleting") : t("server.confirm")}
              </button>
              <button class="btn ghost sm" onclick={() => (confirmFolder = null)}>{t("common.cancel")}</button>
            {:else}
              <button
                class="btn ghost sm danger-text"
                title={t("server.deleteWorld")}
                onclick={() => (confirmFolder = world.folder)}
              >
                <Icon name="trash" size={13} />
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .world {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: var(--bg-card);
    border: 2px solid var(--border);
  }
  .info {
    min-width: 0;
  }
  .name-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-weight: 600;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .loc {
    flex-shrink: 0;
    font-family: var(--font-pixel);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    padding: 1px 5px;
  }
  .sub {
    font-size: 12px;
    color: var(--text-muted);
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .danger-text:hover {
    color: var(--danger);
    border-color: var(--danger);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 40px 20px;
    color: var(--text-muted);
    text-align: center;
  }
  .empty .mark {
    opacity: 0.6;
  }
</style>
