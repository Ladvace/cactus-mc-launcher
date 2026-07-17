<script lang="ts">
  import { boardApi } from "$lib/boardApi";
  import { toast } from "$lib/stores/toast.svelte";
  import Icon from "./Icon.svelte";

  type Mode = "idle" | "hosting" | "joining";
  let mode = $state<Mode>("idle");
  let hostPort = $state(25565);
  let joinCode = $state("");
  let code = $state(""); // shown while hosting
  let localPort = $state(0); // shown while joining
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function host() {
    busy = true;
    error = null;
    try {
      code = await boardApi.linkHost(hostPort);
      mode = "hosting";
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  async function join() {
    const trimmed = joinCode.trim();
    if (!trimmed) return;
    busy = true;
    error = null;
    try {
      localPort = await boardApi.linkJoin(trimmed);
      mode = "joining";
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    try {
      await boardApi.linkStop();
    } catch {
      /* nothing running */
    }
    mode = "idle";
    code = "";
    localPort = 0;
    joinCode = "";
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text);
    toast.success("Copied.");
  }
</script>

<section class="link">
  <div class="head">
    <Icon name="globe" size={14} />
    <div class="head-text">
      <strong>Play over the internet</strong>
      <small>No port-forwarding or Hamachi — just share a code.</small>
    </div>
  </div>

  {#if mode === "idle"}
    <div class="row">
      <button class="btn primary sm" onclick={host} disabled={busy}>Host a session</button>
      <label class="port">
        port
        <input class="input" type="number" min="1" max="65535" bind:value={hostPort} />
      </label>
    </div>
    <div class="row">
      <input
        class="input grow"
        placeholder="Enter a code to join…"
        bind:value={joinCode}
        onkeydown={(event) => event.key === "Enter" && join()}
      />
      <button class="btn ghost sm" onclick={join} disabled={busy || !joinCode.trim()}>
        Join
      </button>
    </div>
  {:else if mode === "hosting"}
    <p class="hint">Share this code with friends:</p>
    <div class="row">
      <code class="code">{code}</code>
      <button class="btn ghost sm" onclick={() => copy(code)}>Copy</button>
      <button class="btn ghost sm" onclick={stop}>Stop</button>
    </div>
    <small class="note">
      Keep the launcher open and your server running on port {hostPort}.
    </small>
  {:else}
    <p class="hint">Connected — in Minecraft, Direct Connect to:</p>
    <div class="row">
      <code class="code">127.0.0.1:{localPort}</code>
      <button class="btn ghost sm" onclick={() => copy(`127.0.0.1:${localPort}`)}>Copy</button>
      <button class="btn ghost sm" onclick={stop}>Disconnect</button>
    </div>
  {/if}

  {#if error}<p class="err">{error}</p>{/if}
</section>

<style>
  .link {
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
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .input.grow {
    flex: 1;
  }
  .port {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }
  .port .input {
    width: 76px;
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
