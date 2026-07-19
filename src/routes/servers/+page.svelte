<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { goto } from "$app/navigation";
  import { api } from "$lib/api";
  import { copyText } from "$lib/clipboard";
  import { serversStore } from "$lib/stores/servers.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import type { ServerStatus } from "$lib/types";
  import Icon from "$lib/components/Icon.svelte";

  const clientInstances = $derived(
    instancesStore.instances.filter((i) => i.kind === "client"),
  );
  // Address of the card whose instance-chooser is open (2+ compatible case).
  let chooserFor = $state<string | null>(null);

  const VER = /\d+\.\d+(?:\.\d+)?/g;

  function cmpVer(a: string, b: string): number {
    const pa = a.split(".").map(Number);
    const pb = b.split(".").map(Number);
    for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
      const d = (pa[i] ?? 0) - (pb[i] ?? 0);
      if (d) return d;
    }
    return 0;
  }

  // Best-effort compatibility: does the server's reported version share this
  // instance's exact version or its major.minor? Unknown version → assume ok.
  function isCompatible(serverVersion: string | undefined, mcVersion: string): boolean {
    if (!serverVersion) return true;
    const mm = mcVersion.match(/\d+\.\d+/)?.[0] ?? mcVersion;
    return serverVersion.includes(mcVersion) || serverVersion.includes(mm);
  }

  // The newest version the server advertises — what a new instance should target.
  function targetVersion(serverVersion: string | undefined): string {
    const tokens = serverVersion?.match(VER) ?? [];
    return [...tokens].sort(cmpVer).at(-1) ?? "";
  }

  function compatibleFor(address: string) {
    const sv = pings[address]?.status?.version;
    return clientInstances.filter((i) => isCompatible(sv, i.mcVersion));
  }

  function play(address: string, instanceId: string) {
    chooserFor = null;
    const server = serversStore.servers.find((s) => s.address === address);
    const modded = (pings[address]?.status?.mods?.length ?? 0) > 0;
    if (server?.requires) {
      toast.info(`${server.name} needs ${server.requires} — make sure this instance has it.`);
    } else if (modded) {
      toast.info("This is a modded server — make sure your instance has its mods.");
    } else {
      toast.success(`Launching into ${address}…`);
    }
    launchStore.launch(instanceId, address);
    goto(`/instance/${instanceId}`);
  }

  function onPlayClick(address: string) {
    const compatible = compatibleFor(address);
    if (compatible.length === 1) {
      play(address, compatible[0].id);
    } else if (compatible.length > 1) {
      chooserFor = chooserFor === address ? null : address;
    } else {
      // No compatible instance — offer to create one (pre-filled), then join.
      const target = targetVersion(pings[address]?.status?.version);
      ui.openCreateInstance({ mcVersion: target || undefined, joinServer: address });
      toast.info(
        target
          ? `No ${target}-compatible instance — create one to join.`
          : "No compatible instance — create one to join.",
      );
    }
  }

  type Ping = { state: "loading" | "online" | "offline"; status?: ServerStatus };

  let pings = $state<Record<string, Ping>>({});
  let showAdd = $state(false);
  let newName = $state("");
  let newAddress = $state("");
  let newRequires = $state("");

  function pingOne(address: string) {
    pings[address] = { state: "loading" };
    api
      .pingServer(address)
      .then((status) => (pings[address] = { state: "online", status }))
      .catch(() => (pings[address] = { state: "offline" }));
  }

  function refresh() {
    for (const server of serversStore.servers) pingOne(server.address);
  }

  // Ping the current list once on load.
  $effect(() => {
    refresh();
  });

  function addServer(event: SubmitEvent) {
    event.preventDefault();
    const address = newAddress.trim();
    if (!address) return;
    const added = serversStore.add({
      name: newName.trim() || address,
      address,
      description: "",
      tags: [],
      requires: newRequires.trim() || undefined,
    });
    if (!added) {
      toast.error("That server is already in your list.");
      return;
    }
    pingOne(address);
    newName = "";
    newAddress = "";
    newRequires = "";
    showAdd = false;
  }

  function removeServer(address: string) {
    serversStore.remove(address);
    delete pings[address];
  }
</script>

