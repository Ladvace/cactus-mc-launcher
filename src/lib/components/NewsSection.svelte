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
  // Remaining stories grouped into columns of two (stacked) for the scroller.
  const columns = $derived.by(() => {
    const rest = items.slice(1);
    const cols: NewsItem[][] = [];
    for (let i = 0; i < rest.length; i += 2) cols.push(rest.slice(i, i + 2));
    return cols;
  });

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

    <div class="strip">
      {#if loading && items.length === 0}
        <div class="feature skeleton"><div class="sk-img"></div></div>
        {#each Array(2) as _, c (c)}
          <div class="col">
            {#each Array(2) as _, i (i)}
              <div class="mini skeleton">
                <div class="sk-thumb"></div>
                <div class="sk-lines"><span class="sk-line"></span><span class="sk-line short"></span></div>
              </div>
            {/each}
          </div>
        {/each}
      {:else if featured}
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

        <!-- Two-up columns, scrolling horizontally -->
        {#each columns as col, c (c)}
          <div class="col">
            {#each col as item (item.id)}
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
        {/each}
      {/if}
    </div>
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

  /* Horizontal scroller: lead card, then columns of two. */
  .strip {
    --news-h: 288px;
    display: flex;
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 10px;
    scroll-snap-type: x proximity;
  }

  /* Lead story — large image with a gradient scrim + overlaid text. */
  .feature {
    flex: 0 0 clamp(300px, 42%, 440px);
    height: var(--news-h);
    scroll-snap-align: start;
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
    flex: 0 0 clamp(260px, 32%, 330px);
    height: var(--news-h);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .mini {
    flex: 1;
    min-height: 0;
    scroll-snap-align: start;
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
