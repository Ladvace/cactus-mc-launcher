<script lang="ts">
  import { api } from "$lib/api";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { presence } from "$lib/stores/presence.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { skinFace } from "$lib/skin";
  import Icon from "./Icon.svelte";
  import type { FriendsList, FriendsPrefs } from "$lib/types";

  let data = $state<FriendsList | null>(null);
  let prefs = $state<FriendsPrefs | null>(null);
  let loading = $state(false);
  let failed = $state(false);
  let busy = $state(false);
  let addName = $state("");

  const account = $derived(accountsStore.active);

  async function load() {
    if (!account) {
      data = null;
      return;
    }
    loading = true;
    failed = false;
    try {
      data = await api.getFriends();
      prefs = await api.getFriendPrefs().catch(() => null);
    } catch {
      failed = true;
      data = null;
    } finally {
      loading = false;
    }
  }

  async function togglePref(key: "friendsEnabled" | "acceptInvites") {
    if (!prefs || busy) return;
    busy = true;
    const next = { ...prefs, [key]: !prefs[key] };
    try {
      prefs = await api.setFriendPrefs(next.friendsEnabled, next.acceptInvites);
    } catch (err) {
      toast.error(String(err));
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    account;
    load();
  });

  async function mutate(opts: { name?: string; profileId?: string; add: boolean }, ok?: string) {
    if (busy) return;
    busy = true;
    try {
      data = await api.friendUpdate(opts);
      if (ok) toast.success(ok);
    } catch (err) {
      toast.error(String(err));
    } finally {
      busy = false;
    }
  }

  function add() {
    const name = addName.trim();
    if (!name) return;
    addName = "";
    mutate({ name, add: true }, `Friend request sent to ${name}.`);
  }

  const onlineUuids = $derived(new Set(presence.players.map((p) => p.uuid)));
  const isOnline = (profileId: string) => onlineUuids.has(profileId.replace(/-/g, ""));
</script>

<!-- The friends API is new/unstable; if the account can't use it, stay quiet. -->
{#if account && !failed}
  <section class="friends">
    <div class="head">
      <h3>Friends</h3>
      <button class="refresh" title="Refresh" onclick={load} disabled={loading || busy}>
        <Icon name="refresh" size={12} />
      </button>
    </div>

    {#if prefs}
      <div class="prefs">
        <label class="pref">
          <input type="checkbox" checked={prefs.friendsEnabled} disabled={busy}
            onchange={() => togglePref("friendsEnabled")} />
          Friends enabled
        </label>
        <label class="pref">
          <input type="checkbox" checked={prefs.acceptInvites} disabled={busy}
            onchange={() => togglePref("acceptInvites")} />
          Accept invites
        </label>
      </div>
      {#if !prefs.friendsEnabled}
        <p class="muted warn">Friends are off — turn them on so you can add friends and others can add you.</p>
      {/if}
    {/if}

    <form class="add" onsubmit={(e) => (e.preventDefault(), add())}>
      <input placeholder="Add a friend by username…" bind:value={addName} maxlength="16" spellcheck="false" />
      <button class="btn primary sm" type="submit" disabled={busy || !addName.trim()}>Add</button>
    </form>

    {#if loading && !data}
      <p class="muted">Loading…</p>
    {:else if data}
      {#if data.incoming.length > 0}
        <p class="label">Requests received</p>
        <ul class="list">
          {#each data.incoming as person (person.profileId)}
            <li>
              <img class="face" src={skinFace(person.profileId, 30)} alt="" />
              <span class="name">{person.name}</span>
              <button class="ic accept" title="Accept" disabled={busy}
                onclick={() => mutate({ profileId: person.profileId, add: true }, `${person.name} added.`)}>
                <Icon name="check" size={13} />
              </button>
              <button class="ic decline" title="Decline" disabled={busy}
                onclick={() => mutate({ profileId: person.profileId, add: false })}>
                <Icon name="close" size={13} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if data.friends.length > 0}
        <p class="label">Your friends</p>
        <ul class="list">
          {#each data.friends as friend (friend.profileId)}
            <li>
              <img class="face" src={skinFace(friend.profileId, 30)} alt="" />
              <span class="name">{friend.name}</span>
              {#if isOnline(friend.profileId)}<span class="dot" title="Online in Cactus"></span>{/if}
              <button class="ic remove" title="Remove friend" disabled={busy}
                onclick={() => mutate({ profileId: friend.profileId, add: false }, `${friend.name} removed.`)}>
                <Icon name="close" size={13} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if data.outgoing.length > 0}
        <p class="label">Requests sent</p>
        <ul class="list">
          {#each data.outgoing as person (person.profileId)}
            <li>
              <img class="face" src={skinFace(person.profileId, 30)} alt="" />
              <span class="name">{person.name}</span>
              <span class="pending">Pending</span>
              <button class="ic remove" title="Cancel request" disabled={busy}
                onclick={() => mutate({ profileId: person.profileId, add: false })}>
                <Icon name="close" size={13} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if data.empty}
        <p class="muted">No friends yet — add one above, or add friends in Minecraft.</p>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .friends {
    margin-bottom: 18px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }
  .head h3 {
    font-size: 14px;
  }
  .refresh {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }
  .refresh:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .refresh:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .prefs {
    display: flex;
    gap: 16px;
    margin-bottom: 10px;
  }
  .pref {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .warn {
    margin-bottom: 10px;
    color: var(--warning);
  }
  .add {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }
  .add input {
    flex: 1;
    padding: 7px 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .add input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin: 8px 0 6px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    list-style: none;
    margin-bottom: 6px;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 9px;
    background: var(--bg-card);
    border: 2px solid var(--border);
  }
  .face {
    width: 26px;
    height: 26px;
    image-rendering: pixelated;
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
  .name {
    flex: 1;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #57c84a;
    box-shadow: 0 0 5px rgba(87, 200, 74, 0.9);
    flex-shrink: 0;
  }
  .pending {
    font-size: 11px;
    color: var(--text-muted);
  }
  .ic {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
  }
  .ic:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--accent);
  }
  .ic.accept:hover:not(:disabled) {
    color: #57c84a;
    border-color: #57c84a;
  }
  .ic.decline:hover:not(:disabled),
  .ic.remove:hover:not(:disabled) {
    color: var(--danger);
    border-color: var(--danger);
  }
  .ic:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
