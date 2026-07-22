<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { t } from "$lib/i18n";
  import { api } from "$lib/api";
  import { copyText } from "$lib/clipboard";
  import { formatCount } from "$lib/format";
  import { serversStore } from "$lib/stores/servers.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import type { ServerStatus, BrowseServer } from "$lib/types";
  import type { FeaturedServer } from "$lib/servers";
  import Icon from "$lib/components/Icon.svelte";
  import Select from "$lib/components/Select.svelte";
  import ContextMenu, { type MenuItem } from "$lib/components/ContextMenu.svelte";

  type Ping = { state: "loading" | "online" | "offline"; status?: ServerStatus };

  let pings = $state<Record<string, Ping>>({});
  let showAdd = $state(false);
  let newName = $state("");
  let newAddress = $state("");

  // Discover: browse a public directory (minecraft-list.info) to find servers.
  let view = $state<"mine" | "discover">("mine");
  let dQuery = $state("");
  let dDebounced = $state("");
  let dSort = $state("players");
  let dHits = $state<BrowseServer[]>([]);
  let dPage = $state(1);
  let dHasMore = $state(false);
  let dLoading = $state(false);
  let dLoadingMore = $state(false);
  let dError = $state<string | null>(null);

  const dSortOptions = $derived([
    { value: "players", label: t("servers.sortPlayers") },
    { value: "votes", label: t("servers.sortVotes") },
    { value: "rating", label: t("servers.sortRating") },
  ]);

  // Strip Minecraft §-formatting codes from directory-provided text.
  const clean = (value: string) => value.replace(/§./g, "").trim();

  $effect(() => {
    const value = dQuery;
    const timer = setTimeout(() => (dDebounced = value), 350);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (view !== "discover") return;
    void [dDebounced, dSort];
    discoverSearch();
  });

  async function discoverSearch() {
    dLoading = true;
    dError = null;
    dPage = 1;
    try {
      const res = await api.browseServers({ query: dDebounced, sort: dSort, page: 1 });
      dHits = res.servers;
      dHasMore = res.hasMore;
    } catch (err) {
      dError = String(err);
      dHits = [];
    } finally {
      dLoading = false;
    }
  }

  async function discoverMore() {
    dLoadingMore = true;
    try {
      const next = dPage + 1;
      const res = await api.browseServers({ query: dDebounced, sort: dSort, page: next });
      dHits = [...dHits, ...res.servers];
      dPage = next;
      dHasMore = res.hasMore;
    } catch (err) {
      toast.error(String(err));
    } finally {
      dLoadingMore = false;
    }
  }

  function addFromDiscover(server: BrowseServer) {
    const name = clean(server.name) || server.address;
    const added = serversStore.add({
      name,
      address: server.address,
      description: clean(server.description),
      tags: server.country ? [server.country] : [],
    });
    if (added) {
      toast.success(t("servers.addedToMine", { name }));
      pingOne(server.address);
    } else {
      toast.error(t("servers.alreadyInList"));
    }
  }

  let addMenu = $state<{ x: number; y: number; server: FeaturedServer } | null>(null);
  const clientInstances = $derived(
    instancesStore.instances.filter((i) => i.kind === "client"),
  );

  const addMenuItems = $derived<MenuItem[]>(
    addMenu
      ? clientInstances.map((instance) => ({
          label: `${instance.name} · ${instance.mcVersion}`,
          icon: "plus",
          onSelect: () => addToInstance(instance.id, addMenu!.server),
        }))
      : [],
  );

  async function addToInstance(instanceId: string, server: FeaturedServer) {
    addMenu = null;
    try {
      await api.addServerToInstance(instanceId, server.name, server.address);
      const inst = clientInstances.find((i) => i.id === instanceId);
      toast.success(
        t("servers.addedToInstance", {
          server: server.name,
          instance: inst?.name ?? t("servers.theInstance"),
        }),
      );
    } catch (err) {
      toast.error(String(err));
    }
  }

  function onAddToClick(event: MouseEvent, server: FeaturedServer) {
    if (clientInstances.length === 0) {
      toast.error(t("servers.createInstanceFirst"));
      return;
    }
    if (addMenu?.server.address === server.address) {
      addMenu = null;
      return;
    }
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    addMenu = { x: rect.left, y: rect.bottom + 4, server };
  }

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

  $effect(() => {
    refresh();
  });

  function addServer(event: SubmitEvent) {
    event.preventDefault();
    const address = newAddress.trim();
    if (!address) return;
    if (!serversStore.add({ name: newName.trim() || address, address, description: "", tags: [] })) {
      toast.error(t("servers.alreadyInList"));
      return;
    }
    pingOne(address);
    newName = "";
    newAddress = "";
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
      <h1>{t("nav.servers")}</h1>
      <p class="sub">{t("servers.subtitle")}</p>
    </div>
    {#if view === "mine"}
      <div class="head-actions">
        <button class="btn ghost sm" onclick={() => serversStore.reset()} title={t("servers.restoreDefaultList")}>{t("servers.reset")}</button>
        <button class="btn ghost sm" onclick={refresh} title={t("servers.refreshStatus")}>
          <Icon name="refresh" size={15} /> {t("servers.refresh")}
        </button>
        <button class="btn primary sm" onclick={() => (showAdd = !showAdd)}>
          <Icon name="plus" size={15} /> {t("servers.addServer")}
        </button>
      </div>
    {/if}
  </header>

  <div class="seg">
    <button class="seg-btn" class:on={view === "mine"} onclick={() => (view = "mine")}>
      {t("servers.myServers")}
    </button>
    <button class="seg-btn" class:on={view === "discover"} onclick={() => (view = "discover")}>
      <Icon name="compass" size={14} /> {t("servers.discover")}
    </button>
  </div>

  {#if view === "mine"}
  {#if showAdd}
    <form class="add-form" onsubmit={addServer}>
      <input class="in" placeholder={t("servers.namePlaceholder")} bind:value={newName} maxlength="40" />
      <input class="in" placeholder={t("servers.addressPlaceholder")} bind:value={newAddress} maxlength="120" />
      <button class="btn primary sm" type="submit">{t("common.add")}</button>
      <button class="btn ghost sm" type="button" onclick={() => (showAdd = false)}>{t("common.cancel")}</button>
    </form>
  {/if}

  {#if serversStore.servers.length === 0}
    <p class="empty">{t("servers.emptyPrefix")}<button class="link" onclick={() => serversStore.reset()}>{t("servers.restoreDefaults")}</button>.</p>
  {/if}

  <div class="grid">
    {#each serversStore.servers as server (server.address)}
      {@const ping = pings[server.address]}
      <div class="card">
        <button class="remove" title={t("common.remove")} aria-label={t("servers.removeServerAria", { name: server.name })} onclick={() => removeServer(server.address)}>✕</button>

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
                ● {t("servers.countOnline", { count: ping.status?.online?.toLocaleString() ?? "0" })}
              {:else}
                ● {t("servers.offline")}
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

        <div class="addr-row">
          <code class="addr">{server.address}</code>
          <div class="actions">
            <button class="btn sm" onclick={() => copyText(server.address, t("servers.copiedAddress", { address: server.address }))} title={t("servers.copyAddress")}>
              <Icon name="copy" size={13} /> {t("servers.copy")}
            </button>
            <button class="btn ghost sm" onclick={(e) => onAddToClick(e, server)} title={t("servers.addToInstanceList")}>
              <Icon name="plus" size={13} />
            </button>
            {#if server.website}
              <button class="btn ghost sm" onclick={() => openUrl(server.website!)} title={t("servers.openWebsite")}>
                <Icon name="globe" size={13} />
              </button>
            {/if}
          </div>
        </div>

      </div>
    {/each}
  </div>
  {:else}
    <div class="discover-toolbar">
      <div class="search">
        <Icon name="search" size={16} />
        <input class="search-input" placeholder={t("servers.discoverSearch")} bind:value={dQuery} />
      </div>
      <Select bind:value={dSort} options={dSortOptions} width="180px" />
    </div>

    {#if dError}
      <div class="d-status error">
        <p>{t("servers.discoverError")}</p>
        <button class="btn ghost sm" onclick={discoverSearch}>{t("common.retry")}</button>
      </div>
    {:else if dLoading}
      <div class="grid">
        {#each Array(8) as _, index (index)}
          <div class="card"><span class="skeleton" style="width:100%;height:96px"></span></div>
        {/each}
      </div>
    {:else if dHits.length === 0}
      <p class="d-status">{t("servers.discoverEmpty")}</p>
    {:else}
      <div class="grid">
        {#each dHits as srv (srv.address)}
          <div class="card">
            <div class="card-head">
              <div class="icon">
                {#if srv.favicon}
                  <img src={srv.favicon} alt="" />
                {:else}
                  <Icon name="globe" size={18} />
                {/if}
              </div>
              <h2>{clean(srv.name) || srv.address}</h2>
              <span class="status" class:online={srv.online} class:offline={!srv.online}>
                {#if srv.online}
                  ● {t("servers.countOnline", { count: srv.players.toLocaleString() })}
                {:else}
                  ● {t("servers.offline")}
                {/if}
              </span>
            </div>

            {#if clean(srv.description)}
              <p class="desc">{clean(srv.description)}</p>
            {/if}

            <div class="tags">
              {#if srv.version}<span class="tag ver">{clean(srv.version)}</span>{/if}
              {#if srv.country}<span class="tag">{srv.country}</span>{/if}
              {#if srv.votes > 0}
                <span class="tag"><Icon name="heart" size={10} /> {formatCount(srv.votes)}</span>
              {/if}
            </div>

            <div class="addr-row">
              <code class="addr">{srv.address}</code>
              <div class="actions">
                <button class="btn sm" onclick={() => copyText(srv.address, t("servers.copiedAddress", { address: srv.address }))} title={t("servers.copyAddress")}>
                  <Icon name="copy" size={13} />
                </button>
                <button class="btn ghost sm" onclick={(e) => onAddToClick(e, { name: clean(srv.name) || srv.address, address: srv.address, description: "", tags: [] })} title={t("servers.addToInstanceList")}>
                  <Icon name="cube" size={13} />
                </button>
                <button class="btn primary sm" onclick={() => addFromDiscover(srv)} title={t("servers.addToMine")}>
                  <Icon name="plus" size={13} /> {t("common.add")}
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>

      {#if dHasMore}
        <div class="more">
          <button class="btn ghost" onclick={discoverMore} disabled={dLoadingMore}>
            {dLoadingMore ? t("common.loading") : t("servers.loadMore")}
          </button>
        </div>
      {/if}
    {/if}

    <p class="powered">
      {t("servers.poweredBy")}
      <button class="link" onclick={() => openUrl("https://minecraft-list.info")}>minecraft-list.info</button>
    </p>
  {/if}

  {#if addMenu}
    <ContextMenu x={addMenu.x} y={addMenu.y} items={addMenuItems} onClose={() => (addMenu = null)} />
  {/if}

  <p class="disclaimer">
    {t("servers.disclaimer")}
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
  .seg {
    display: inline-flex;
    gap: 2px;
    margin-bottom: 1.1rem;
    padding: 3px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .seg-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 16px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, background 0.12s;
  }
  .seg-btn:hover {
    color: var(--text);
  }
  .seg-btn.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .discover-toolbar {
    display: flex;
    gap: 0.6rem;
    margin-bottom: 1rem;
  }
  .discover-toolbar .search {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  .discover-toolbar .search :global(.hn) {
    position: absolute;
    left: 12px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 9px 12px 9px 36px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .d-status {
    padding: 3rem 1rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .d-status.error {
    color: var(--danger);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
  }
  .tag.ver {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag :global(.hn) {
    vertical-align: -1px;
  }
  .more {
    display: flex;
    justify-content: center;
    margin: 1.5rem 0 0;
  }
  .powered {
    margin: 1.5rem 0 0;
    text-align: center;
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .powered .link {
    color: var(--text-secondary);
  }
  .powered .link:hover {
    color: var(--accent);
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
    grid-auto-rows: 1fr;
    gap: 0.9rem;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    height: 100%;
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
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
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
  .disclaimer {
    margin: 2rem 0 0;
    font-size: 0.68rem;
    line-height: 1.5;
    color: var(--text-muted);
    text-align: center;
  }
</style>
