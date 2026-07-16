<script lang="ts">
  import Icon from "./Icon.svelte";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { presence } from "$lib/stores/presence.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { skinFace } from "$lib/skin";

  const online = boardApi.configured();
  const account = $derived(accountsStore.active);

  // Sign in with the active Minecraft account (no separate login).
  $effect(() => {
    const a = account;
    if (!online || !a) return;
    if (
      !boardAuth.loggingIn &&
      (!boardAuth.signedIn || boardAuth.session?.uuid !== a.uuid)
    ) {
      boardAuth.login();
    }
  });

  // Poll while mounted; re-poll as soon as we have a session token.
  $effect(() => {
    presence.open();
    return () => presence.close();
  });
  $effect(() => {
    if (boardAuth.token) void presence.poll();
  });

  // Local editable copies of the broadcast fields (committed on change).
  let status = $state(presence.status);
  let address = $state(presence.serverAddress);

  const myUuid = $derived(boardAuth.session?.uuid ?? "");
  const others = $derived(presence.players.filter((p) => p.uuid !== myUuid));
  const me = $derived(presence.players.find((p) => p.uuid === myUuid) ?? null);

  async function toggleOnline() {
    await presence.setEnabled(!presence.enabled);
  }
  function saveFields() {
    presence.saveFields(status.trim(), address.trim());
  }

  async function copyAddress(addr: string) {
    try {
      await navigator.clipboard.writeText(addr);
      toast.success(`Copied ${addr}`);
    } catch (e) {
      toast.error(String(e));
    }
  }

  function timeAgo(iso: string): string {
    const s = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
    if (s < 30) return "now";
    if (s < 90) return "just now";
    return `${Math.round(s / 60)}m ago`;
  }
</script>

{#if !online}
  <p class="offline">
    <Icon name="globe" size={13} /> Play Together is offline in this build.
  </p>
{:else if !boardAuth.signedIn}
  <p class="offline">
    <Icon name="user" size={13} /> Add a Microsoft account to see who's online.
  </p>
{:else}
  <section class="me">
    <label class="switch-row">
      <input
        type="checkbox"
        checked={presence.enabled}
        onchange={toggleOnline}
      />
      <span class="track"><span class="thumb"></span></span>
      <span class="switch-label">
        Appear online
        <small>Let other players see you and, optionally, a server to join.</small>
      </span>
    </label>

    {#if presence.enabled}
      <div class="fields">
        <label class="field">
          <span>Status</span>
          <input
            class="input"
            placeholder="e.g. looking to play modded 1.20"
            maxlength="120"
            bind:value={status}
            onblur={saveFields}
            onkeydown={(e) => e.key === "Enter" && saveFields()}
          />
        </label>
        <label class="field">
          <span>Server address (optional)</span>
          <input
            class="input"
            placeholder="e.g. play.example.net"
            maxlength="80"
            bind:value={address}
            onblur={saveFields}
            onkeydown={(e) => e.key === "Enter" && saveFields()}
          />
        </label>
      </div>
      <p class="hint">
        Others connect from a client instance: Multiplayer → Direct Connect.
        You drop off the list a couple of minutes after leaving this page.
      </p>
    {/if}
  </section>

  <div class="list-head">
    <h3>Online now</h3>
    <span class="count">{others.length + (me ? 1 : 0)}</span>
    <button class="btn ghost sm" onclick={() => presence.poll()} disabled={presence.loading}>
      Refresh
    </button>
  </div>

  {#if presence.error}
    <p class="offline"><Icon name="globe" size={13} /> {presence.error}</p>
  {/if}

  {#if others.length === 0 && !me}
    <div class="empty">
      <div class="mark"><Icon name="users" size={30} /></div>
      <p>No one online right now. Toggle “Appear online” so others can find you.</p>
    </div>
  {:else}
    <ul class="players">
      {#if me}
        {@render row(me, true)}
      {/if}
      {#each others as p (p.uuid)}
        {@render row(p, false)}
      {/each}
    </ul>
  {/if}
{/if}

{#snippet row(p: import("$lib/types").PresencePlayer, isMe: boolean)}
  <li class="player" class:me-row={isMe}>
    <img class="face" src={skinFace(p.uuid, 32)} alt={p.name} />
    <div class="body">
      <span class="name">{p.name}{#if isMe} <span class="you">you</span>{/if}</span>
      <span class="status">{p.status || "online"}</span>
    </div>
    <span class="ago">{timeAgo(p.updatedAt)}</span>
    {#if p.serverAddress}
      <button class="btn ghost sm" title="Copy server address" onclick={() => copyAddress(p.serverAddress!)}>
        <Icon name="copy" size={13} /> {p.serverAddress}
      </button>
    {/if}
  </li>
{/snippet}

<style>
  .offline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .me {
    background: var(--bg-card);
    border: 2px solid var(--border);
    padding: 14px 16px;
    margin-bottom: 18px;
  }
  .switch-row {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }
  .switch-row input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .track {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    width: 44px;
    height: 24px;
    padding: 2px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    transition: background 0.12s, border-color 0.12s;
  }
  .thumb {
    width: 16px;
    height: 16px;
    background: var(--text-muted);
    transition: transform 0.14s ease, background 0.12s;
  }
  .switch-row input:checked + .track {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .switch-row input:checked + .track .thumb {
    transform: translateX(20px);
    background: var(--accent);
  }
  .switch-label {
    display: flex;
    flex-direction: column;
    font-size: 13px;
    font-weight: 600;
  }
  .switch-label small {
    font-weight: 400;
    font-size: 12px;
    color: var(--text-muted);
  }
  .fields {
    display: flex;
    gap: 12px;
    margin-top: 14px;
    flex-wrap: wrap;
  }
  .field {
    flex: 1;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .hint {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }
  .list-head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .list-head h3 {
    font-size: 14px;
  }
  .count {
    font-size: 12px;
    color: var(--text-muted);
  }
  .list-head .btn {
    margin-left: auto;
  }
  .players {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .player {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-card);
    border: 2px solid var(--border);
  }
  .player.me-row {
    border-color: var(--accent);
  }
  .face {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    image-rendering: pixelated;
    border: 2px solid rgba(0, 0, 0, 0.3);
  }
  .body {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .you {
    font-family: var(--font-pixel);
    font-size: 8px;
    text-transform: uppercase;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 0 4px;
    vertical-align: middle;
  }
  .status {
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 40ch;
  }
  .ago {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-muted);
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
