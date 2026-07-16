<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { api } from "$lib/api";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "$lib/components/Icon.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import {
    backgroundCss,
    bgKind,
    PATTERNS,
    DEFAULT_COLOR,
  } from "$lib/background";
  import { fileToBackgroundDataUri } from "$lib/image";
  import type { CacheStats, Settings } from "$lib/types";

  // Local editable copy; committed on "Save".
  let draft = $state<Settings>({ ...settingsStore.settings });
  let saved = $state(false);
  let saving = $state(false);

  // --- Shared content cache stats ---
  let cacheStats = $state<CacheStats | null>(null);
  let cacheLoading = $state(false);

  async function loadCache() {
    cacheLoading = true;
    try {
      cacheStats = await api.contentCacheStats();
    } catch {
      cacheStats = null;
    } finally {
      cacheLoading = false;
    }
  }
  $effect(() => {
    loadCache();
  });

  function formatBytes(n: number): string {
    if (n <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
    return `${(n / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  // --- Background ---
  const kind = $derived(bgKind(draft.background ?? ""));
  const colorValue = $derived(
    draft.background.startsWith("color:")
      ? draft.background.slice(6)
      : DEFAULT_COLOR
  );
  const activePattern = $derived(
    draft.background.startsWith("pattern:") ? draft.background.slice(8) : ""
  );

  let bgFileInput = $state<HTMLInputElement>();

  function setColor(v: string) {
    draft.background = `color:${v}`;
  }
  function setPattern(name: string) {
    draft.background = `pattern:${name}`;
  }
  async function onBgFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    try {
      draft.background = `image:${await fileToBackgroundDataUri(file)}`;
    } catch (err) {
      console.error("background upload failed", err);
    }
  }
  function pickBgSticker() {
    ui.openStickerPicker("App background", (uri) => {
      draft.background = `image:${uri}`;
    });
  }

  function resetResolution() {
    draft.gameWidth = 854;
    draft.gameHeight = 480;
  }

  // Sync the draft once settings finish loading.
  $effect(() => {
    if (settingsStore.loaded) {
      draft = { ...settingsStore.settings };
    }
  });

  // --- Managed Java auto-setup ---
  let javaBusy = $state(false);
  let javaLabel = $state("");
  let javaCur = $state(0);
  let javaTotal = $state(0);
  let javaInstalled = $state<string[]>([]);
  let javaError = $state<string | null>(null);

  const javaPct = $derived(
    javaTotal > 0 ? Math.round((javaCur / javaTotal) * 100) : null
  );

  $effect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ label: string; current: number; total: number }>(
      "java-setup",
      (e) => {
        javaLabel = e.payload.label;
        javaCur = e.payload.current;
        javaTotal = e.payload.total;
      }
    ).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  async function setupJava() {
    javaBusy = true;
    javaError = null;
    javaInstalled = [];
    javaLabel = "Fetching runtime list…";
    javaCur = 0;
    javaTotal = 0;
    try {
      javaInstalled = await api.setupJava();
    } catch (e) {
      javaError = String(e);
    } finally {
      javaBusy = false;
      javaLabel = "";
      javaCur = 0;
      javaTotal = 0;
    }
  }

  async function save() {
    saving = true;
    try {
      await settingsStore.save({ ...draft });
      saved = true;
      setTimeout(() => (saved = false), 1800);
    } finally {
      saving = false;
    }
  }
</script>

<div class="page">
  <header class="head">
    <h1>Settings</h1>
    <div class="head-actions">
      {#if saved}<span class="saved">Saved ✓</span>{/if}
      <button class="btn primary" onclick={save} disabled={saving}>
        {saving ? "Saving…" : "Save changes"}
      </button>
    </div>
  </header>

  <section class="card-block">
    <h3>Account</h3>
    <div class="setting">
      <div class="label">
        <span>Offline username</span>
        <small>Used for offline launches until Microsoft sign-in is added.</small>
      </div>
      <input
        class="input narrow"
        placeholder="Player"
        bind:value={draft.offlineUsername}
        autocomplete="off"
      />
    </div>
  </section>

  <section class="card-block">
    <h3>Appearance</h3>
    <div class="setting bg-setting">
      <div class="label">
        <span>Background</span>
        <small>
          Give the app a solid colour, a pattern, or your own image / GIF /
          sticker.
        </small>
      </div>
      <div
        class="bg-preview"
        style="background: {backgroundCss(draft.background)};"
      ></div>
    </div>

    <div class="bg-kinds">
      <button
        class="chip"
        class:on={kind === "default"}
        onclick={() => (draft.background = "")}
      >
        Default
      </button>
      <button
        class="chip"
        class:on={kind === "color"}
        onclick={() => setColor(colorValue)}
      >
        Solid color
      </button>
      <button
        class="chip"
        class:on={kind === "pattern"}
        onclick={() => setPattern(activePattern || "dots")}
      >
        Pattern
      </button>
      <button
        class="chip"
        class:on={kind === "image"}
        onclick={pickBgSticker}
      >
        Image / GIF
      </button>
    </div>

    {#if kind === "color"}
      <div class="bg-detail">
        <input
          type="color"
          class="color-input"
          value={colorValue}
          oninput={(e) => setColor(e.currentTarget.value)}
        />
        <span class="hex">{colorValue}</span>
      </div>
    {:else if kind === "pattern"}
      <div class="bg-detail patterns">
        {#each PATTERNS as p}
          <button
            class="swatch"
            class:on={activePattern === p}
            style="background: {backgroundCss(`pattern:${p}`)};"
            title={p}
            aria-label={p}
            onclick={() => setPattern(p)}
          ></button>
        {/each}
      </div>
    {:else if kind === "image"}
      <div class="bg-detail">
        <button class="btn ghost" onclick={() => bgFileInput?.click()}>
          <Icon name="edit" size={14} /> Upload…
        </button>
        <button class="btn ghost" onclick={pickBgSticker}>
          <Icon name="sparkles" size={14} /> Stickers & GIFs…
        </button>
      </div>
    {/if}
    <input
      bind:this={bgFileInput}
      type="file"
      accept="image/png,image/jpeg,image/gif,image/webp"
      style="display:none"
      onchange={onBgFile}
    />
  </section>

  <section class="card-block">
    <h3>Java & Memory</h3>
    <div class="setting">
      <div class="label">
        <span>Managed Java</span>
        <small>
          Pre-download the runtimes Minecraft needs (Java 8, 17 & 21). Used
          automatically at launch — you don't need to set a path below.
        </small>
      </div>
      <button class="btn primary" onclick={setupJava} disabled={javaBusy}>
        {javaBusy ? "Installing…" : "Auto-install Java"}
      </button>
    </div>
    {#if javaBusy || javaInstalled.length > 0 || javaError}
      <div class="java-status">
        {#if javaBusy}
          <div class="progress-head">
            <span>{javaLabel || "Preparing…"}</span>
            {#if javaPct !== null}<span>{javaPct}%</span>{/if}
          </div>
          <div class="bar">
            <div
              class="bar-fill"
              class:indeterminate={javaPct === null}
              style={javaPct !== null ? `width:${javaPct}%` : ""}
            ></div>
          </div>
        {:else if javaError}
          <p class="java-err">{javaError}</p>
        {:else}
          <p class="java-ok">✓ Ready: {javaInstalled.join(", ")}</p>
        {/if}
      </div>
    {/if}
    <div class="setting">
      <div class="label">
        <span>Java path (optional)</span>
        <small>
          Leave empty to use managed Java. Only set this to force your own Java
          executable.
        </small>
      </div>
      <input
        class="input narrow"
        placeholder="/path/to/java"
        bind:value={draft.javaPath}
      />
    </div>
    <div class="setting">
      <div class="label">
        <span>Maximum memory</span>
        <small>{draft.maxMemoryMb} MB</small>
      </div>
      <input
        type="range"
        min="1024"
        max="16384"
        step="512"
        bind:value={draft.maxMemoryMb}
        class="range"
      />
    </div>
    <div class="setting">
      <div class="label">
        <span>JVM arguments</span>
        <small>Extra flags passed to the JVM at launch.</small>
      </div>
      <input
        class="input narrow"
        placeholder="-XX:+UseG1GC …"
        bind:value={draft.jvmArgs}
      />
    </div>
  </section>

  <section class="card-block">
    <h3>Game window</h3>
    <div class="setting">
      <div class="label"><span>Default resolution</span></div>
      <div class="res">
        <input type="number" class="input tiny" bind:value={draft.gameWidth} />
        <span>×</span>
        <input type="number" class="input tiny" bind:value={draft.gameHeight} />
        <button
          class="btn ghost"
          onclick={resetResolution}
          disabled={draft.gameWidth === 854 && draft.gameHeight === 480}
          title="Reset to 854 × 480"
        >
          <Icon name="refresh" size={14} /> Reset
        </button>
      </div>
    </div>
  </section>

  <section class="card-block">
    <h3>Storage</h3>
    <div class="setting">
      <div class="label">
        <span>Shared content cache</span>
        <small
          >Mods, resource packs & shaders are downloaded once and hard-linked
          into each instance — identical files never take space twice.</small
        >
      </div>
      <button class="btn ghost" onclick={loadCache} disabled={cacheLoading}>
        {cacheLoading ? "…" : "Refresh"}
      </button>
    </div>
    {#if cacheStats}
      <div class="stats">
        <div class="stat">
          <span class="n">{cacheStats.files}</span>
          <span class="l">unique files</span>
        </div>
        <div class="stat">
          <span class="n">{formatBytes(cacheStats.bytes)}</span>
          <span class="l">on disk</span>
        </div>
        <div class="stat save">
          <span class="n">{formatBytes(cacheStats.savedBytes)}</span>
          <span class="l">saved by sharing</span>
        </div>
      </div>
    {/if}
  </section>
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 760px;
    margin: 0 auto;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }
  .head h1 {
    font-size: 24px;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .saved {
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
  }
  .card-block {
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    padding: 18px 20px;
    margin-bottom: 18px;
  }
  .card-block h3 {
    font-size: 14px;
    margin-bottom: 8px;
  }
  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 14px 0;
    border-top: 1px solid var(--border-subtle);
  }
  .card-block h3 + .setting {
    border-top: none;
  }
  .label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .label small {
    font-size: 12px;
    color: var(--text-muted);
    max-width: 420px;
    line-height: 1.4;
  }
  .stats {
    display: flex;
    gap: 10px;
    padding-top: 14px;
    border-top: 1px solid var(--border-subtle);
  }
  .stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .stat .n {
    font-family: var(--font-pixel);
    font-size: 18px;
    color: var(--text);
  }
  .stat .l {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stat.save .n {
    color: var(--accent);
  }
  /* Background picker */
  .bg-preview {
    width: 108px;
    height: 60px;
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
    flex-shrink: 0;
  }
  .bg-kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 14px 0 0;
    border-top: 1px solid var(--border-subtle);
  }
  .chip {
    padding: 7px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 600;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.22);
    transition: border-color 0.1s, color 0.1s, background 0.1s;
  }
  .chip:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .chip.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }
  .bg-detail {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-top: 14px;
  }
  .color-input {
    width: 52px;
    height: 40px;
    padding: 0;
    background: var(--bg-input);
    border: 2px solid var(--border);
    cursor: pointer;
  }
  .hex {
    font-family: var(--font-pixel);
    font-size: 14px;
    color: var(--text-secondary);
    text-transform: uppercase;
  }
  .patterns {
    flex-wrap: wrap;
  }
  .swatch {
    width: 56px;
    height: 40px;
    border: 2px solid var(--border);
    border-radius: 0;
    cursor: pointer;
    transition: border-color 0.1s, transform 0.08s;
  }
  .swatch:hover {
    border-color: var(--accent);
    transform: scale(1.05);
  }
  .swatch.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .label span {
    font-size: 13.5px;
    font-weight: 500;
  }
  .label small {
    color: var(--text-muted);
    font-size: 12px;
  }
  .narrow {
    width: 240px;
    flex-shrink: 0;
  }
  .range {
    width: 240px;
    accent-color: var(--accent);
  }
  .res {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }
  .tiny {
    width: 90px;
  }

  /* Managed Java status */
  .java-status {
    padding: 0 0 14px;
    max-width: 100%;
  }
  .progress-head {
    display: flex;
    justify-content: space-between;
    font-size: 12.5px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .bar {
    height: 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    overflow: hidden;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.3);
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    background-image: repeating-linear-gradient(
      90deg,
      rgba(0, 0, 0, 0.18) 0 2px,
      transparent 2px 8px
    );
    transition: width 0.2s steps(16);
  }
  .bar-fill.indeterminate {
    width: 35%;
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    0% {
      margin-left: -35%;
    }
    100% {
      margin-left: 100%;
    }
  }
  .java-ok {
    margin: 0;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
  }
  .java-err {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }
</style>
