<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import Icon from "./Icon.svelte";
  import type { NewsItem } from "$lib/types";

  const PER_PAGE = 3; // 1 lead card + 2 stacked mini cards

  let items = $state<NewsItem[]>([]);
  let loading = $state(true);
  let failed = $state(false);
  let page = $state(0);

  // Split into pages of three; each page renders a lead + a two-up column.
  const pages = $derived.by(() => {
    const out: NewsItem[][] = [];
    for (let i = 0; i < items.length; i += PER_PAGE) out.push(items.slice(i, i + PER_PAGE));
    return out;
  });
  const current = $derived(pages[page] ?? []);
  const featured = $derived(current[0] ?? null);
  const minis = $derived(current.slice(1));

  async function load(force = false) {
    loading = true;
    failed = false;
    try {
      items = await api.getNews(force);
      page = 0;
    } catch {
      failed = true;
    } finally {
      loading = false;
    }
  }

  onMount(() => load());

  function hide() {
    settingsStore.save({ ...settingsStore.settings, showNews: false });
  }

  function go(delta: number) {
    page = Math.min(Math.max(page + delta, 0), pages.length - 1);
  }

  function open(item: NewsItem) {
    if (item.link) openUrl(item.link).catch(() => {});
  }

  function fmtDate(iso: string): string {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return iso;
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  }
</script>

