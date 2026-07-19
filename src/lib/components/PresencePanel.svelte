<script lang="ts">
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { presence } from "$lib/stores/presence.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { skinFace } from "$lib/skin";
  import { MOD_LOADERS } from "$lib/types";

  const online = boardApi.configured();
  const account = $derived(accountsStore.active);

  // Sign in with the active Minecraft account (no separate login).
  $effect(() => {
    const activeAccount = account;
    if (!online || !activeAccount) return;
    // Don't auto-retry after a failure (that loops forever on "Connecting…");
    // the user retries via the button, and switching account clears the error.
    if (
      !boardAuth.loggingIn &&
      !boardAuth.error &&
      (!boardAuth.signedIn || boardAuth.session?.uuid !== activeAccount.uuid)
    ) {
      boardAuth.login();
    }
  });

  // Lifecycle only: start polling on mount, stop on unmount. Wrapped in
  // untrack so the reactive reads inside open()/poll() (players, token) don't
  // turn this into a loop that re-opens (and clears presence) on every poll.
  $effect(() => {
    untrack(() => presence.open());
    return () => untrack(() => presence.close());
  });
  // Re-poll as soon as a session token arrives (depends on the token only).
  $effect(() => {
    if (boardAuth.token) untrack(() => void presence.poll());
  });

  // Local editable copies of the broadcast fields (committed on change).
  let status = $state(presence.status);
  let address = $state(presence.serverAddress);
  let mcVersion = $state(presence.mcVersion);
  let loader = $state(presence.loader);

  // Filters for the online list ("" = Any).
  let filterVersion = $state("");
  let filterLoader = $state("");

  const myUuid = $derived(boardAuth.session?.uuid ?? "");
  const me = $derived(presence.players.find((player) => player.uuid === myUuid) ?? null);

  // Versions actually present online, for the version filter dropdown.
  const versionsOnline = $derived(
    [...new Set(presence.players.map((player) => player.mcVersion).filter(Boolean))].sort() as string[]
  );

  function passes(player: import("$lib/types").PresencePlayer): boolean {
    if (filterLoader && (player.loader ?? "") !== filterLoader) return false;
    if (filterVersion && (player.mcVersion ?? "") !== filterVersion) return false;
    return true;
  }
  const others = $derived(
    presence.players.filter((player) => player.uuid !== myUuid && passes(player))
  );

  function loaderLabel(loaderValue: string | null): string {
    return MOD_LOADERS.find((option) => option.value === loaderValue)?.label ?? (loaderValue ?? "");
  }

  async function toggleOnline() {
    await presence.setEnabled(!presence.enabled);
  }
  function saveFields() {
    presence.saveFields({
      status: status.trim(),
      serverAddress: address.trim(),
      mcVersion: mcVersion.trim(),
      loader: loader,
    });
  }

  async function copyAddress(addr: string) {
    try {
      await navigator.clipboard.writeText(addr);
      toast.success(`Copied ${addr}`);
    } catch (error) {
      toast.error(String(error));
    }
  }

  function timeAgo(iso: string): string {
    const seconds = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
    if (seconds < 30) return "now";
    if (seconds < 90) return "just now";
    return `${Math.round(seconds / 60)}m ago`;
  }
</script>

{#if !online}
  <p class="offline">
    <Icon name="globe" size={13} /> Play Together is offline in this build.
  </p>
{:else if !account}
  {#if accountsStore.accounts.length > 0}
    <p class="offline">
      <Icon name="user" size={13} />
      You're in offline mode — switch to your Microsoft account to see who's online.
      <button class="link" onclick={() => ui.openAccounts()}>Switch account</button>
    </p>
  {:else}
    <p class="offline">
      <Icon name="user" size={13} /> Add a Microsoft account to see who's online.
    </p>
  {/if}
{:else if boardAuth.loggingIn && !boardAuth.signedIn}
  <p class="offline"><span class="spin"></span> Connecting…</p>
{:else if !boardAuth.signedIn}
  <p class="offline">
    <Icon name="user" size={13} />
    Couldn't connect{boardAuth.error ? `: ${boardAuth.error}` : ""}.
    <button class="link" onclick={() => boardAuth.login()}>Retry</button>
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
            placeholder="e.g. looking to play modded"
            maxlength="120"
            bind:value={status}
            onblur={saveFields}
            onkeydown={(event) => event.key === "Enter" && saveFields()}
          />
        </label>
        <label class="field short">
          <span>MC version</span>
          <input
            class="input"
            placeholder="1.20.1"
            maxlength="32"
            bind:value={mcVersion}
            onblur={saveFields}
            onkeydown={(event) => event.key === "Enter" && saveFields()}
          />
        </label>
        <label class="field short">
          <span>Loader</span>
          <select class="select" bind:value={loader} onchange={saveFields}>
            <option value="">Any</option>
            {#each MOD_LOADERS as loaderOption}
              <option value={loaderOption.value}>{loaderOption.label}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Server address (optional)</span>
          <input
            class="input"
            placeholder="e.g. play.example.net"
            maxlength="80"
            bind:value={address}
            onblur={saveFields}
            onkeydown={(event) => event.key === "Enter" && saveFields()}
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
    <div class="filters">
      <select class="select mini" bind:value={filterLoader} aria-label="Filter by loader">
        <option value="">Any loader</option>
        {#each MOD_LOADERS as loaderOption}
          <option value={loaderOption.value}>{loaderOption.label}</option>
        {/each}
      </select>
      <select class="select mini" bind:value={filterVersion} aria-label="Filter by version">
        <option value="">Any version</option>
        {#each versionsOnline as version}
          <option value={version}>{version}</option>
        {/each}
      </select>
      <button class="btn ghost sm" onclick={() => presence.poll()} disabled={presence.loading}>
        Refresh
      </button>
    </div>
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
      {#each others as player (player.uuid)}
        {@render row(player, false)}
      {/each}
    </ul>
  {/if}
{/if}

{#snippet row(player: import("$lib/types").PresencePlayer, isMe: boolean)}
  <li class="player" class:me-row={isMe}>
    <img class="face" src={skinFace(player.uuid, 32)} alt={player.name} />
    <div class="body">
      <span class="name">{player.name}{#if isMe} <span class="you">you</span>{/if}</span>
      <span class="status">{player.status || "online"}</span>
    </div>
    {#if player.mcVersion || player.loader}
      <span class="tag">{loaderLabel(player.loader)}{player.mcVersion ? ` ${player.mcVersion}` : ""}</span>
    {/if}
    <span class="ago">{timeAgo(player.updatedAt)}</span>
    {#if player.serverAddress}
      <button class="btn ghost sm" title="Copy server address" onclick={() => copyAddress(player.serverAddress!)}>
        <Icon name="copy" size={13} /> {player.serverAddress}
      </button>
    {/if}
  </li>
{/snippet}

<style>
  .offline {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .offline .link {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  .spin {
    flex-shrink: 0;
    box-sizing: border-box;
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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
  .filters {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .select.mini {
    width: auto;
    padding: 6px 28px 6px 10px;
    font-size: 12px;
    background-position: right 8px center;
  }
  .field.short {
    flex: 0 0 130px;
    min-width: 110px;
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
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .tag {
    flex-shrink: 0;
    font-family: var(--font-pixel);
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    padding: 2px 6px;
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
