<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { copyText } from "$lib/clipboard";
  import { FEATURED_SERVERS } from "$lib/servers";
  import type { ServerStatus } from "$lib/types";
  import Icon from "$lib/components/Icon.svelte";

  type Ping = { state: "loading" | "online" | "offline"; status?: ServerStatus };

  // Per-address ping state, keyed by server address.
  let pings = $state<Record<string, Ping>>({});

  function refresh() {
    for (const server of FEATURED_SERVERS) {
      pings[server.address] = { state: "loading" };
      api
        .pingServer(server.address)
        .then((status) => (pings[server.address] = { state: "online", status }))
        .catch(() => (pings[server.address] = { state: "offline" }));
    }
  }

  // Ping every server once on load (each resolves independently).
  $effect(() => {
    refresh();
  });
</script>

<div class="page">
  <header class="head">
    <div>
      <h1>Servers</h1>
      <p class="sub">Popular public servers. Copy an address and add it in Minecraft's Multiplayer menu.</p>
    </div>
    <button class="btn ghost sm" onclick={refresh} title="Refresh status">
      <Icon name="refresh" size={15} /> Refresh
    </button>
  </header>

  <div class="grid">
    {#each FEATURED_SERVERS as server (server.address)}
      {@const ping = pings[server.address]}
      <div class="card">
        <div class="card-head">
          <h2>{server.name}</h2>
          <span
            class="status"
            class:online={ping?.state === "online"}
            class:offline={ping?.state === "offline"}
            class:loading={!ping || ping.state === "loading"}
          >
            {#if !ping || ping.state === "loading"}
              Checking…
            {:else if ping.state === "online"}
              ● {ping.status?.online?.toLocaleString()} online
            {:else}
              ● Offline
            {/if}
          </span>
        </div>

        <p class="desc">{server.description}</p>

        <div class="tags">
          {#each server.tags as tag}<span class="tag">{tag}</span>{/each}
        </div>

        <div class="addr-row">
          <code class="addr">{server.address}</code>
          <div class="actions">
            <button
              class="btn sm"
              onclick={() => copyText(server.address, `Copied ${server.address}`)}
              title="Copy address"
            >
              <Icon name="copy" size={13} /> Copy
            </button>
            {#if server.website}
              <button
                class="btn ghost sm"
                onclick={() => openUrl(server.website!)}
                title="Open website"
              >
                <Icon name="globe" size={13} />
              </button>
            {/if}
          </div>
        </div>
      </div>
    {/each}
  </div>

  <p class="disclaimer">
    NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG
    OR MICROSOFT. Server names are trademarks of their respective owners and are
    listed here only to identify each service.
  </p>
</div>

<style>
  .page {
    max-width: 900px;
    margin: 0 auto;
    padding: 2rem 1.5rem 3rem;
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }
  h1 {
    margin: 0;
    font-size: 1.5rem;
  }
  .sub {
    margin: 0.25rem 0 0;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.9rem;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    padding: 0.9rem 1rem;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius, 10px);
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  h2 {
    margin: 0;
    font-size: 1.05rem;
  }
  .status {
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .status.online {
    color: var(--success, #3fb950);
  }
  .status.offline {
    color: var(--text-muted);
  }
  .status.loading {
    color: var(--text-muted);
  }
  .desc {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-secondary, var(--text-muted));
    line-height: 1.45;
    flex: 1;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .tag {
    font-size: 0.7rem;
    padding: 0.1rem 0.45rem;
    border-radius: 999px;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--text) 8%, transparent);
  }
  .addr-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-top: 0.15rem;
  }
  .addr {
    font-family: var(--font-mono, monospace);
    font-size: 0.8rem;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
  }
  .disclaimer {
    margin: 2rem 0 0;
    font-size: 0.68rem;
    line-height: 1.5;
    color: var(--text-muted);
    text-align: center;
  }
</style>
