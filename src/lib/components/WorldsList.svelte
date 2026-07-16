<script lang="ts">
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
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
    } catch (e) {
      toast.error(String(e));
    } finally {
      loading = false;
    }
  }

  async function reveal(w: WorldInfo) {
    try {
      await revealItemInDir(w.path);
    } catch (e) {
      toast.error(String(e));
    }
  }

  async function backup(w: WorldInfo) {
    busyFolder = w.folder;
    try {
      const path = await api.backupWorld(id, w.folder);
      toast.success(`Backed up “${w.name}”.`);
      try {
        await revealItemInDir(path);
      } catch {
        /* reveal is best-effort */
      }
    } catch (e) {
      toast.error(String(e));
    } finally {
      busyFolder = null;
    }
  }

  async function remove(w: WorldInfo) {
    busyFolder = w.folder;
    try {
      await api.deleteWorld(id, w.folder);
      confirmFolder = null;
      toast.success(`Deleted “${w.name}”.`);
      await load();
    } catch (e) {
      toast.error(String(e));
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

  function fmtWhen(iso: string | null): string {
    if (!iso) return "never";
    const s = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
    if (s < 90) return "just now";
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    return `${Math.round(h / 24)}d ago`;
  }
</script>

<div class="worlds">
  <div class="head">
    <span class="muted">
      {worlds.length} world{worlds.length === 1 ? "" : "s"}
      {#if running}· <strong>stop the server before deleting a world</strong>{/if}
    </span>
    <button class="btn ghost sm" onclick={load} disabled={loading}>Reload</button>
  </div>

  {#if loading && worlds.length === 0}
    <p class="muted">Loading…</p>
  {:else if worlds.length === 0}
    <div class="empty">
      <div class="mark"><Icon name="globe" size={30} /></div>
      <p>No worlds yet. They appear here after the world is generated on first launch.</p>
    </div>
  {:else}
    <ul class="list">
      {#each worlds as w (w.folder)}
        <li class="world">
          <div class="info">
            <div class="name-row">
              <span class="name" title={w.folder}>{w.name}</span>
              <span class="loc">{w.location === "server" ? "server" : "save"}</span>
            </div>
            <span class="sub">{fmtSize(w.sizeBytes)} · saved {fmtWhen(w.lastModified)}</span>
          </div>
          <div class="acts">
            <button class="btn ghost sm" title="Show in file manager" onclick={() => reveal(w)}>
              <Icon name="folder" size={13} />
            </button>
            <button
              class="btn ghost sm"
              disabled={busyFolder === w.folder}
              onclick={() => backup(w)}
            >
              {busyFolder === w.folder && confirmFolder !== w.folder ? "Zipping…" : "Backup"}
            </button>
            {#if confirmFolder === w.folder}
              <button
                class="btn danger sm"
                disabled={busyFolder === w.folder}
                onclick={() => remove(w)}
              >
                {busyFolder === w.folder ? "Deleting…" : "Confirm"}
              </button>
              <button class="btn ghost sm" onclick={() => (confirmFolder = null)}>Cancel</button>
            {:else}
              <button
                class="btn ghost sm danger-text"
                title="Delete world"
                onclick={() => (confirmFolder = w.folder)}
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
  .btn.sm {
    padding: 6px 10px;
    font-size: 12px;
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
