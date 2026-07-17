<script lang="ts">
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import type { OpEntry, PlayerEntry } from "$lib/types";

  let { id, running = false }: { id: string; running?: boolean } = $props();

  let ops = $state<OpEntry[]>([]);
  let whitelist = $state<PlayerEntry[]>([]);
  let opName = $state("");
  let wlName = $state("");
  let busy = $state(false);

  let lastId = "";
  $effect(() => {
    if (id && id !== lastId) {
      lastId = id;
      load();
    }
  });

  async function load() {
    try {
      [ops, whitelist] = await Promise.all([api.readOps(id), api.readWhitelist(id)]);
    } catch (error) {
      toast.error(String(error));
    }
  }

  // A running server owns the files, so drive it via console commands and
  // re-read once it has rewritten them; a stopped server is edited directly.
  async function run(consoleCmd: string, edit: () => Promise<void>) {
    busy = true;
    try {
      if (running) {
        await api.sendServerCommand(id, consoleCmd);
        await new Promise((resolve) => setTimeout(resolve, 700));
      } else {
        await edit();
      }
      await load();
    } catch (error) {
      toast.error(String(error));
    } finally {
      busy = false;
    }
  }

  async function addOp() {
    const name = opName.trim();
    if (!name) return;
    await run(`op ${name}`, () => api.addOp(id, name));
    opName = "";
  }
  async function removeOp(name: string) {
    await run(`deop ${name}`, () => api.removeOp(id, name));
  }
  async function addWl() {
    const name = wlName.trim();
    if (!name) return;
    await run(`whitelist add ${name}`, () => api.addWhitelist(id, name));
    wlName = "";
  }
  async function removeWl(name: string) {
    await run(`whitelist remove ${name}`, () => api.removeWhitelist(id, name));
  }
</script>

<div class="players">
  <p class="lead muted">
    {#if running}
      Changes apply live on the running server.
    {:else}
      The server is stopped — changes are written for its next start. Adding a
      player looks up their account (online-mode) to record the right UUID.
    {/if}
  </p>

  <section class="list-block">
    <div class="block-head">
      <h3>Operators</h3>
      <span class="muted">{ops.length}</span>
    </div>
    <p class="block-hint">Ops can run admin commands (gamemode, give, stop, kick…).</p>
    <div class="add-row">
      <input
        class="input"
        placeholder="Player name"
        bind:value={opName}
        disabled={busy}
        onkeydown={(e) => e.key === "Enter" && addOp()}
      />
      <button class="btn primary sm" disabled={busy || !opName.trim()} onclick={addOp}>Op</button>
    </div>
    {#if ops.length}
      <ul class="rows">
        {#each ops as op (op.uuid)}
          <li class="row">
            <span class="name">{op.name}</span>
            <span class="lvl">level {op.level}</span>
            <button class="del" title="Remove op" disabled={busy} onclick={() => removeOp(op.name)}>
              <Icon name="trash" size={13} />
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty muted">No operators.</p>
    {/if}
  </section>

  <section class="list-block">
    <div class="block-head">
      <h3>Whitelist</h3>
      <span class="muted">{whitelist.length}</span>
    </div>
    <p class="block-hint">
      When <code>white-list</code> is on (Properties tab), only these players can join.
    </p>
    <div class="add-row">
      <input
        class="input"
        placeholder="Player name"
        bind:value={wlName}
        disabled={busy}
        onkeydown={(e) => e.key === "Enter" && addWl()}
      />
      <button class="btn primary sm" disabled={busy || !wlName.trim()} onclick={addWl}>Add</button>
    </div>
    {#if whitelist.length}
      <ul class="rows">
        {#each whitelist as player (player.uuid)}
          <li class="row">
            <span class="name">{player.name}</span>
            <button class="del" title="Remove" disabled={busy} onclick={() => removeWl(player.name)}>
              <Icon name="trash" size={13} />
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty muted">Whitelist is empty.</p>
    {/if}
  </section>
</div>

<style>
  .players {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .lead {
    font-size: 12.5px;
    margin: 0;
    max-width: 70ch;
  }
  .muted {
    color: var(--text-muted);
  }
  .list-block {
    background: var(--bg-card);
    border: 2px solid var(--border);
    padding: 16px 18px;
  }
  .block-head {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .block-head h3 {
    font-size: 14px;
  }
  .block-hint {
    margin: 4px 0 12px;
    font-size: 12px;
    color: var(--text-muted);
  }
  code {
    font-family: var(--font-pixel);
    color: var(--accent);
  }
  .add-row {
    display: flex;
    gap: 8px;
    max-width: 360px;
    margin-bottom: 12px;
  }
  .add-row .input {
    flex: 1;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: var(--bg-input);
    border: 2px solid var(--border-subtle);
  }
  .row .name {
    flex: 1;
    font-weight: 600;
    font-size: 13px;
  }
  .row .lvl {
    font-size: 11px;
    color: var(--text-muted);
  }
  .del {
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 2px 4px;
  }
  .del:hover:not(:disabled) {
    color: var(--danger);
  }
  .empty {
    font-size: 12.5px;
    margin: 0;
  }
</style>