<div class="page">
  <header class="head">
    <div>
      <h1>Servers</h1>
      <p class="sub">Your quick-connect list — copy an address and add it in Minecraft's Multiplayer menu.</p>
    </div>
    <div class="head-actions">
      <button class="btn ghost sm" onclick={() => serversStore.reset()} title="Restore the default list">Reset</button>
      <button class="btn ghost sm" onclick={refresh} title="Refresh status">
        <Icon name="refresh" size={15} /> Refresh
      </button>
      <button class="btn primary sm" onclick={() => (showAdd = !showAdd)}>
        <Icon name="plus" size={15} /> Add server
      </button>
    </div>
  </header>

  {#if showAdd}
    <form class="add-form" onsubmit={addServer}>
      <input class="in" placeholder="Name (optional)" bind:value={newName} maxlength="40" />
      <input class="in" placeholder="Address, e.g. play.example.net" bind:value={newAddress} maxlength="120" />
      <input class="in" placeholder="Requires mod/modpack (optional)" bind:value={newRequires} maxlength="60" />
      <button class="btn primary sm" type="submit">Add</button>
      <button class="btn ghost sm" type="button" onclick={() => (showAdd = false)}>Cancel</button>
    </form>
  {/if}

  {#if serversStore.servers.length === 0}
    <p class="empty">No servers yet. Add one above, or <button class="link" onclick={() => serversStore.reset()}>restore the defaults</button>.</p>
  {/if}

  <div class="grid">
    {#each serversStore.servers as server (server.address)}
      {@const ping = pings[server.address]}
      <div class="card">
        <button class="remove" title="Remove" aria-label="Remove {server.name}" onclick={() => removeServer(server.address)}>✕</button>

        <div class="card-head">
          <div class="icon">
            {#if !ping || ping.state === "loading"}
              <span class="skeleton" style="width:100%;height:100%;border-radius:6px"></span>
            {:else if ping.status?.favicon}
              <img src={ping.status.favicon} alt="" />
            {:else}
              <Icon name="globe" size={18} />
            {/if}
          </div>
          <h2>{server.name}</h2>
          {#if !ping || ping.state === "loading"}
            <span class="skeleton" style="width:66px;height:12px"></span>
          {:else}
            <span class="status" class:online={ping.state === "online"} class:offline={ping.state === "offline"}>
              {#if ping.state === "online"}
                ● {ping.status?.online?.toLocaleString()} online
              {:else}
                ● Offline
              {/if}
            </span>
          {/if}
        </div>

        {#if server.description}
          <p class="desc">{server.description}</p>
        {/if}

        {#if server.tags?.length}
          <div class="tags">
            {#each server.tags as tag}<span class="tag">{tag}</span>{/each}
          </div>
        {/if}

        {#if server.requires}
          <span class="req"><Icon name="package" size={12} /> Requires {server.requires}</span>
        {:else if ping?.status?.mods?.length}
          <span class="req"><Icon name="package" size={12} /> Modded · {ping.status.mods.length} mods</span>
        {/if}

        <div class="addr-row">
          <code class="addr">{server.address}</code>
          <div class="actions">
            <button class="btn primary sm" onclick={() => onPlayClick(server.address)} title="Launch Minecraft and join">
              <Icon name="play" size={13} /> Play
            </button>
            <button class="btn sm" onclick={() => copyText(server.address, `Copied ${server.address}`)} title="Copy address">
              <Icon name="copy" size={13} />
            </button>
            {#if server.website}
              <button class="btn ghost sm" onclick={() => openUrl(server.website!)} title="Open website">
                <Icon name="globe" size={13} />
              </button>
            {/if}
          </div>
        </div>

        {#if chooserFor === server.address}
          <div class="chooser">
            <span class="chooser-label">
              Choose an instance{#if ping?.status?.version} <span class="chooser-srv">server runs {ping.status.version}</span>{/if}
            </span>
            {#each compatibleFor(server.address) as instance (instance.id)}
              <button class="chooser-item" onclick={() => play(server.address, instance.id)}>
                <span>{instance.name}</span>
                <span class="chooser-ver">{instance.mcVersion}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <p class="disclaimer">
    NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG
    OR MICROSOFT. Server names and icons are the property of their respective owners
    and are shown only to identify each service.
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
    margin-bottom: 1rem;
  }
  .head-actions {
    display: flex;
    gap: 0.4rem;
    flex-shrink: 0;
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
  .add-form {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }
  .in {
    flex: 1;
    min-width: 160px;
    padding: 0.5rem 0.7rem;
    background: var(--bg-input, var(--bg-card));
    border: 1px solid var(--border);
    border-radius: var(--radius, 8px);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
  }
  .in:focus {
    outline: none;
    border-color: var(--accent);
  }
  .empty {
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    padding: 0;
    text-decoration: underline;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.9rem;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    padding: 0.9rem 1rem;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius, 10px);
  }
  .remove {
    position: absolute;
    top: 0.4rem;
    right: 0.4rem;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.8rem;
    opacity: 0;
    transition: opacity 0.12s, color 0.12s, background 0.12s;
  }
  .card:hover .remove {
    opacity: 1;
  }
  .remove:hover {
    color: var(--danger, #e5484d);
    background: color-mix(in srgb, var(--danger, #e5484d) 15%, transparent);
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding-right: 1.2rem;
  }
  .icon {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    overflow: hidden;
    background: color-mix(in srgb, var(--text) 8%, transparent);
    color: var(--text-muted);
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    image-rendering: pixelated;
  }
  h2 {
    margin: 0;
    font-size: 1.02rem;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .status.online {
    color: var(--success, #3fb950);
  }
  .status.offline {
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
  .req {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.72rem;
    color: var(--accent);
  }
  .addr-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-top: auto;
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
  .chooser {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding-top: 0.4rem;
    border-top: 1px solid var(--border);
  }
  .chooser-label {
    font-size: 0.7rem;
    color: var(--text-muted);
    padding: 0.1rem 0.2rem;
  }
  .chooser-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    background: none;
    border: none;
    border-radius: 6px;
    color: var(--text);
    font: inherit;
    font-size: 0.82rem;
    text-align: left;
    cursor: pointer;
  }
  .chooser-item:hover {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .chooser-ver {
    font-size: 0.72rem;
    color: var(--text-muted);
    font-family: var(--font-mono, monospace);
  }
  .chooser-srv {
    color: var(--text-muted);
    font-weight: 400;
  }
  .disclaimer {
    margin: 2rem 0 0;
    font-size: 0.68rem;
    line-height: 1.5;
    color: var(--text-muted);
    text-align: center;
  }
</style>
