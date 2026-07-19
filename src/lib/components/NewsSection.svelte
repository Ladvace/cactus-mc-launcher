<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import Icon from "./Icon.svelte";
  import type { NewsItem } from "$lib/types";

  let items = $state<NewsItem[]>([]);
  let loading = $state(true);
  let failed = $state(false);

  const featured = $derived(items[0] ?? null);
  const rest = $derived(items.slice(1, 6));

  async function load(force = false) {
    loading = true;
    failed = false;
    try {
      items = await api.getNews(force);
    } catch {
      failed = true;
    } finally {
      loading = false;
    }
  }

  onMount(() => load());

  function open(item: NewsItem) {
    if (item.link) openUrl(item.link).catch(() => {});
  }

  function fmtDate(iso: string): string {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return iso;
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  }
</script>

<!-- News is non-essential: if it fails to load, quietly show nothing. -->
{#if !failed && (loading || items.length > 0)}
  <section class="news">
    <div class="news-head">
      <h2>Latest news</h2>
      <button class="refresh" title="Refresh news" onclick={() => load(true)} disabled={loading}>
        <Icon name="refresh" size={13} />
      </button>
    </div>

    {#if loading && items.length === 0}
      <div class="layout">
        <div class="feature skeleton"><div class="sk-img"></div></div>
        <div class="list">
          {#each Array(4) as _, i (i)}
            <div class="row skeleton">
              <div class="sk-thumb"></div>
              <div class="sk-lines"><span class="sk-line"></span><span class="sk-line short"></span></div>
            </div>
          {/each}
        </div>
      </div>
    {:else if featured}
      <div class="layout">
        <!-- Lead story -->
        <button
          class="feature"
          class:link={!!featured.link}
          onclick={() => open(featured)}
          style={featured.image ? `background-image:url('${featured.image}')` : ""}
          class:noimg={!featured.image}
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

        <!-- Headline list -->
        <div class="list">
          {#each rest as item (item.id)}
            <button class="row" class:link={!!item.link} onclick={() => open(item)}>
              {#if item.image}
                <div class="thumb" style="background-image:url('{item.image}')"></div>
              {:else}
                <div class="thumb placeholder"><Icon name="globe" size={16} /></div>
              {/if}
              <div class="row-body">
                <span class="row-title">{item.title}</span>
                <span class="row-meta">
                  {#if item.category}<span class="row-cat">{item.category}</span> · {/if}{fmtDate(item.date)}
                </span>
              </div>
            </button>
          {/each}
        </div>
      </div>
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
    gap: 10px;
    margin-bottom: 12px;
  }
  .news-head h2 {
    font-size: 15px;
  }
  .refresh {
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
  .refresh:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .refresh:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .layout {
    display: grid;
    grid-template-columns: 1.5fr 1fr;
    gap: 14px;
    align-items: stretch;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  /* Lead story — large image with a gradient scrim + overlaid text. */
  .feature {
    position: relative;
    min-height: 260px;
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
    background: linear-gradient(to top, rgba(0, 0, 0, 0.88) 12%, rgba(0, 0, 0, 0.55) 55%, transparent);
  }
  .cat {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent);
    font-weight: 600;
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
  .feature-meta .more {
    color: var(--accent);
    margin-left: auto;
  }

  /* Headline list */
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px;
    text-align: left;
    background: var(--bg-card);
    border: 2px solid var(--border);
    color: var(--text);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.03), inset -2px -2px 0 rgba(0, 0, 0, 0.22);
    transition: border-color 0.12s, transform 0.12s;
    flex: 1;
    min-height: 0;
  }
  .row.link {
    cursor: pointer;
  }
  .row.link:hover {
    border-color: var(--accent);
    transform: translateX(2px);
  }
  .thumb {
    flex-shrink: 0;
    width: 62px;
    height: 46px;
    background-size: cover;
    background-position: center;
    background-color: var(--bg-app);
    border: 1px solid var(--border);
  }
  .thumb.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .row-body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .row-title {
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .row-meta {
    font-size: 10.5px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row-cat {
    color: var(--accent);
  }

  /* Skeletons */
  .skeleton {
    pointer-events: none;
  }
  .sk-img {
    width: 100%;
    height: 100%;
  }
  .feature.skeleton {
    background: none;
  }
  .sk-img,
  .sk-thumb {
    background: linear-gradient(90deg, var(--bg-app), var(--bg-card), var(--bg-app));
    background-size: 200% 100%;
    animation: shimmer 1.3s ease-in-out infinite;
  }
  .sk-thumb {
    flex-shrink: 0;
    width: 62px;
    height: 46px;
  }
  .sk-lines {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
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
