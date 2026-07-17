<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { readJson, writeJson } from "$lib/storage";
  import Icon from "./Icon.svelte";

  const TOKEN_KEY = "cactus:ngrokToken";
  const AUTHTOKEN_URL = "https://dashboard.ngrok.com/get-started/your-authtoken";

  // The host's own ngrok authtoken — remembered locally so it's entered once.
  let token = $state(readJson<string>(TOKEN_KEY, ""));
  let port = $state(25565);
  let address = $state(""); // public host:port while a tunnel is up
  let busy = $state(false);
  let error = $state<string | null>(null);

  const running = $derived(address !== "");

  async function start() {
    if (!token.trim()) return;
    busy = true;
    error = null;
    try {
      writeJson(TOKEN_KEY, token.trim());
      address = await api.tunnelStart(token.trim(), port);
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

  function copy(text: string) {
    navigator.clipboard.writeText(text);
    toast.success("Copied.");
  }
</script>

<section class="tunnel">
  <div class="head">
    <Icon name="globe" size={14} />
    <div class="head-text">
      <strong>Play over the internet</strong>
      <small>Share your server through ngrok — no port-forwarding or Hamachi.</small>
    </div>
  </div>

  {#if running}
    <p class="hint">Live — friends Direct Connect in Minecraft to:</p>
    <div class="row">
      <code class="code">{address}</code>
      <button class="btn ghost sm" onclick={() => copy(address)}>Copy</button>
      <button class="btn ghost sm" onclick={stop}>Stop</button>
    </div>
    <small class="note">Keep the launcher open and your server running on port {port}.</small>
  {:else}
    <label class="field">
      <span>
        ngrok authtoken
        <button class="link" onclick={() => openUrl(AUTHTOKEN_URL)}>get yours ↗</button>
      </span>
      <input
        class="input"
        type="password"
        placeholder="Paste your ngrok authtoken"
        bind:value={token}
        autocomplete="off"
      />
    </label>
    <div class="row">
      <label class="port">
        server port
        <input class="input" type="number" min="1" max="65535" bind:value={port} />
      </label>
      <button class="btn primary sm" onclick={start} disabled={busy || !token.trim()}>
        {busy ? "Starting…" : "Start sharing"}
      </button>
    </div>
  {/if}

  {#if error}<p class="err">{error}</p>{/if}
</section>

<style>
  .tunnel {
    padding: 12px 14px;
    border: 2px solid var(--border);
    background: var(--bg-card);
    margin-bottom: 14px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    color: var(--text-secondary);
  }
  .head-text {
    display: flex;
    flex-direction: column;
  }
  .head-text small {
    color: var(--text-muted);
    font-size: 11.5px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }
  .field > span {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-muted);
  }
  .link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
    padding: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .port {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    margin-right: auto;
  }
  .port .input {
    width: 84px;
  }
  .hint {
    margin: 0 0 6px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .code {
    flex: 1;
    padding: 6px 10px;
    font-family: var(--font-pixel);
    color: var(--accent);
    background: var(--bg-input);
    border: 2px solid var(--border-subtle);
    user-select: all;
  }
  .note {
    color: var(--text-muted);
    font-size: 11px;
  }
  .err {
    margin: 6px 0 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
