<script lang="ts">
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { copyText } from "$lib/clipboard";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { DEFAULT_PORT, parseServerPort } from "$lib/serverAddress";

  let { id }: { id: string } = $props();

  const AUTHTOKEN_URL = "https://dashboard.ngrok.com/get-started/your-authtoken";

  const instance = $derived(instancesStore.get(id));
  const globalToken = $derived((settingsStore.settings.ngrokAuthtoken ?? "").trim());
  const instanceToken = $derived((instance?.ngrokAuthtoken ?? "").trim());
  // A per-instance token wins over the global one from Settings.
  const effectiveToken = $derived(instanceToken || globalToken);

  let port = $state(DEFAULT_PORT);
  let address = $state(""); // public host:port while sharing
  let busy = $state(false);
  let error = $state<string | null>(null);
  let editingToken = $state(false);
  let tokenDraft = $state("");

  let lastId = "";
  $effect(() => {
    if (id && id !== lastId) {
      lastId = id;
      loadPort();
      // The tunnel is global (one at a time); restore its state so navigating
      // away and back doesn't lose the running address / Stop button.
      api.tunnelStatus().then((running) => (address = running ?? "")).catch(() => {});
    }
  });
  async function loadPort() {
    try {
      port = parseServerPort(await api.readServerProperties(id));
    } catch {
      port = DEFAULT_PORT;
    }
  }

  async function start() {
    if (!effectiveToken) {
      tokenDraft = "";
      editingToken = true;
      return;
    }
    busy = true;
    error = null;
    try {
      address = await api.tunnelStart(effectiveToken, port);
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    try {
      await api.tunnelStop();
    } catch {
      /* nothing running */
    }
    address = "";
  }

  async function saveToken() {
    const token = tokenDraft.trim();
    if (!token) return;
    await instancesStore.update(id, { ngrokAuthtoken: token });
    editingToken = false;
    tokenDraft = "";
    start();
  }

  async function clearInstanceToken() {
    await instancesStore.update(id, { ngrokAuthtoken: "" });
  }
</script>

<div class="share">
  <span class="label"><Icon name="globe" size={13} /> Share online</span>

  {#if address}
    <button class="chip" title="Copy" onclick={() => copyText(address)}>
      <span class="host">{address}</span>
      <span class="tag">internet</span>
      <Icon name="copy" size={13} />
    </button>
    <button class="btn ghost sm" onclick={stop}>Stop</button>
    <span class="warn">Anyone with this address can join — turn on the whitelist in Properties.</span>
  {:else if editingToken}
    <input
      class="input token"
      type="password"
      placeholder="ngrok authtoken (saved to this server)"
      bind:value={tokenDraft}
      onkeydown={(event) => event.key === "Enter" && saveToken()}
    />
    <button class="btn primary sm" disabled={!tokenDraft.trim()} onclick={saveToken}>
      Save &amp; share
    </button>
    <button class="linkish" onclick={() => openUrl(AUTHTOKEN_URL)}>get one ↗</button>
  {:else}
    <button class="btn primary sm" onclick={start} disabled={busy}>
      {busy ? "Starting…" : "Share via ngrok"}
    </button>
    {#if effectiveToken}
      <span class="src">
        key: {instanceToken ? "this server" : "Settings"}
        {#if instanceToken}
          <button class="linkish" onclick={clearInstanceToken}>use global</button>
        {/if}
      </span>
    {:else}
      <span class="src muted">add an ngrok key (here or in Settings)</span>
    {/if}
  {/if}

  {#if error}<span class="err">{error}</span>{/if}
</div>

<style>
  .share {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 10px;
  }
  .label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    transition: border-color 0.12s, color 0.12s;
  }
  .chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .host {
    font-family: var(--font-mono);
    font-size: 13px;
    user-select: text;
  }
  .tag {
    font-family: var(--font-pixel);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    border: 1px solid var(--border-subtle);
    padding: 1px 4px;
  }
  .token {
    width: 260px;
    max-width: 60vw;
  }
  .src {
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  .linkish {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 11.5px;
    padding: 0;
  }
  .err {
    font-size: 12px;
    color: var(--danger);
    flex-basis: 100%;
  }
  .warn {
    flex-basis: 100%;
    font-size: 11.5px;
    color: var(--text-muted);
  }
</style>
