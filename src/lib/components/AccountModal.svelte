<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import SkinPanel from "./SkinPanel.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { skinFace } from "$lib/skin";
  import { copyText } from "$lib/clipboard";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();

  const offlineName = $derived(
    settingsStore.settings.offlineUsername || t("account.defaultPlayer")
  );
  const deviceCode = $derived(accountsStore.deviceCode);
  const active = $derived(accountsStore.active);

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch {
    }
  }

  const copyCode = (code: string) => copyText(code, t("account.codeCopied"));

  function openAchievements() {
    onClose();
    goto("/achievements");
  }
</script>

<Modal title={t("account.title")} {open} {onClose} width={460}>
  {#if active}
    {#key active.id}
      <SkinPanel account={active} />
    {/key}
  {/if}

  <div class="list">
    <button
      class="row"
      class:active={accountsStore.activeId === null}
      onclick={() => accountsStore.setActive(null)}
    >
      <span class="avatar offline"><Icon name="user" size={18} /></span>
      <span class="info">
        <span class="name">{offlineName}</span>
        <span class="kind">{t("account.offline")}</span>
      </span>
      {#if accountsStore.activeId === null}
        <span class="badge">{t("account.active")}</span>
      {/if}
    </button>

    {#if !accountsStore.loaded}
      <div class="row skel-row">
        <span class="skeleton" style="width:36px;height:36px"></span>
        <span class="info">
          <span class="skeleton" style="width:100px;height:13px"></span>
          <span class="skeleton" style="width:60px;height:11px;margin-top:4px"></span>
        </span>
      </div>
    {:else}
      {#each accountsStore.accounts as acc (acc.id)}
        <div class="row" class:active={accountsStore.activeId === acc.id}>
          <button class="row-main" onclick={() => accountsStore.setActive(acc.id)}>
            <img class="avatar" src={skinFace(acc.uuid, 36)} alt={acc.username} />
            <span class="info">
              <span class="name">{acc.username}</span>
              <span class="kind">Microsoft</span>
            </span>
          </button>
          {#if accountsStore.activeId === acc.id}
            <span class="badge">{t("account.active")}</span>
          {/if}
          <button
            class="remove"
            title={t("account.removeAccount")}
            onclick={() => accountsStore.remove(acc.id)}
          >
            <Icon name="trash" size={15} />
          </button>
        </div>
      {/each}
    {/if}
  </div>

  <div class="add-area">
    {#if !accountsStore.microsoftConfigured}
      <div class="notice">
        {t("account.notConfigured")}
        <code>src-tauri/src/auth/mod.rs</code>
        {t("account.notConfiguredSuffix")}
      </div>
    {:else if deviceCode}
      <div class="device-code">
        <p class="dc-title">{t("account.signInMicrosoft")}</p>
        <p class="dc-instructions">
          {t("account.enterCode")}
        </p>
        <div class="code-row">
          <button
            class="code"
            title={t("account.clickToCopy")}
            onclick={() => copyCode(deviceCode.userCode)}
          >
            {deviceCode.userCode}
          </button>
          <button class="btn ghost sm" onclick={() => copyCode(deviceCode.userCode)}>
            <Icon name="copy" size={14} /> {t("account.copy")}
          </button>
        </div>
        <button class="btn primary" onclick={() => openLink(deviceCode.verificationUri)}>
          {t("account.openUri", { uri: deviceCode.verificationUri })}
        </button>
        <p class="dc-status">
          <span class="spinner"></span>
          {deviceCode.status === "authorizing"
            ? t("account.signingIn")
            : t("account.waitingAuthorize")}
        </p>
        {#if deviceCode.status !== "authorizing"}
          <button class="btn ghost sm dc-cancel" onclick={() => accountsStore.cancelLogin()}>
            {t("common.cancel")}
          </button>
        {/if}
      </div>
    {:else}
      <button
        class="btn primary add-btn"
        onclick={() => accountsStore.login()}
        disabled={accountsStore.loggingIn}
      >
        <Icon name="plus" size={16} />
        {accountsStore.loggingIn ? t("account.starting") : t("account.addMicrosoft")}
      </button>
    {/if}

    {#if accountsStore.loginError}
      <p class="err">{accountsStore.loginError}</p>
    {/if}
  </div>

  <div class="achv-area">
    <button class="btn ghost achv-btn" onclick={openAchievements}>
      <Icon name="trophy" size={15} />
      {t("account.achievementsStats")}
    </button>
    <p class="achv-hint">{t("account.achievementsHint")}</p>
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
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text);
    text-align: left;
    transition: border-color 0.12s, background 0.12s;
  }
  button.row:hover,
  .row:hover {
    border-color: var(--accent);
  }
  .skel-row {
    align-items: center;
    gap: 12px;
    pointer-events: none;
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
    border-radius: 0;
    border: 2px solid rgba(0, 0, 0, 0.3);
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
    font-family: var(--font-pixel);
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    border: 2px solid var(--accent);
    border-radius: 0;
    padding: 3px 9px;
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
  .achv-area {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border-subtle);
    text-align: center;
  }
  .achv-btn {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .achv-hint {
    margin: 8px 0 0;
    font-size: 11px;
    color: var(--text-muted);
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
  .code-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .code {
    font-family: var(--font-pixel), "SF Mono", Menlo, monospace;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--accent);
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.3);
    padding: 10px 20px;
    cursor: pointer;
    user-select: all;
  }
  .code:hover {
    border-color: var(--accent);
  }
  .dc-status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 12.5px;
    margin: 4px 0 0;
  }
  .dc-status .spinner {
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
