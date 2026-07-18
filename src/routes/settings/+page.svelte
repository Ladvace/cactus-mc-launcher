<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { api } from "$lib/api";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Icon from "$lib/components/Icon.svelte";
  import ProgressBar from "$lib/components/ProgressBar.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import {
    backgroundCss,
    bgKind,
    parsePattern,
    parseImage,
    parseTexture,
    PATTERNS,
    DEFAULT_COLOR,
  } from "$lib/background";
  import { fileToBackgroundDataUri } from "$lib/image";
  import { pickFolder } from "$lib/dialog";
  import { THEME_PRESETS, DECOR_THEMES } from "$lib/themes";
  import { LINKS } from "$lib/links";
  import { playClick } from "$lib/sound";
  import type { CacheStats, DockPosition, Settings } from "$lib/types";

  const DOCK_POSITIONS: { value: DockPosition; label: string }[] = [
    { value: "bottom", label: "Bottom" },
    { value: "top", label: "Top" },
    { value: "left", label: "Left" },
    { value: "right", label: "Right" },
  ];

  // Local editable copy; committed on "Save".
  let draft = $state<Settings>({ ...settingsStore.settings });
  let saved = $state(false);
  let saving = $state(false);

  let cacheStats = $state<CacheStats | null>(null);
  let cacheLoading = $state(false);

  // The Giphy key persists on its own (on blur) so it's remembered even without
  // pressing "Save changes" — it behaves like connecting a credential.
  async function saveGiphyKey() {
    const key = draft.giphyApiKey.trim();
    draft.giphyApiKey = key;
    if (key === (settingsStore.settings.giphyApiKey ?? "")) return;
    await settingsStore.save({ ...draft, giphyApiKey: key });
  }

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

  let clearingCache = $state(false);
  async function clearCache() {
    clearingCache = true;
    try {
      cacheStats = await api.clearContentCache();
      toast.success("Cache cleared.");
    } catch (error) {
      toast.error(String(error));
    } finally {
      clearingCache = false;
    }
  }

  let resetOpen = $state(false);
  let resetting = $state(false);
  async function resetEverything() {
    resetting = true;
    try {
      await api.resetAppData();
      localStorage.clear();
      location.reload();
    } catch (error) {
      toast.error(String(error));
      resetting = false;
    }
  }
  $effect(() => {
    loadCache();
  });

  function formatBytes(n: number): string {
    if (n <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const unitIndex = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
    return `${(n / Math.pow(1024, unitIndex)).toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
  }

  const kind = $derived(bgKind(draft.background ?? ""));
  const colorValue = $derived.by(() => {
    const value = draft.background.startsWith("color:") ? draft.background.slice(6) : "";
    // Decor-theme presets store a gradient here; the colour input needs a hex.
    return /^#[0-9a-fA-F]{6}$/.test(value) ? value : DEFAULT_COLOR;
  });
  const activePattern = $derived(
    kind === "pattern" ? parsePattern(draft.background).name : ""
  );
  // Chosen base/tint colours for pattern & image backgrounds (default when unset).
  const patternColor = $derived(
    kind === "pattern" ? (parsePattern(draft.background).color ?? DEFAULT_COLOR) : DEFAULT_COLOR
  );
  const imageColor = $derived(
    kind === "image" ? (parseImage(draft.background).color ?? DEFAULT_COLOR) : DEFAULT_COLOR
  );
  const textureOpacity = $derived(
    kind === "texture" ? parseTexture(draft.background).opacity : 0.5
  );

  let bgFileInput = $state<HTMLInputElement>();

  function setColor(value: string) {
    draft.background = `color:${value}`;
  }
  // Set the pattern, keeping any chosen base colour.
  function setPattern(name: string) {
    const { color } = parsePattern(draft.background);
    draft.background = color ? `pattern:${name}|${color}` : `pattern:${name}`;
  }
  function setPatternColor(color: string) {
    const { name } = parsePattern(draft.background);
    draft.background = `pattern:${name || "dots"}|${color}`;
  }
  function setImageColor(color: string) {
    const { uri } = parseImage(draft.background);
    if (uri) draft.background = `image:${color}|${uri}`;
  }
  function setTextureOpacity(value: number) {
    const { uri, color } = parseTexture(draft.background);
    const colorPrefix = color ? `${color}|` : "";
    draft.background = `texture:${value}|${colorPrefix}${uri}`;
  }
  async function onBgFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
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

  // Managed Java already covers these majors; these let you point a version at
  // your own JDK instead (matches what each Minecraft version requires).
  const JAVA_MAJORS = [8, 17, 21];
  // Installed managed Java paths (shown when a major is auto-managed).
  let managedPaths = $state<Record<string, string>>({});
  $effect(() => {
    api.resolvedJavaPaths().then((paths) => (managedPaths = paths)).catch(() => {});
  });
  // Which majors the user has switched to a custom path.
  let javaEditing = $state<Record<string, boolean>>({});

  function javaManaged(major: number): boolean {
    const key = String(major);
    return !(javaEditing[key] ?? Boolean(draft.javaPaths[key]?.trim()));
  }
  function setJavaPath(major: number, value: string) {
    const paths = { ...draft.javaPaths };
    if (value) paths[String(major)] = value;
    else delete paths[String(major)];
    draft.javaPaths = paths;
  }
  function toggleJavaManaged(major: number, auto: boolean) {
    const key = String(major);
    javaEditing = { ...javaEditing, [key]: !auto };
    if (auto) {
      setJavaPath(major, ""); // back to managed
    } else if (!draft.javaPaths[key]) {
      setJavaPath(major, managedPaths[key] ?? ""); // prefill with the managed path
    }
  }

  async function browseInstancesDir() {
    const folder = await pickFolder("Choose where new instances install");
    if (folder) draft.instancesDir = folder;
  }

  // The global ngrok key persists on its own (like a connected credential); a
  // per-server key set on the instance overrides it.
  async function saveNgrokToken() {
    const token = draft.ngrokAuthtoken.trim();
    draft.ngrokAuthtoken = token;
    if (token === (settingsStore.settings.ngrokAuthtoken ?? "")) return;
    await settingsStore.save({ ...draft, ngrokAuthtoken: token });
  }

  // Sync the draft once settings finish loading.
  $effect(() => {
    if (settingsStore.loaded) {
      draft = { ...settingsStore.settings };
    }
  });

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
      (event) => {
        javaLabel = event.payload.label;
        javaCur = event.payload.current;
        javaTotal = event.payload.total;
      }
    ).then((fn) => (unlisten = fn));
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
      managedPaths = await api.resolvedJavaPaths();
    } catch (err) {
      javaError = String(err);
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

{#snippet colorField(value: string, onPick: (v: string) => void)}
  <input
    type="color"
    class="color-input"
    {value}
    oninput={(event) => onPick(event.currentTarget.value)}
  />
  <span class="hex">{value}</span>
{/snippet}

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
    <h3>Interface</h3>
    <div class="setting">
      <div class="label">
        <span>Button sounds</span>
        <small>Play a soft click when you press buttons.</small>
      </div>
      <label class="switch">
        <input
          type="checkbox"
          bind:checked={draft.uiSounds}
          onchange={() => draft.uiSounds && playClick()}
        />
        <span class="track"><span class="thumb"></span></span>
      </label>
    </div>

    <div class="setting">
      <div class="label">
        <span>Giphy API key (stickers)</span>
        <small>
          Animated stickers are off by default. Paste a free key from
          <button class="linkish" onclick={() => openUrl("https://developers.giphy.com")}>
            developers.giphy.com
          </button>
          to enable the Stickers tab. The emoji picker always works.
        </small>
      </div>
      <input
        class="input narrow"
        type="password"
        placeholder="Paste key to enable"
        bind:value={draft.giphyApiKey}
        onblur={saveGiphyKey}
        onkeydown={(event) => event.key === "Enter" && event.currentTarget.blur()}
        autocomplete="off"
        spellcheck="false"
      />
    </div>

    <div class="setting">
      <div class="label">
        <span>Dock position</span>
        <small>Which edge of the window the app dock sits on.</small>
      </div>
      <div class="seg">
        {#each DOCK_POSITIONS as position}
          <button
            class="seg-btn"
            class:on={draft.dockPosition === position.value}
            onclick={() => (draft.dockPosition = position.value)}
          >
            {position.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="setting">
      <div class="label">
        <span>Magnify dock on hover</span>
        <small>The macOS-style zoom as you move across dock icons.</small>
      </div>
      <label class="switch">
        <input type="checkbox" bind:checked={draft.dockMagnify} />
        <span class="track"><span class="thumb"></span></span>
      </label>
    </div>
  </section>

  <section class="card-block">
    <h3>Appearance</h3>

    <div class="label themes-label">
      <span>Theme presets</span>
      <small>One-click backgrounds — solid colours, patterns, and cactus decor.</small>
    </div>
    <div class="themes">
      {#each THEME_PRESETS as preset (preset.name)}
        <button
          class="theme"
          class:on={draft.background === preset.bg && (draft.decorTheme ?? "") === (preset.decor ?? "")}
          onclick={() => {
            draft.background = preset.bg;
            draft.decorTheme = preset.decor ?? "";
          }}
          title={preset.name}
        >
          <span class="theme-swatch" style="background: {backgroundCss(preset.bg)};">
            {#if preset.decor}
              <img
                class="theme-decor"
                src={DECOR_THEMES.find((decor) => decor.id === preset.decor)?.placements[0].sprite}
                alt=""
              />
            {/if}
          </span>
          <span class="theme-name">{preset.name}</span>
        </button>
      {/each}
    </div>

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
        {@render colorField(colorValue, setColor)}
      </div>
    {:else if kind === "pattern"}
      <div class="bg-detail patterns">
        {#each PATTERNS as pattern}
          <button
            class="swatch"
            class:on={activePattern === pattern}
            style="background: {backgroundCss(`pattern:${pattern}|${patternColor}`)};"
            title={pattern}
            aria-label={pattern}
            onclick={() => setPattern(pattern)}
          ></button>
        {/each}
      </div>
      <div class="bg-detail bg-color">
        <span class="bg-color-label">Base color</span>
        {@render colorField(patternColor, setPatternColor)}
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
      <div class="bg-detail bg-color">
        <span class="bg-color-label">Tint</span>
        {@render colorField(imageColor, setImageColor)}
      </div>
    {:else if kind === "texture"}
      <div class="bg-detail bg-color">
        <span class="bg-color-label">Texture opacity</span>
        <input
          type="range"
          class="opacity-range"
          min="0.1"
          max="1"
          step="0.05"
          value={textureOpacity}
          oninput={(event) => setTextureOpacity(parseFloat(event.currentTarget.value))}
        />
        <span class="hex">{Math.round(textureOpacity * 100)}%</span>
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
          <ProgressBar label={javaLabel || "Preparing…"} pct={javaPct} />
        {:else if javaError}
          <p class="java-err">{javaError}</p>
        {:else}
          <p class="java-ok">✓ Ready: {javaInstalled.join(", ")}</p>
        {/if}
      </div>
    {/if}
    <div class="setting">
      <div class="label">
        <span>Java runtimes (optional)</span>
        <small>
          Leave empty to use managed Java. Point a version at your own JDK if you
          prefer — each Minecraft version uses the Java it needs (8 for old
          versions, 17 for 1.17–1.20, 21 for 1.20.5+).
        </small>
      </div>
    </div>
    {#each JAVA_MAJORS as major (major)}
      <div class="setting java-major">
        <div class="label">
          <span>Java {major}</span>
          <small>
            {javaManaged(major)
              ? managedPaths[String(major)]
                ? "Managed by Cactus"
                : "Downloaded automatically when needed"
              : "Using your own Java"}
          </small>
        </div>
        <label class="switch" title="Manage automatically">
          <input
            type="checkbox"
            checked={javaManaged(major)}
            onchange={(event) => toggleJavaManaged(major, event.currentTarget.checked)}
          />
          <span class="track"><span class="thumb"></span></span>
        </label>
      </div>
      <input
        class="input java-input"
        class:managed={javaManaged(major)}
        disabled={javaManaged(major)}
        placeholder="/path/to/java"
        value={javaManaged(major)
          ? (managedPaths[String(major)] ?? "")
          : (draft.javaPaths[String(major)] ?? "")}
        oninput={(event) => setJavaPath(major, event.currentTarget.value)}
        spellcheck="false"
      />
    {/each}
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
    <h3>Servers</h3>
    <div class="setting">
      <div class="label">
        <span>ngrok authtoken</span>
        <small>
          Lets you share a server over the internet (no port-forwarding). Get a
          free token at
          <button class="linkish" onclick={() => openUrl("https://dashboard.ngrok.com/get-started/your-authtoken")}>
            ngrok.com
          </button>. A server can override this with its own key.
        </small>
      </div>
      <input
        class="input narrow"
        type="password"
        placeholder="Paste your ngrok authtoken"
        bind:value={draft.ngrokAuthtoken}
        onblur={saveNgrokToken}
        onkeydown={(event) => event.key === "Enter" && event.currentTarget.blur()}
        autocomplete="off"
        spellcheck="false"
      />
    </div>
  </section>

  <section class="card-block">
    <h3>Storage</h3>
    <div class="setting">
      <div class="label">
        <span>Instances folder</span>
        <small>
          Where new instances' game data (mods, saves, worlds) is installed.
          Existing instances stay where they are.
        </small>
        <small class="path">{draft.instancesDir || "Default (app data folder)"}</small>
      </div>
      <div class="folder-actions">
        <button class="btn ghost" onclick={browseInstancesDir}>Browse…</button>
        {#if draft.instancesDir}
          <button class="btn ghost" onclick={() => (draft.instancesDir = "")}>Reset</button>
        {/if}
      </div>
    </div>
    <div class="setting">
      <div class="label">
        <span>Shared content cache</span>
        <small
          >Mods, resource packs & shaders are downloaded once and hard-linked
          into each instance — identical files never take space twice.</small
        >
      </div>
      <div class="folder-actions">
        <button class="btn ghost" onclick={clearCache} disabled={cacheLoading || clearingCache}>
          {clearingCache ? "Clearing…" : "Clear cache"}
        </button>
        <button class="btn ghost" onclick={loadCache} disabled={cacheLoading}>
          {cacheLoading ? "…" : "Refresh"}
        </button>
      </div>
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

    <div class="setting danger-row">
      <div class="label">
        <span>Reset everything</span>
        <small>
          Delete all instances, downloads, and settings and start fresh. This
          can't be undone.
        </small>
      </div>
      <button class="btn danger" onclick={() => (resetOpen = true)}>
        <Icon name="trash" size={14} /> Reset…
      </button>
    </div>
  </section>

  <section class="card-block">
    <h3>About</h3>
    <p class="about-app">Cactus Launcher — spiky but not spooky.</p>
    <div class="about-links">
      {#if LINKS.github}
        <button class="btn ghost sm" onclick={() => openUrl(LINKS.github)}>
          <Icon name="cube" size={14} /> GitHub
        </button>
      {/if}
      {#if LINKS.discord}
        <button class="btn ghost sm" onclick={() => openUrl(LINKS.discord)}>
          <Icon name="users" size={14} /> Discord
        </button>
      {/if}
    </div>
    <p class="about-credit">
      Created by
      {#if LINKS.website}
        <button class="linkish" onclick={() => openUrl(LINKS.website)}>{LINKS.authorName}</button>
      {:else}
        <strong>{LINKS.authorName}</strong>
      {/if}
    </p>
  </section>

  <div class="save-bar">
    {#if saved}<span class="saved">Saved ✓</span>{/if}
    <button class="btn primary" onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save changes"}
    </button>
  </div>
</div>

<Modal title="Reset everything?" open={resetOpen} onClose={() => (resetOpen = false)} width={430}>
  <p class="reset-warn">
    This permanently deletes <strong>all instances, downloads, and settings</strong>
    and restarts the app fresh. This can't be undone.
  </p>
  <div class="reset-actions">
    <button class="btn ghost" onclick={() => (resetOpen = false)}>Cancel</button>
    <button class="btn danger" onclick={resetEverything} disabled={resetting}>
      {resetting ? "Resetting…" : "Delete everything"}
    </button>
  </div>
</Modal>

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
  /* Toggle switch */
  .switch {
    flex-shrink: 0;
    cursor: pointer;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .switch .track {
    display: inline-flex;
    align-items: center;
    width: 44px;
    height: 24px;
    padding: 2px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    transition: background 0.12s, border-color 0.12s;
  }
  .switch .thumb {
    width: 16px;
    height: 16px;
    background: var(--text-muted);
    transition: transform 0.14s ease, background 0.12s;
  }
  .switch input:checked + .track {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .switch input:checked + .track .thumb {
    transform: translateX(20px);
    background: var(--accent);
  }
  .switch input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
  .themes-label {
    margin-bottom: 10px;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
    gap: 8px;
    margin-bottom: 18px;
  }
  .theme {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 0;
    background: transparent;
    border: none;
    text-align: left;
  }
  .theme-swatch {
    position: relative;
    height: 44px;
    border: 2px solid var(--border);
    overflow: hidden;
  }
  .theme-decor {
    position: absolute;
    right: 3px;
    bottom: 2px;
    width: 26px;
    height: auto;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.5));
  }
  .theme:hover .theme-swatch {
    border-color: var(--accent);
  }
  .theme.on .theme-swatch {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .theme-name {
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  .theme.on .theme-name {
    color: var(--accent);
  }
  .bg-color-label {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .opacity-range {
    flex: 1;
    cursor: pointer;
    accent-color: var(--accent);
  }
  .path {
    font-family: var(--font-pixel);
    font-size: 11px;
    color: var(--accent);
    word-break: break-all;
  }
  .folder-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .danger-row {
    border-top: 1px solid rgba(255, 91, 91, 0.2);
    margin-top: 6px;
    padding-top: 16px;
  }
  .danger-row .label > span:first-child {
    color: var(--danger);
  }
  .reset-warn {
    margin: 0 0 18px;
    color: var(--text-secondary);
    line-height: 1.6;
  }
  .reset-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .java-major {
    padding-bottom: 2px;
  }
  .java-input {
    width: 100%;
    margin: 0 0 10px;
    font-size: 12.5px;
  }
  .java-input.managed {
    opacity: 0.55;
    cursor: not-allowed;
  }
  /* Segmented control (dock position). */
  .seg {
    display: flex;
    flex-shrink: 0;
  }
  .seg-btn {
    position: relative;
    padding: 8px 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 600;
    /* Overlap the neighbour's border so the seam is a single line, not double. */
    margin-left: -2px;
  }
  .seg-btn:first-child {
    margin-left: 0;
  }
  .seg-btn:hover {
    color: var(--text);
  }
  .seg-btn.on {
    color: var(--accent);
    background: var(--accent-soft);
    border-color: var(--accent);
    /* Raise the active button so its full accent border sits above neighbours. */
    z-index: 1;
  }
  .save-bar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    padding-top: 4px;
  }
  .about-app {
    margin: 0 0 12px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .about-links {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .about-credit {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-muted);
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
  .linkish {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
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