<!-- News is non-essential: hidden by setting, or if it fails to load. -->
{#if settingsStore.settings.showNews && !failed && (loading || items.length > 0)}
  <section class="news">
    <div class="news-head">
      <h2>Latest news</h2>
      <div class="head-actions">
        {#if pages.length > 1}
          <button class="nav" title="Previous" onclick={() => go(-1)} disabled={page === 0}>
            <Icon name="chevron-left" size={14} />
          </button>
          <button
            class="nav"
            title="Next"
            onclick={() => go(1)}
            disabled={page >= pages.length - 1}
          >
            <Icon name="chevron-right" size={14} />
          </button>
        {/if}
        <button class="nav" title="Refresh news" onclick={() => load(true)} disabled={loading}>
          <Icon name="refresh" size={13} />
        </button>
        <button class="nav" title="Hide news (re-enable in Settings)" onclick={hide}>
          <Icon name="close" size={14} />
        </button>
      </div>
    </div>

    {#if loading && items.length === 0}
      <div class="strip">
        <div class="feature skeleton"><div class="sk-img"></div></div>
        <div class="col">
          {#each Array(2) as _, i (i)}
            <div class="mini skeleton">
              <div class="sk-thumb"></div>
              <div class="sk-lines"><span class="sk-line"></span><span class="sk-line short"></span></div>
            </div>
          {/each}
        </div>
      </div>
    {:else if featured}
      {#key page}
        <div class="strip" in:fade={{ duration: 140 }}>
          <!-- Lead story -->
          <button
            class="feature"
            class:link={!!featured.link}
            class:noimg={!featured.image}
            onclick={() => open(featured)}
            style={featured.image ? `background-image:url('${featured.image}')` : ""}
          >
            <div class="feature-scrim">
              {#if featured.category}<span class="cat">{featured.category}</span>{/if}
              <h3 class="feature-title">{featured.title}</h3>
              {#if featured.summary}<p class="feature-sum">{featured.summary}</p>{/if}
              <span class="feature-meta">
                {fmtDate(featured.date)}
                {#if featured.link}<span class="more">Read more →</span>{/if}
              </span>
            </div>
          </button>

          <!-- Two-up column -->
          <div class="col">
            {#each minis as item (item.id)}
              <button class="mini" class:link={!!item.link} onclick={() => open(item)}>
                {#if item.image}
                  <div class="mini-thumb" style="background-image:url('{item.image}')"></div>
                {:else}
                  <div class="mini-thumb placeholder"><Icon name="globe" size={18} /></div>
                {/if}
                <div class="mini-body">
                  {#if item.category}<span class="mini-cat">{item.category}</span>{/if}
                  <span class="mini-title">{item.title}</span>
                  <span class="mini-meta">
                    {fmtDate(item.date)}
                    {#if item.link}<span class="more">Read →</span>{/if}
                  </span>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/key}

      {#if pages.length > 1}
        <div class="dots" role="tablist" aria-label="News pages">
          {#each pages as _, i (i)}
            <button
              class="dot"
              class:on={i === page}
              role="tab"
              aria-selected={i === page}
              aria-label={`Page ${i + 1}`}
              onclick={() => (page = i)}
            ></button>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .news {
    margin-top: 34px;
  }
  .news-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
  }
  .news-head h2 {
    font-size: 15px;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .nav {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }
  .nav:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .nav:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .strip {
    --news-h: 259px;
    display: grid;
    grid-template-columns: 1.5fr 1fr;
    gap: 12px;
  }
  @media (max-width: 760px) {
    .strip {
      grid-template-columns: 1fr;
    }
  }

  /* Lead story — large image with a gradient scrim + overlaid text. */
  .feature {
    height: var(--news-h);
    position: relative;
    padding: 0;
    text-align: left;
    border: 2px solid var(--border);
    background-color: var(--bg-app);
    background-size: cover;
    background-position: center;
    color: #fff;
    overflow: hidden;
    display: flex;
    align-items: flex-end;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.03), inset -2px -2px 0 rgba(0, 0, 0, 0.25);
    transition: border-color 0.12s;
  }
  .feature.noimg {
    background: linear-gradient(135deg, var(--bg-card), var(--bg-app));
  }
  .feature.link {
    cursor: pointer;
  }
  .feature.link:hover {
    border-color: var(--accent);
  }
  .feature-scrim {
    width: 100%;
    padding: 16px 18px 15px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.9) 12%, rgba(0, 0, 0, 0.55) 55%, transparent);
  }
  .feature-title {
    font-size: 19px;
    line-height: 1.2;
    color: #fff;
  }
  .feature-sum {
    font-size: 12.5px;
    line-height: 1.45;
    color: rgba(255, 255, 255, 0.82);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin: 0;
  }
  .feature-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.65);
    margin-top: 2px;
  }

  /* A column of two stacked mini cards. */
  .col {
    height: var(--news-h);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .mini {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 10px;
    padding: 8px;
    text-align: left;
    background: var(--bg-card);
    border: 2px solid var(--border);
    color: var(--text);
    overflow: hidden;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.03), inset -2px -2px 0 rgba(0, 0, 0, 0.22);
    transition: border-color 0.12s, transform 0.12s;
  }
  .mini.link {
    cursor: pointer;
  }
  .mini.link:hover {
    border-color: var(--accent);
    transform: translateY(-2px);
  }
  .mini-thumb {
    flex-shrink: 0;
    width: 96px;
    background-size: cover;
    background-position: center;
    background-color: var(--bg-app);
    border: 1px solid var(--border);
  }
  .mini-thumb.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .mini-body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 2px 2px 2px 0;
  }
  .cat,
  .mini-cat {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
    font-weight: 600;
  }
  .mini-title {
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .mini-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 10.5px;
    color: var(--text-muted);
    margin-top: auto;
  }
  .more {
    color: var(--accent);
    margin-left: auto;
  }

  /* Step indicators */
  .dots {
    display: flex;
    justify-content: center;
    gap: 7px;
    margin-top: 12px;
  }
  .dot {
    width: 9px;
    height: 9px;
    padding: 0;
    background: var(--border);
    border: 1px solid var(--border);
    cursor: pointer;
    transition: background 0.12s, width 0.15s, border-color 0.12s;
  }
  .dot:hover {
    border-color: var(--accent);
  }
  .dot.on {
    width: 22px;
    background: var(--accent);
    border-color: var(--accent);
  }

  /* Skeletons */
  .skeleton {
    pointer-events: none;
  }
  .feature.skeleton {
    background: none;
  }
  .sk-img {
    width: 100%;
    height: 100%;
  }
  .sk-img,
  .sk-thumb {
    background: linear-gradient(90deg, var(--bg-app), var(--bg-card), var(--bg-app));
    background-size: 200% 100%;
    animation: shimmer 1.3s ease-in-out infinite;
  }
  .sk-thumb {
    flex-shrink: 0;
    width: 96px;
  }
  .sk-lines {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    padding: 4px 0;
  }
  .sk-line {
    height: 10px;
    background: var(--border);
    border-radius: 2px;
  }
  .sk-line.short {
    width: 55%;
  }
  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }
</style>
