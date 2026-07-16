<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import SkinViewer from "./SkinViewer.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { skinFace } from "$lib/skin";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
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

  // --- Skin viewer / changer (active Microsoft account) ---
  const active = $derived(accountsStore.active);
  let mode = $state<"3d" | "2d">("3d");
  let variant = $state<"classic" | "slim">("classic");
  let skinData = $state(""); // data URI for the 3D viewer (fetched to avoid CORS)
  let capeData = $state(""); // active cape as a data URI
  let capes = $state<{ id: string; alias: string; url: string; active: boolean }[]>([]);
  let skinInput = $state<HTMLInputElement>();
  let changing = $state(false);
  let dragOver = $state(false);
  let skinMsg = $state<string | null>(null);

  // Load the current skin (via Rust to dodge cross-origin WebGL taint).
  $effect(() => {
    const a = active;
    if (!open || !a) {
      skinData = "";
      return;
    }
    api
      .downloadImage(`https://minotar.net/skin/${a.uuid}`)
      .then((d) => (skinData = d))
      .catch(() => (skinData = ""));
  });

  // Load capes the account owns.
  $effect(() => {
    const a = active;
    if (!open || !a) {
      capes = [];
      capeData = "";
      return;
    }
    api
      .getCapes()
      .then((cs) => {
        capes = cs;
        loadActiveCape(cs);
      })
      .catch(() => (capes = []));
  });

  function loadActiveCape(cs: typeof capes) {
    const activeCape = cs.find((c) => c.active);
    if (activeCape) {
      api
        .downloadImage(activeCape.url)
        .then((d) => (capeData = d))
        .catch(() => (capeData = ""));
    } else {
      capeData = "";
    }
  }

  async function chooseCape(id: string | null) {
    try {
      await api.setCape(id);
      capes = capes.map((c) => ({ ...c, active: c.id === id }));
      loadActiveCape(capes);
    } catch (e) {
      toast.error(String(e));
    }
  }

  function fileToDataUrl(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(r.result as string);
      r.onerror = () => reject(r.error);
      r.readAsDataURL(file);
    });
  }

  async function applySkinFile(file: File) {
    if (!active) return;
    changing = true;
    skinMsg = null;
    try {
      const buf = await file.arrayBuffer();
      await api.setSkin(Array.from(new Uint8Array(buf)), variant);
      skinData = await fileToDataUrl(file); // instant preview
      skinMsg = "Skin updated ✓";
      setTimeout(() => (skinMsg = null), 4000);
    } catch (err) {
      toast.error(String(err));
    } finally {
      changing = false;
    }
  }

  function onSkinFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) applySkinFile(file);
  }

  function onSkinDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const file = e.dataTransfer?.files?.[0];
    if (file && file.type === "image/png") applySkinFile(file);
  }

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch {
      /* user can copy the link manually */
    }
  }
</script>

<Modal title="Accounts" {open} {onClose} width={460}>
  {#if active}
    <div class="skin-panel">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="skin-stage"
        class:drag={dragOver}
        ondragover={(e) => {
          e.preventDefault();
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
            src={`https://minotar.net/armor/body/${active.uuid}/210.png`}
            alt="{active.username}'s skin"
          />
        {/if}
        {#if dragOver}<div class="drop-hint">Drop a PNG skin</div>{/if}
      </div>
      <div class="skin-controls">
        <div class="mode-toggle">
          <button class:on={mode === "3d"} onclick={() => (mode = "3d")}>3D</button>
          <button class:on={mode === "2d"} onclick={() => (mode = "2d")}>2D</button>
        </div>
        <span class="skin-label">Model</span>
        <select class="select" bind:value={variant}>
          <option value="classic">Classic (Steve)</option>
          <option value="slim">Slim (Alex)</option>
        </select>
        {#if capes.length}
          <span class="skin-label">Cape</span>
          <div class="cape-row">
            <button
              class="cape-btn"
              class:on={!capes.some((c) => c.active)}
              onclick={() => chooseCape(null)}>None</button
            >
            {#each capes as c (c.id)}
              <button class="cape-btn" class:on={c.active} onclick={() => chooseCape(c.id)}>
                {c.alias}
              </button>
            {/each}
          </div>
        {/if}
        <button
          class="btn primary"
          disabled={changing}
          onclick={() => skinInput?.click()}
        >
          <Icon name="edit" size={14} />
          {changing ? "Applying…" : "Change skin…"}
        </button>
        {#if skinMsg}<p class="skin-msg">{skinMsg}</p>{/if}
        <p class="skin-hint">Upload or drop a 64×64 PNG skin.</p>
      </div>
      <input
        bind:this={skinInput}
        type="file"
        accept="image/png"
        style="display:none"
        onchange={onSkinFile}
      />
    </div>
  {/if}

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
          <img class="avatar" src={skinFace(acc.uuid, 36)} alt={acc.username} />
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
    gap: 16px;
    padding: 14px;
    margin-bottom: 16px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .skin-stage {
    position: relative;
    width: 170px;
    height: 250px;
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
    margin: 0;
    font-size: 12px;
    color: var(--accent);
  }
  .skin-hint {
    margin: 0;
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
