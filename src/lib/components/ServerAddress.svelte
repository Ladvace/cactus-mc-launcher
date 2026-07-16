<script lang="ts">
  import Icon from "./Icon.svelte";
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { DEFAULT_PORT, parseServerPort, formatAddress } from "$lib/serverAddress";

  let { id }: { id: string } = $props();

  let port = $state(DEFAULT_PORT);
  let lanIp = $state<string | null>(null);
  let copied = $state<string | null>(null);

  let lastId = "";
  $effect(() => {
    if (id && id !== lastId) {
      lastId = id;
      load();
    }
  });

  async function load() {
    try {
      port = parseServerPort(await api.readServerProperties(id));
    } catch {
      port = DEFAULT_PORT;
    }
    try {
      lanIp = await api.getLocalIp();
    } catch {
      lanIp = null;
    }
  }

  const localAddr = $derived(formatAddress("localhost", port));
  const lanAddr = $derived(lanIp ? formatAddress(lanIp, port) : null);

  async function copy(addr: string) {
    try {
      await navigator.clipboard.writeText(addr);
      copied = addr;
      setTimeout(() => copied === addr && (copied = null), 1500);
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

<div class="addr">
  <span class="addr-label"><Icon name="globe" size={13} /> Server address</span>
  <button class="chip" title="Copy" onclick={() => copy(localAddr)}>
    <span class="host">{localAddr}</span>
    <span class="tag">this PC</span>
    <Icon name={copied === localAddr ? "check" : "copy"} size={13} />
  </button>
  {#if lanAddr}
    <button class="chip" title="Copy" onclick={() => copy(lanAddr)}>
      <span class="host">{lanAddr}</span>
      <span class="tag">LAN</span>
      <Icon name={copied === lanAddr ? "check" : "copy"} size={13} />
    </button>
  {/if}
</div>

<style>
  .addr {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }
  .addr-label {
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
    font-family: "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
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
</style>
