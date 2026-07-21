<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import SkinViewer from "./SkinViewer.svelte";
  import SkinEditor from "./SkinEditor.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { skinFace } from "$lib/skin";
  import { skinCache } from "$lib/stores/skins.svelte";
  import Select from "./Select.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
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
  let editorOpen = $state(false);
  let mode = $state<"3d" | "2d">("3d");
  let variant = $state<"classic" | "slim">("classic");
  const VARIANTS = $derived<{ value: "classic" | "slim"; label: string }[]>([
    { value: "classic", label: t("account.modelClassic") },
    { value: "slim", label: t("account.modelSlim") },
  ]);
  let skinData = $state(""); // data URI for the 3D viewer (fetched to avoid CORS)
  let capeData = $state("");
  let capes = $state<{ id: string; alias: string; url: string; active: boolean }[]>([]);
  let skinInput = $state<HTMLInputElement>();
  let changing = $state(false);
  let dragOver = $state(false);
  let skinMsg = $state<string | null>(null);

  // Load the current skin (via Rust to dodge cross-origin WebGL taint).
  $effect(() => {
    const activeAccount = active;
    if (!open || !activeAccount) {
      skinData = "";
      return;
    }
    const cached = skinCache.getFull(activeAccount.uuid);
    if (cached) {
      skinData = cached;
      return;
    }
    let cancelled = false;
    api
      .downloadImage(`https://minotar.net/skin/${activeAccount.uuid}`)
      .then((dataUri) => !cancelled && (skinData = dataUri))
      .catch(() => !cancelled && (skinData = ""));
    return () => (cancelled = true);
  });

  $effect(() => {
    const activeAccount = active;
    if (!open || !activeAccount) {
      capes = [];
      capeData = "";
      return;
    }
    let cancelled = false;
    api
      .getCapes()
      .then((ownedCapes) => {
        if (cancelled) return;
        capes = ownedCapes;
        loadActiveCape(ownedCapes);
      })
      .catch(() => !cancelled && (capes = []));
    return () => (cancelled = true);
  });

  function loadActiveCape(capeList: typeof capes) {
    const activeCape = capeList.find((cape) => cape.active);
    if (activeCape) {
      api
        .downloadImage(activeCape.url)
        .then((dataUri) => (capeData = dataUri))
        .catch(() => (capeData = ""));
    } else {
      capeData = "";
    }
  }

  async function chooseCape(id: string | null) {
    try {
      await api.setCape(id);
      capes = capes.map((cape) => ({ ...cape, active: cape.id === id }));
      loadActiveCape(capes);
    } catch (error) {
      toast.error(String(error));
    }
  }

  function fileToDataUrl(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  async function applySkinFile(file: File) {
    if (!active) return;
    changing = true;
    skinMsg = null;
    try {
      const buffer = await file.arrayBuffer();
      await api.setSkin(Array.from(new Uint8Array(buffer)), variant);
      skinData = await fileToDataUrl(file);
      skinCache.setSkin(active.uuid, skinData);
      skinMsg = t("account.skinUpdated");
      setTimeout(() => (skinMsg = null), 4000);
    } catch (err) {
      toast.error(String(err));
    } finally {
      changing = false;
    }
  }

  async function resetSkin() {
    if (!active) return;
    changing = true;
    skinMsg = null;
    try {
      await api.resetSkin();
      skinCache.clear(active.uuid);
      skinData = await api
        .downloadImage(`https://minotar.net/skin/${active.uuid}?ts=${Date.now()}`)
        .catch(() => "");
      skinMsg = t("account.skinReset");
      setTimeout(() => (skinMsg = null), 4000);
    } catch (err) {
      toast.error(String(err));
    } finally {
      changing = false;
    }
  }

  function onSkinFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) applySkinFile(file);
  }

  function onSkinDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;
    const file = event.dataTransfer?.files?.[0];
    if (file && file.type === "image/png") applySkinFile(file);
  }

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
    <div class="skin-panel">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="skin-stage"
        class:drag={dragOver}
        ondragover={(event) => {
          event.preventDefault();
          dragOver = true;
        }}
        ondragleave={() => (dragOver = false)}
        ondrop={onSkinDrop}
      >
        {#if mode === "3d"}
          {#if skinData}
            <SkinViewer skin={skinData} model={variant} cape={capeData} width={170} height={250} />
          {:else}
            <div class="skin-loading"><span class="spinner"></span></div>
          {/if}
        {:else}
          <img
            class="skin-2d"
            src={skinCache.getBody(active.uuid) ?? `https://minotar.net/armor/body/${active.uuid}/210.png`}
            alt={t("account.skinAlt", { username: active.username })}
          />
        {/if}
        {#if dragOver}<div class="drop-hint">{t("account.dropPngSkin")}</div>{/if}
      </div>
      <div class="skin-controls">
        <div class="mode-toggle">
          <button class:on={mode === "3d"} onclick={() => (mode = "3d")}>3D</button>
          <button class:on={mode === "2d"} onclick={() => (mode = "2d")}>2D</button>
        </div>
        <span class="skin-label">{t("account.model")}</span>
        <Select bind:value={variant} options={VARIANTS} ariaLabel={t("account.model")} />
        {#if capes.length}
          <span class="skin-label">{t("account.cape")}</span>
          <div class="cape-row">
            <button
              class="cape-btn"
              class:on={!capes.some((cape) => cape.active)}
              onclick={() => chooseCape(null)}>{t("account.capeNone")}</button
            >
            {#each capes as cape (cape.id)}
              <button class="cape-btn" class:on={cape.active} onclick={() => chooseCape(cape.id)}>
                {cape.alias}
              </button>
            {/each}
          </div>
        {/if}
        <div class="skin-actions">
          <button
            class="btn primary"
            disabled={changing}
            onclick={() => skinInput?.click()}
          >
            <Icon name="upload" size={14} />
            {changing ? t("account.applying") : t("account.changeSkin")}
          </button>
          <button class="btn" onclick={() => (editorOpen = true)}>
            <Icon name="edit" size={14} /> {t("skinEditor.drawSkin")}
          </button>
          <button class="btn ghost sm" disabled={changing} onclick={resetSkin}>
            <Icon name="refresh" size={13} /> {t("account.resetSkin")}
          </button>
        </div>
      </div>
      <input
        bind:this={skinInput}
        type="file"
        accept="image/png"
        style="display:none"
        onchange={onSkinFile}
      />
    </div>
    {#if skinMsg}<p class="skin-msg">{skinMsg}</p>{/if}
    <p class="skin-hint">{t("account.skinUploadHint")}</p>
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

{#if active}
  <SkinEditor
    open={editorOpen}
    onClose={() => (editorOpen = false)}
    uuid={active.uuid}
    {variant}
    onApplied={(uri) => {
      skinData = uri;
      if (active) skinCache.setSkin(active.uuid, uri);
    }}
  />
{/if}

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
  .skin-panel {
    display: flex;
    align-items: stretch;
    gap: 16px;
    padding: 14px;
    margin-bottom: 8px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .skin-stage {
    position: relative;
    width: 170px;
    min-height: 250px;
    align-self: stretch;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: radial-gradient(circle at 50% 35%, var(--bg-card), var(--bg-app));
    border: 2px solid var(--border-subtle);
  }
  .skin-stage.drag {
    border-color: var(--accent);
    border-style: dashed;
  }
  .drop-hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.55);
    color: var(--accent);
    font-size: 13px;
    pointer-events: none;
  }
  .skin-2d {
    max-height: 230px;
    image-rendering: pixelated;
  }
  .skin-loading {
    color: var(--text-muted);
  }
  .spinner {
    width: 22px;
    height: 22px;
    border: 3px solid rgba(255, 255, 255, 0.25);
    border-top-color: var(--accent);
    border-radius: 50%;
    display: inline-block;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .skin-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-width: 0;
    justify-content: center;
  }
  .skin-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mode-toggle {
    display: flex;
    border: 2px solid var(--border);
    align-self: flex-start;
  }
  .mode-toggle button {
    padding: 5px 14px;
    background: var(--bg-card);
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-pixel);
    font-size: 12px;
  }
  .mode-toggle button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .skin-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-top: 4px;
  }
  .cape-row {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .cape-btn {
    padding: 4px 8px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 11.5px;
  }
  .cape-btn.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .skin-msg {
    margin: 0 0 2px;
    font-size: 12px;
    color: var(--accent);
  }
  .skin-hint {
    margin: 0 0 16px;
    font-size: 11px;
    color: var(--text-muted);
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
  }
  .err {
    margin: 12px 0 0;
    color: var(--danger);
    font-size: 12.5px;
  }
</style>
