<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import Icon from "$lib/components/Icon.svelte";
  import type { AchievementsPayload, AdvancementView } from "$lib/types";

  let data = $state<AchievementsPayload | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let tab = $state<"achievements" | "stats">("achievements");
  let categoryFilter = $state<string>("all");
  let showLocked = $state(true);

  const CATEGORY_LABELS: Record<string, string> = {
    story: "Story",
    nether: "Nether",
    end: "The End",
    adventure: "Adventure",
    husbandry: "Husbandry",
    other: "Other",
  };
  const CATEGORY_ICON: Record<string, string> = {
    story: "book",
    nether: "fire",
    end: "moon",
    adventure: "flag",
    husbandry: "heart",
    other: "trophy",
  };

  async function load() {
    loading = true;
    error = null;
    try {
      data = await api.getAchievements();
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  const visibleAdvancements = $derived.by<AdvancementView[]>(() => {
    if (!data) return [];
    return data.advancements.filter((a) => {
      if (categoryFilter !== "all" && a.category !== categoryFilter) return false;
      if (!showLocked && !a.done) return false;
      return true;
    });
  });

  const earnedCustom = $derived(data?.custom.filter((c) => c.earned).length ?? 0);

  function displayName(a: AdvancementView): string {
    return !a.done && a.hidden ? "???" : a.name;
  }
  function displayDesc(a: AdvancementView): string {
    if (a.done) return a.earnedIn ?? "Earned";
    if (a.hidden) return "Hidden — unlock to reveal";
    return "Locked";
  }

  function fmtNumber(n: number): string {
    return n.toLocaleString();
  }
  function fmtHours(ticks: number): string {
    const hours = ticks / 20 / 3600;
    if (hours >= 100) return `${Math.round(hours)} h`;
    return `${hours.toFixed(1)} h`;
  }
  function fmtDistance(cm: number): string {
    const km = cm / 100 / 1000;
    if (km >= 1) return `${km.toFixed(1)} km`;
    return `${Math.round(cm / 100)} m`;
  }
  function fmtDate(iso: string | null): string {
    if (!iso) return "";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
  }
  function cleanKey(key: string): string {
    const last = key.split(":").pop() ?? key;
    return last.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
  }
</script>

<div class="page">
  <header class="head">
    <div>
      <h1>Achievements</h1>
      <p class="sub">
        {#if data?.player.name}
          Lifetime progress for <strong>{data.player.name}</strong> across all instances
        {:else}
          Your lifetime Minecraft progress
        {/if}
      </p>
    </div>
    <button class="btn ghost" onclick={load} disabled={loading} aria-label="Refresh">
      <Icon name="refresh" size={15} /> Refresh
    </button>
  </header>

  {#if loading}
    <div class="state">Scanning your worlds…</div>
  {:else if error}
    <div class="state err">Couldn’t load achievements: {error}</div>
  {:else if data && !data.hasData}
    <div class="state">
      <Icon name="trophy" size={28} />
      <p>No world data found yet.</p>
      <small
        >Play a singleplayer or LAN world in one of your instances, then check back. Progress
        on remote servers lives server-side and can’t be read locally.</small
      >
    </div>
  {:else if data}
    <section class="hero">
      <div class="hero-ring" style="--pct:{data.completion.percent}">
        <span class="hero-pct">{data.completion.percent}%</span>
        <span class="hero-cap">complete</span>
      </div>
      <div class="hero-meta">
        <div class="hero-line">
          <strong>{data.completion.earned}</strong> / {data.completion.total} advancements
        </div>
        <div class="hero-line muted">
          <strong>{earnedCustom}</strong> / {data.custom.length} challenges ·
          <strong>{data.player.worldsScanned}</strong>
          {data.player.worldsScanned === 1 ? "world" : "worlds"} scanned
        </div>
        <div class="cat-bars">
          {#each data.categories as cat (cat.key)}
            <div class="cat-bar">
              <div class="cat-top">
                <Icon name={CATEGORY_ICON[cat.key] ?? "trophy"} size={13} />
                <span>{CATEGORY_LABELS[cat.key] ?? cat.key}</span>
                <span class="cat-count">{cat.earned}/{cat.total}</span>
              </div>
              <div class="track"><div class="fill" style="width:{cat.total ? (cat.earned / cat.total) * 100 : 0}%"></div></div>
            </div>
          {/each}
        </div>
      </div>
    </section>

    <nav class="tabs">
      <button class:active={tab === "achievements"} onclick={() => (tab = "achievements")}>Achievements</button>
      <button class:active={tab === "stats"} onclick={() => (tab = "stats")}>Lifetime Stats</button>
    </nav>

    {#if tab === "achievements"}
      <section class="block">
        <h2>Cactus Challenges <span class="badge">launcher-exclusive</span></h2>
        <div class="custom-grid">
          {#each data.custom as c (c.id)}
            <div class="custom-tile" class:locked={!c.earned}>
              <div class="custom-ic"><Icon name={c.icon} size={20} /></div>
              <div class="custom-body">
                <div class="custom-name">
                  {c.name}
                  {#if c.earned}<Icon name="check" size={13} />{/if}
                </div>
                <div class="custom-desc">{c.description}</div>
                {#if !c.earned && c.progress > 0}
                  <div class="track sm"><div class="fill" style="width:{c.progress}%"></div></div>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </section>

      <section class="block">
        <div class="block-head">
          <h2>Advancements</h2>
          <div class="filters">
            <label class="chk">
              <input type="checkbox" bind:checked={showLocked} /> Show locked
            </label>
            <select bind:value={categoryFilter} class="select">
              <option value="all">All categories</option>
              {#each data.categories as cat (cat.key)}
                <option value={cat.key}>{CATEGORY_LABELS[cat.key] ?? cat.key}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="adv-grid">
          {#each visibleAdvancements as a (a.id)}
            <div class="adv-tile" class:locked={!a.done} title={a.name}>
              <div class="adv-ic"><Icon name={CATEGORY_ICON[a.category] ?? "trophy"} size={18} /></div>
              <div class="adv-body">
                <div class="adv-name">{displayName(a)}</div>
                <div class="adv-desc">{displayDesc(a)}</div>
              </div>
              {#if a.done && a.earnedAt}
                <div class="adv-date">{fmtDate(a.earnedAt)}</div>
              {/if}
            </div>
          {/each}
        </div>
        {#if visibleAdvancements.length === 0}
          <p class="empty">Nothing to show with these filters.</p>
        {/if}
      </section>
    {:else}
      <section class="block">
        <div class="stat-grid">
          <div class="stat"><span class="v">{fmtHours(data.stats.playTimeTicks)}</span><span class="k">Playtime</span></div>
          <div class="stat"><span class="v">{fmtNumber(data.stats.blocksMined)}</span><span class="k">Blocks mined</span></div>
          <div class="stat"><span class="v">{fmtNumber(data.stats.mobsKilled)}</span><span class="k">Mobs killed</span></div>
          <div class="stat"><span class="v">{fmtNumber(data.stats.deaths)}</span><span class="k">Deaths</span></div>
          <div class="stat"><span class="v">{fmtDistance(data.stats.distanceCm)}</span><span class="k">Distance traveled</span></div>
          <div class="stat"><span class="v">{fmtNumber(data.stats.jumps)}</span><span class="k">Jumps</span></div>
          <div class="stat"><span class="v">{fmtNumber(data.stats.itemsPickedUp)}</span><span class="k">Items picked up</span></div>
          <div class="stat"><span class="v">{fmtNumber(data.stats.timesSlept)}</span><span class="k">Nights slept</span></div>
        </div>
      </section>

      <div class="two-col">
        <section class="block">
          <h2>Top mined blocks</h2>
          {#if data.stats.topMined.length}
            <ul class="bars">
              {#each data.stats.topMined as e (e.key)}
                <li>
                  <span class="bar-label">{cleanKey(e.key)}</span>
                  <div class="track"><div class="fill" style="width:{(e.count / (data.stats.topMined[0].count || 1)) * 100}%"></div></div>
                  <span class="bar-val">{fmtNumber(e.count)}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty">No data.</p>
          {/if}
        </section>

        <section class="block">
          <h2>Top mobs defeated</h2>
          {#if data.stats.topKilled.length}
            <ul class="bars">
              {#each data.stats.topKilled as e (e.key)}
                <li>
                  <span class="bar-label">{cleanKey(e.key)}</span>
                  <div class="track"><div class="fill" style="width:{(e.count / (data.stats.topKilled[0].count || 1)) * 100}%"></div></div>
                  <span class="bar-val">{fmtNumber(e.count)}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty">No data.</p>
          {/if}
        </section>
      </div>

      <section class="block">
        <h2>By instance</h2>
        <div class="table">
          <div class="tr th">
            <span>Instance</span><span>Worlds</span><span>Playtime</span><span>Mined</span><span>Killed</span><span>Deaths</span>
          </div>
          {#each data.instances as inst (inst.id)}
            <div class="tr">
              <span class="name">{inst.name}</span>
              <span>{inst.worlds}</span>
              <span>{fmtHours(inst.playTimeTicks)}</span>
              <span>{fmtNumber(inst.blocksMined)}</span>
              <span>{fmtNumber(inst.mobsKilled)}</span>
              <span>{fmtNumber(inst.deaths)}</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 920px;
    margin: 0 auto;
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 22px;
    gap: 16px;
  }
  .head h1 {
    font-size: 24px;
  }
  .sub {
    color: var(--text-muted);
    font-size: 13px;
    margin-top: 4px;
  }
  .sub strong {
    color: var(--text);
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
    padding: 60px 20px;
    color: var(--text-muted);
  }
  .state small {
    max-width: 420px;
    font-size: 12px;
    line-height: 1.5;
  }
  .state.err {
    color: var(--warning);
  }

  .hero {
    display: flex;
    align-items: center;
    gap: 24px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04), inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    padding: 20px 24px;
    margin-bottom: 20px;
  }
  .hero-ring {
    flex-shrink: 0;
    width: 108px;
    height: 108px;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: radial-gradient(closest-side, var(--bg-card) 79%, transparent 80%),
      conic-gradient(var(--accent) calc(var(--pct) * 1%), var(--border) 0);
  }
  .hero-pct {
    font-size: 26px;
    font-weight: 700;
    color: var(--accent);
    line-height: 1;
  }
  .hero-cap {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .hero-meta {
    flex: 1;
    min-width: 0;
  }
  .hero-line {
    font-size: 15px;
    margin-bottom: 4px;
  }
  .hero-line.muted {
    color: var(--text-muted);
    font-size: 13px;
    margin-bottom: 14px;
  }
  .hero-line strong {
    color: var(--text);
  }
  .cat-bars {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px 18px;
  }
  .cat-top {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }
  .cat-count {
    margin-left: auto;
    color: var(--text);
    font-family: var(--font-mono, monospace);
    font-size: 11px;
  }

  .track {
    height: 6px;
    background: var(--border);
    overflow: hidden;
  }
  .track.sm {
    height: 4px;
    margin-top: 6px;
  }
  .fill {
    height: 100%;
    background: var(--accent);
  }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 2px solid var(--border);
    margin-bottom: 18px;
  }
  .tabs button {
    background: none;
    border: none;
    color: var(--text-muted);
    font: inherit;
    font-size: 14px;
    padding: 8px 14px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .block {
    margin-bottom: 26px;
  }
  .block h2 {
    font-size: 15px;
    margin-bottom: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .block-head h2 {
    margin-bottom: 0;
  }
  .badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: var(--accent-soft);
    padding: 2px 7px;
  }

  .filters {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    cursor: pointer;
  }
  .select {
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--border);
    padding: 5px 8px;
    font: inherit;
    font-size: 12px;
  }

  .custom-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 10px;
  }
  .custom-tile {
    display: flex;
    gap: 12px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    padding: 12px 14px;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.03), inset -2px -2px 0 rgba(0, 0, 0, 0.25);
  }
  .custom-tile.locked {
    opacity: 0.55;
  }
  .custom-ic {
    flex-shrink: 0;
    width: 38px;
    height: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .custom-tile.locked .custom-ic {
    background: var(--border);
    color: var(--text-muted);
  }
  .custom-body {
    min-width: 0;
    flex: 1;
  }
  .custom-name {
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--text);
  }
  .custom-name :global(.hn) {
    color: var(--accent);
  }
  .custom-desc {
    font-size: 11.5px;
    color: var(--text-muted);
    line-height: 1.4;
    margin-top: 2px;
  }

  .adv-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 8px;
  }
  .adv-tile {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    padding: 9px 11px;
  }
  .adv-tile.locked {
    opacity: 0.42;
  }
  .adv-ic {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .adv-tile.locked .adv-ic {
    background: var(--border);
    color: var(--text-muted);
  }
  .adv-body {
    min-width: 0;
    flex: 1;
  }
  .adv-name {
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .adv-desc {
    font-size: 10.5px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .adv-date {
    flex-shrink: 0;
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono, monospace);
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    padding: 14px 16px;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.03), inset -2px -2px 0 rgba(0, 0, 0, 0.25);
  }
  .stat .v {
    font-size: 20px;
    font-weight: 700;
    color: var(--accent);
  }
  .stat .k {
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .two-col {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }
  @media (max-width: 640px) {
    .two-col {
      grid-template-columns: 1fr;
    }
  }
  .bars {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .bars li {
    display: grid;
    grid-template-columns: 110px 1fr 60px;
    align-items: center;
    gap: 10px;
  }
  .bar-label {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bar-val {
    font-size: 11px;
    color: var(--text-muted);
    text-align: right;
    font-family: var(--font-mono, monospace);
  }

  .table {
    display: flex;
    flex-direction: column;
    border: 2px solid var(--border);
  }
  .tr {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr 1fr 1fr 1fr;
    gap: 8px;
    padding: 9px 14px;
    font-size: 12.5px;
    border-bottom: 1px solid var(--border);
  }
  .tr:last-child {
    border-bottom: none;
  }
  .tr.th {
    color: var(--text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: rgba(0, 0, 0, 0.15);
  }
  .tr .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    color: var(--text-muted);
    font-size: 12px;
    padding: 8px 0;
  }

  .btn.ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 7px 12px;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .btn.ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
