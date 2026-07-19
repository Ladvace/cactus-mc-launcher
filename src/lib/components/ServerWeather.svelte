<script lang="ts">
  import { api } from "$lib/api";
  import { boardApi } from "$lib/boardApi";
  import Sparkline from "./Sparkline.svelte";
  import type { ServerStatus, ServerSample } from "$lib/types";

  let { address }: { address: string } = $props();

  let status = $state<ServerStatus | null>(null);
  let loading = $state(false);
  let failed = $state(false);
  let history = $state<ServerSample[]>([]);

  const peak = $derived(
    history.reduce((most, sample) => Math.max(most, sample.online ?? 0), 0)
  );

  async function refresh() {
    if (!address) return;
    loading = true;
    failed = false;
    try {
      status = await api.pingServer(address);
    } catch {
      status = null;
      failed = true;
    } finally {
      loading = false;
    }
  }

  async function loadHistory() {
    if (!address || !boardApi.configured()) return;
    try {
      history = await boardApi.serverHistory(address, 24);
    } catch {
      history = [];
    }
  }

  // Read address synchronously so the effect tracks it as a dependency.
  $effect(() => {
    void address;
    refresh();
    loadHistory();
  });
</script>

<div class="weather" title={status?.motd ?? ""}>
  {#if loading && !status}
    <span class="dot pending"></span>
    <span class="label">Checking…</span>
  {:else if status}
    <span class="dot online"></span>
    <span class="count">{status.online}<span class="sep">/</span>{status.max}</span>
    <span class="label">online</span>
    {#if status.players.length}
      <span class="sample" title={status.players.join(", ")}>
        · {status.players.slice(0, 3).join(", ")}{status.players.length > 3
          ? "…"
          : ""}
      </span>
    {/if}
    <span class="ping">{status.pingMs} ms</span>
  {:else if failed}
    <span class="dot off"></span>
    <span class="label">Offline</span>
  {/if}
  <button class="refresh" title="Refresh" onclick={refresh} disabled={loading}>
    ↻
  </button>
</div>

{#if history.length > 1}
  <div class="history">
    <Sparkline values={history.map((sample) => sample.online)} />
    <div class="history-meta">
      <span>last 24h</span>
      <span>peak {peak}</span>
    </div>
  </div>
{/if}

<style>
  .weather {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.online {
    background: #5b8a3a;
    box-shadow: 0 0 0 3px rgba(91, 138, 58, 0.2);
  }
  .dot.off {
    background: var(--text-muted);
  }
  .dot.pending {
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  .count {
    font-family: var(--font-pixel);
    color: var(--text);
  }
  .sep {
    color: var(--text-muted);
    margin: 0 1px;
  }
  .sample {
    color: var(--text-muted);
    max-width: 40ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ping {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-muted);
  }
  .refresh {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 13px;
    padding: 2px 4px;
  }
  .refresh:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .history {
    margin-top: 8px;
  }
  .history-meta {
    display: flex;
    justify-content: space-between;
    margin-top: 2px;
    font-size: 10.5px;
    color: var(--text-muted);
  }
</style>
