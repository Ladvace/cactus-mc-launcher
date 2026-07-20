<script lang="ts">
  import { api } from "$lib/api";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { presence } from "$lib/stores/presence.svelte";
  import { skinFace } from "$lib/skin";
  import Icon from "./Icon.svelte";
  import type { FriendsList } from "$lib/types";

  let data = $state<FriendsList | null>(null);
  let loading = $state(false);
  let failed = $state(false);

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
    } catch {
      failed = true;
      data = null;
    } finally {
      loading = false;
    }
  }

  // Reload when the active account changes.
  $effect(() => {
    account;
    load();
  });

  // Cross-reference with Cactus presence (uuids there are dashless).
  const onlineUuids = $derived(new Set(presence.players.map((p) => p.uuid)));
  const isOnline = (profileId: string) => onlineUuids.has(profileId.replace(/-/g, ""));
</script>

<!-- The friends API is new/unstable; if the account can't use it, stay quiet. -->
{#if account && !failed}
  <section class="friends">
    <div class="head">
      <h3>Friends</h3>
      {#if data && data.incoming.length > 0}
        <span class="badge">{data.incoming.length} request{data.incoming.length > 1 ? "s" : ""}</span>
      {/if}
      <button class="refresh" title="Refresh" onclick={load} disabled={loading}>
        <Icon name="refresh" size={12} />
      </button>
    </div>

    {#if loading && !data}
      <p class="muted">Loading…</p>
    {:else if data && data.friends.length > 0}
      <ul class="list">
        {#each data.friends as friend (friend.profileId)}
          <li>
            <img class="face" src={skinFace(friend.profileId, 30)} alt="" />
            <span class="name">{friend.name}</span>
            {#if isOnline(friend.profileId)}
              <span class="dot" title="Online in Cactus"></span>
            {/if}
          </li>
        {/each}
      </ul>
    {:else if data}
      <p class="muted">No friends yet — add friends in Minecraft to see them here.</p>
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
  .badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: var(--accent-soft);
    padding: 2px 7px;
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
  .muted {
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 6px;
    list-style: none;
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
  }
  .name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    margin-left: auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #57c84a;
    box-shadow: 0 0 5px rgba(87, 200, 74, 0.9);
    flex-shrink: 0;
  }
</style>
