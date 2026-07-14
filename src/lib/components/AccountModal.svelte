<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();

  const offlineName = $derived(
    settingsStore.settings.offlineUsername || "Player"
  );
  const dc = $derived(accountsStore.deviceCode);

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch {
      /* user can copy the link manually */
    }
  }
</script>

<Modal title="Accounts" {open} {onClose} width={460}>
  <div class="list">
    <!-- Offline -->
    <button
      class="row"
      class:active={accountsStore.activeId === null}
      onclick={() => accountsStore.setActive(null)}
    >
      <span class="avatar offline"><Icon name="user" size={18} /></span>
      <span class="info">
        <span class="name">{offlineName}</span>
        <span class="kind">Offline</span>
      </span>
      {#if accountsStore.activeId === null}
        <span class="badge">Active</span>
      {/if}
    </button>

    <!-- Microsoft accounts -->
    {#each accountsStore.accounts as acc (acc.id)}
      <div class="row" class:active={accountsStore.activeId === acc.id}>
        <button class="row-main" onclick={() => accountsStore.setActive(acc.id)}>
          <img
            class="avatar"
            src={`https://crafatar.com/avatars/${acc.uuid}?size=36&overlay`}
            alt={acc.username}
          />
          <span class="info">
            <span class="name">{acc.username}</span>
            <span class="kind">Microsoft</span>
          </span>
        </button>
        {#if accountsStore.activeId === acc.id}
          <span class="badge">Active</span>
        {/if}
        <button
          class="remove"
          title="Remove account"
          onclick={() => accountsStore.remove(acc.id)}
        >
          <Icon name="trash" size={15} />
        </button>
      </div>
    {/each}
  </div>

  <!-- Add / login -->
  <div class="add-area">
    {#if !accountsStore.microsoftConfigured}
      <div class="notice">
        Microsoft sign-in isn't configured. Add your Azure client ID in
        <code>src-tauri/src/auth/mod.rs</code> to enable it.
      </div>
    {:else if dc}
      <div class="device-code">
        <p class="dc-title">Sign in to Microsoft</p>
        <p class="dc-instructions">
          Open the link and enter this code:
        </p>
        <div class="code">{dc.userCode}</div>
        <button class="btn primary" onclick={() => openLink(dc.verificationUri)}>
          Open {dc.verificationUri}
        </button>
        <p class="dc-status">
          <span class="spinner"></span>
          {dc.status === "authorizing"
            ? "Signing you in…"
            : "Waiting for you to authorize…"}
        </p>
      </div>
    {:else}
      <button
        class="btn primary add-btn"
        onclick={() => accountsStore.login()}
        disabled={accountsStore.loggingIn}
      >
        <Icon name="plus" size={16} />
        {accountsStore.loggingIn ? "Starting…" : "Add Microsoft account"}
      </button>
    {/if}

    {#if accountsStore.loginError}
      <p class="err">{accountsStore.loginError}</p>
    {/if}
  </div>
</Modal>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text);
    text-align: left;
    transition: border-color 0.12s, background 0.12s;
  }
  button.row:hover,
  .row:hover {
    border-color: var(--border);
  }
  .row.active {
    border-color: var(--accent);
  }
  .row-main {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    padding: 0;
  }
  .avatar {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: var(--bg-card);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    flex-shrink: 0;
    object-fit: cover;
    image-rendering: pixelated;
  }
  .info {
    display: flex;
    flex-direction: column;
    line-height: 1.3;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-size: 11.5px;
    color: var(--text-muted);
  }
  .badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 20px;
    padding: 2px 9px;
  }
  .remove {
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 6px;
    border-radius: var(--radius-sm);
    display: flex;
  }
  .remove:hover {
    background: rgba(255, 91, 110, 0.12);
    color: var(--danger);
  }
  .add-area {
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--border-subtle);
  }
  .add-btn {
    width: 100%;
  }
  .notice {
    font-size: 12.5px;
    color: var(--text-muted);
    line-height: 1.6;
  }
  .notice code {
    background: var(--bg-input);
    padding: 1px 5px;
    border-radius: 4px;
    font-size: 11.5px;
  }
  .device-code {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }
  .dc-title {
    margin: 0;
    font-weight: 600;
    font-size: 14px;
  }
  .dc-instructions {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .code {
    font-family: "SF Mono", Menlo, monospace;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--accent);
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 20px;
  }
  .dc-status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 12.5px;
    margin: 4px 0 0;
  }
  .spinner {
    width: 13px;
    height: 13px;
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
  .err {
    margin: 12px 0 0;
    color: var(--danger);
    font-size: 12.5px;
  }
</style>
