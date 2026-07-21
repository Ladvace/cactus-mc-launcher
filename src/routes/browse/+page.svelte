<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import ProjectModal from "$lib/components/ProjectModal.svelte";
  import Select from "$lib/components/Select.svelte";
  import { api } from "$lib/api";
  import { formatCount } from "$lib/format";
  import { t, type MessageKey } from "$lib/i18n";
  import { SOURCES, type ProjectType, type SearchHit, type Source } from "$lib/types";

  const tabs: { labelKey: MessageKey; type: ProjectType }[] = [
    { labelKey: "browse.tabModpacks", type: "modpack" },
    { labelKey: "browse.tabMods", type: "mod" },
    { labelKey: "browse.tabResourcePacks", type: "resourcepack" },
    { labelKey: "browse.tabShaders", type: "shader" },
    { labelKey: "browse.tabDatapacks", type: "datapack" },
  ];

  const sorts: { value: string; labelKey: MessageKey }[] = [
    { value: "relevance", labelKey: "browse.sortRelevance" },
    { value: "downloads", labelKey: "browse.sortDownloads" },
    { value: "follows", labelKey: "browse.sortFollows" },
    { value: "newest", labelKey: "browse.sortNewest" },
    { value: "updated", labelKey: "browse.sortUpdated" },
  ];

  const loaders = ["", "fabric", "quilt", "forge", "neoforge"];

  let source = $state<Source>("modrinth");
  let activeType = $state<ProjectType>("mod");
  let query = $state("");
  let debounced = $state("");
  let gameVersion = $state("");
  let loader = $state("");
  let sort = $state("relevance");

  let sourceEnabled = $state<Record<string, boolean>>({
    modrinth: true,
    curseforge: false,
  });
  let gameVersions = $state<string[]>([]);
  let hits = $state<SearchHit[]>([]);
  let totalHits = $state(0);
  let offset = $state(0);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);

  let selectedHit = $state<SearchHit | null>(null);
  let modalOpen = $state(false);

  const LIMIT = 20;
  const showLoader = $derived(activeType === "mod" || activeType === "modpack");
  const sourceLabel = $derived(
    SOURCES.find((option) => option.value === source)?.label ?? t("browse.contentSourceFallback")
  );

  const sourceOptions = $derived(
    SOURCES.map((option) => ({
      value: option.value,
      label: `${option.label}${sourceEnabled[option.value] ? "" : t("browse.unavailableSuffix")}`,
      disabled: !sourceEnabled[option.value],
    }))
  );
  const gameVersionOptions = $derived([
    { value: "", label: t("browse.anyVersion") },
    ...gameVersions.map((version) => ({ value: version, label: version })),
  ]);
  const loaderOptions = $derived(
    loaders.map((loaderOption) => ({
      value: loaderOption,
      label: loaderOption === "" ? t("browse.anyLoader") : loaderOption,
    }))
  );
  const sortOptions = $derived(
    sorts.map((sortOption) => ({ value: sortOption.value, label: t(sortOption.labelKey) }))
  );

  $effect(() => {
    const currentQuery = query;
    const timer = setTimeout(() => (debounced = currentQuery), 350);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    api
      .listSources()
      .then((list) => {
        const map: Record<string, boolean> = {};
        for (const sourceInfo of list) map[sourceInfo.id] = sourceInfo.enabled;
        sourceEnabled = map;
      })
      .catch(() => {});
  });

  $effect(() => {
    if (gameVersions.length === 0) {
      api
        .getMinecraftVersions()
        .then((list) => {
          gameVersions = list.versions
            .filter((version) => version.type === "release")
            .map((version) => version.id);
        })
        .catch(() => {});
    }
  });

  $effect(() => {
    // Track dependencies:
    void [source, activeType, debounced, gameVersion, loader, sort];
    search();
  });

  function searchParams(from: number) {
    return {
      query: debounced,
      projectType: activeType,
      gameVersion: gameVersion || null,
      loader: showLoader ? loader || null : null,
      sort,
      offset: from,
      limit: LIMIT,
    };
  }

  async function search() {
    loading = true;
    error = null;
    offset = 0;
    try {
      const res = await api.searchContent(source, searchParams(0));
      hits = res.hits;
      totalHits = res.totalHits;
    } catch (err) {
      error = String(err);
      hits = [];
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    loadingMore = true;
    try {
      const next = offset + LIMIT;
      const res = await api.searchContent(source, searchParams(next));
      hits = [...hits, ...res.hits];
      offset = next;
    } catch (err) {
      error = String(err);
    } finally {
      loadingMore = false;
    }
  }

  function openProject(hit: SearchHit) {
    selectedHit = hit;
    modalOpen = true;
  }
</script>

<div class="page">
  <header class="head">
    <h1>{t("nav.browse")}</h1>
  </header>

  <div class="tabs">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={activeType === tab.type}
        onclick={() => (activeType = tab.type)}
      >
        {t(tab.labelKey)}
      </button>
    {/each}
  </div>

  <div class="toolbar">
    <Select
      bind:value={source}
      options={sourceOptions}
      ariaLabel={t("browse.contentSource")}
      width="160px"
    />
    <div class="search">
      <Icon name="search" size={16} />
      <input class="search-input" placeholder={t("browse.searchPlaceholder")} bind:value={query} />
    </div>
    <Select bind:value={gameVersion} options={gameVersionOptions} width="160px" />
    {#if showLoader}
      <Select bind:value={loader} options={loaderOptions} width="160px" />
    {/if}
    <Select bind:value={sort} options={sortOptions} width="160px" />
  </div>

  {#if error}
    <div class="status error">
      <p>{t("browse.cantReach", { source: sourceLabel })}</p>
      <p class="err-detail">{error}</p>
      <button class="btn ghost" onclick={search}>{t("common.retry")}</button>
    </div>
  {:else if loading}
    <div class="results">
      {#each Array(8) as _, index (index)}
        <div class="card skel">
          <span class="skeleton" style="width:56px;height:56px"></span>
          <div class="card-body">
            <span class="skeleton" style="width:55%;height:14px"></span>
            <span class="skeleton" style="width:100%;height:11px"></span>
            <span class="skeleton" style="width:75%;height:11px"></span>
            <span class="skeleton skel-stat" style="width:44px;height:11px"></span>
          </div>
        </div>
      {/each}
    </div>
  {:else if hits.length === 0}
    <div class="status">{t("browse.noResults")}</div>
  {:else}
    <div class="results">
      {#each hits as hit (hit.projectId)}
        <button class="card" onclick={() => openProject(hit)}>
          {#if hit.iconUrl}
            <img class="card-icon" src={hit.iconUrl} alt={hit.title} />
          {:else}
            <div class="card-icon placeholder"><Icon name="package" size={24} /></div>
          {/if}
          <div class="card-body">
            <div class="card-top">
              <span class="card-title">{hit.title}</span>
              <span class="card-author">{t("browse.byAuthor", { author: hit.author })}</span>
            </div>
            <p class="card-desc">{hit.description}</p>
            <div class="card-stats">
              <span><Icon name="package" size={12} /> {formatCount(hit.downloads)}</span>
            </div>
          </div>
        </button>
      {/each}
    </div>

    {#if hits.length < totalHits}
      <div class="more">
        <button class="btn ghost" onclick={loadMore} disabled={loadingMore}>
          {loadingMore ? t("common.loading") : t("browse.loadMore")}
        </button>
      </div>
    {/if}
  {/if}
</div>

<ProjectModal hit={selectedHit} open={modalOpen} onClose={() => (modalOpen = false)} />

<style>
  .page {
    padding: 28px 32px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .head h1 {
    font-size: 24px;
    margin-bottom: 18px;
  }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 20px;
  }
  .tab {
    padding: 10px 14px;
    background: transparent;
    border: none;
    border-bottom: 3px solid transparent;
    color: var(--text-secondary);
    font-family: var(--font-pixel);
    font-size: 14px;
    font-weight: 500;
    margin-bottom: -1px;
    transition: color 0.12s, border-color 0.12s;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .toolbar {
    display: flex;
    gap: 10px;
    margin-bottom: 22px;
  }
  .search {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  .search :global(.hn) {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 9px 12px 9px 36px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .status {
    padding: 48px;
    text-align: center;
    color: var(--text-muted);
  }
  .status.error {
    color: var(--danger);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }
  .err-detail {
    font-size: 12px;
    color: var(--text-muted);
    max-width: 480px;
    word-break: break-word;
  }
  .results {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 14px;
  }
  .card {
    display: flex;
    gap: 14px;
    padding: 14px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: 0;
    text-align: left;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    transition: border-color 0.12s, transform 0.12s, background 0.12s;
  }
  .card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    transform: translateY(-2px);
  }
  .card.skel {
    pointer-events: none;
  }
  .skel-stat {
    margin-top: auto;
  }
  .card-icon {
    width: 56px;
    height: 56px;
    border-radius: 0;
    border: 2px solid rgba(0, 0, 0, 0.3);
    object-fit: cover;
    background: var(--bg-input);
    flex-shrink: 0;
  }
  .card-icon.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .card-body {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .card-top {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .card-title {
    font-weight: 600;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-author {
    font-size: 11.5px;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .card-desc {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-secondary);
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .card-stats {
    display: flex;
    gap: 12px;
    font-size: 11.5px;
    color: var(--text-muted);
    margin-top: auto;
  }
  .card-stats span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .more {
    display: flex;
    justify-content: center;
    margin: 24px 0;
  }
</style>
