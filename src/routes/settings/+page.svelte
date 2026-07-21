<script lang="ts">
  import { untrack } from "svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { t, type MessageKey } from "$lib/i18n";
  import LanguageSelect from "$lib/components/LanguageSelect.svelte";
  import { sliderFill } from "$lib/slider";
  import { formatDate } from "$lib/time";
  import { api } from "$lib/api";
  import { listen } from "@tauri-apps/api/event";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Icon from "$lib/components/Icon.svelte";
  import ProgressBar from "$lib/components/ProgressBar.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { updater } from "$lib/stores/updater.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { toPct } from "$lib/stores/install.svelte";
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

  const DOCK_POSITIONS: { value: DockPosition; labelKey: MessageKey }[] = [
    { value: "bottom", labelKey: "settings.dockBottom" },
    { value: "top", labelKey: "settings.dockTop" },
    { value: "left", labelKey: "settings.dockLeft" },
    { value: "right", labelKey: "settings.dockRight" },
  ];

  let draft = $state<Settings>({ ...settingsStore.settings });
  let saved = $state(false);
  let saving = $state(false);

  // Live sample of the chosen date format.
  const fmtDatePreview = $derived(formatDate(new Date().toISOString(), draft.dateFormat));

  // Labelled ticks under the max-memory slider (a few GB marks).
  const MEM_MIN = 1024;
  const MEM_MAX = 16384;
  const memTicks = [1, 4, 8, 12, 16].map((gb, i, arr) => ({
    gb,
    pct: ((gb * 1024 - MEM_MIN) / (MEM_MAX - MEM_MIN)) * 100,
    align: i === 0 ? "start" : i === arr.length - 1 ? "end" : "mid",
  }));

  let cacheStats = $state<CacheStats | null>(null);
  let cacheLoading = $state(false);

  // Credential-style fields (Giphy key, ngrok token) persist on their own on
  // blur so they're remembered even without pressing "Save changes". A per-server
  // ngrok key set on an instance overrides the global token saved here.
  async function persistCredential(field: "giphyApiKey" | "ngrokAuthtoken") {
    const value = draft[field].trim();
    draft[field] = value;
    if (value === (settingsStore.settings[field] ?? "")) return;
    await settingsStore.save({ ...draft, [field]: value });
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

  let appVersion = $state("");
  $effect(() => {
    getVersion().then((version) => (appVersion = version)).catch(() => {});
  });

  let clearingCache = $state(false);
  async function clearCache() {
    clearingCache = true;
    try {
      cacheStats = await api.clearContentCache();
      toast.success(t("settings.cacheCleared"));
    } catch (error) {
      toast.error(String(error));
    } finally {
      clearingCache = false;
    }
  }

  let dataDir = $state("");
  $effect(() => {
    api.getDataDir().then((dir) => (dataDir = dir)).catch(() => {});
  });
  // Moving data while a game runs from it (esp. shared meta/) would break it.
  const anyRunning = $derived(
    instancesStore.instances.some(
      (instance) => launchStore.isRunning(instance.id) || launchStore.isBusy(instance.id)
    )
  );
  let pendingDataDir = $state<string | null>(null);
  let dataMoveOpen = $state(false);
  let movingData = $state(false);

  async function browseDataDir() {
    const folder = await pickFolder(t("settings.chooseDataFolder"));
    if (folder && folder !== dataDir) {
      pendingDataDir = folder;
      dataMoveOpen = true;
    }
  }
  function resetDataDir() {
    pendingDataDir = null; // null = back to default
    dataMoveOpen = true;
  }
  async function confirmMoveData() {
    movingData = true;
    try {
      await api.setDataDir(pendingDataDir);
      location.reload();
    } catch (error) {
      toast.error(String(error));
      movingData = false;
      dataMoveOpen = false;
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

  function formatBytes(bytes: number): string {
    if (bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const unitIndex = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
    return `${(bytes / Math.pow(1024, unitIndex)).toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
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
    ui.openStickerPicker(t("settings.appBackground"), (uri) => {
      draft.background = `image:${uri}`;
    });
  }

  function resetResolution() {
    draft.gameWidth = 854;
    draft.gameHeight = 480;
  }

  const JAVA_MAJORS = [8, 17, 21];
  let managedPaths = $state<Record<string, string>>({});
  $effect(() => {
    api.resolvedJavaPaths().then((paths) => (managedPaths = paths)).catch(() => {});
  });
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
      setJavaPath(major, managedPaths[key] ?? "");
    }
  }

  async function browseInstancesDir() {
    const folder = await pickFolder(t("settings.chooseInstancesFolder"));
    if (folder) draft.instancesDir = folder;
  }

  // Sync the draft once settings finish loading. Only react to the `loaded`
  // transition — reading `settings` untracked so later mutations don't re-run
  // this effect and clobber in-progress draft edits.
  $effect(() => {
    if (settingsStore.loaded) {
      draft = { ...untrack(() => settingsStore.settings) };
    }
  });

  let javaBusy = $state(false);
  let javaLabel = $state("");
  let javaCur = $state(0);
  let javaTotal = $state(0);
  let javaInstalled = $state<string[]>([]);
  let javaError = $state<string | null>(null);

  const javaPct = $derived(toPct(javaCur, javaTotal));

  $effect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen<{ label: string; current: number; total: number }>(
      "java-setup",
      (event) => {
        javaLabel = event.payload.label;
        javaCur = event.payload.current;
        javaTotal = event.payload.total;
      }
    )
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  async function setupJava() {
    javaBusy = true;
    javaError = null;
    javaInstalled = [];
    javaLabel = t("settings.fetchingRuntimeList");
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
    <h1>{t("settings.title")}</h1>
    <div class="head-actions">
      {#if saved}<span class="saved">{t("settings.saved")}</span>{/if}
      <button class="btn primary" onclick={save} disabled={saving}>
        {saving ? t("settings.saving") : t("settings.saveChanges")}
      </button>
    </div>
  </header>

  <section class="card-block">
    <h3>{t("settings.account")}</h3>
    <div class="setting">
      <div class="label">
        <span>{t("settings.offlineUsername")}</span>
        <small>{t("settings.offlineUsernameDesc")}</small>
      </div>
      <input
        class="input narrow"
        placeholder={t("settings.offlineUsernamePlaceholder")}
        bind:value={draft.offlineUsername}
        autocomplete="off"
      />
    </div>
  </section>

  <section class="card-block">
    <h3>{t("settings.interface")}</h3>
    <div class="setting">
      <div class="label">
        <span>{t("settings.language")}</span>
        <small>{t("settings.languageDesc")}</small>
      </div>
      <LanguageSelect />
    </div>

    <div class="setting">
      <div class="label">
        <span>{t("settings.buttonSounds")}</span>
        <small>{t("settings.buttonSoundsDesc")}</small>
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
        <span>{t("settings.newsOnHome")}</span>
        <small>{t("settings.newsOnHomeDesc")}</small>
      </div>
      <label class="switch">
        <input
          type="checkbox"
          bind:checked={draft.showNews}
          onchange={() => settingsStore.save({ ...settingsStore.settings, showNews: draft.showNews })}
        />
        <span class="track"><span class="thumb"></span></span>
      </label>
    </div>

    {#if draft.showNews}
      <div class="setting">
        <div class="label">
          <span>{t("settings.oneStoryPerPage")}</span>
          <small>{t("settings.oneStoryPerPageDesc")}</small>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            bind:checked={draft.newsSingle}
            onchange={() => settingsStore.save({ ...settingsStore.settings, newsSingle: draft.newsSingle })}
          />
          <span class="track"><span class="thumb"></span></span>
        </label>
      </div>
    {/if}

    <div class="setting">
      <div class="label">
        <span>{t("settings.dateFormat")}</span>
        <small>{t("settings.dateFormatDesc", { sample: fmtDatePreview })}</small>
      </div>
      <select
        class="select"
        bind:value={draft.dateFormat}
        onchange={() => settingsStore.save({ ...settingsStore.settings, dateFormat: draft.dateFormat })}
      >
        <option value="system">{t("settings.dateSystem")}</option>
        <option value="iso">ISO (2026-07-20)</option>
        <option value="us">US (07/20/2026)</option>
        <option value="eu">EU (20/07/2026)</option>
      </select>
    </div>

    <div class="setting">
      <div class="label">
        <span>{t("settings.giphyKey")}</span>
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
        onblur={() => persistCredential("giphyApiKey")}
        onkeydown={(event) => event.key === "Enter" && event.currentTarget.blur()}
        autocomplete="off"
        spellcheck="false"
      />
    </div>

    <div class="setting">
      <div class="label">
        <span>{t("settings.dockPosition")}</span>
        <small>{t("settings.dockPositionDesc")}</small>
      </div>
      <div class="seg">
        {#each DOCK_POSITIONS as position}
          <button
            class="seg-btn"
            class:on={draft.dockPosition === position.value}
            onclick={() => (draft.dockPosition = position.value)}
          >
            {t(position.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    <div class="setting">
      <div class="label">
        <span>{t("settings.magnifyDock")}</span>
        <small>{t("settings.magnifyDockDesc")}</small>
      </div>
      <label class="switch">
        <input type="checkbox" bind:checked={draft.dockMagnify} />
        <span class="track"><span class="thumb"></span></span>
      </label>
    </div>
  </section>

  <section class="card-block">
    <h3>{t("settings.appearance")}</h3>

    <div class="label themes-label">
      <span>{t("settings.themePresets")}</span>
      <small>{t("settings.themePresetsDesc")}</small>
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
        <span>{t("settings.background")}</span>
        <small>{t("settings.backgroundDesc")}</small>
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
        {t("settings.bgDefault")}
      </button>
      <button
        class="chip"
        class:on={kind === "color"}
        onclick={() => setColor(colorValue)}
      >
        {t("settings.bgSolidColor")}
      </button>
      <button
        class="chip"
        class:on={kind === "pattern"}
        onclick={() => setPattern(activePattern || "dots")}
      >
        {t("settings.bgPattern")}
      </button>
      <button
        class="chip"
        class:on={kind === "image"}
        onclick={pickBgSticker}
      >
        {t("settings.bgImageGif")}
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
        <span class="bg-color-label">{t("settings.baseColor")}</span>
        {@render colorField(patternColor, setPatternColor)}
      </div>
    {:else if kind === "image"}
      <div class="bg-detail">
        <button class="btn ghost" onclick={() => bgFileInput?.click()}>
          <Icon name="edit" size={14} /> {t("settings.upload")}
        </button>
        <button class="btn ghost" onclick={pickBgSticker}>
          <Icon name="sparkles" size={14} /> {t("settings.stickersGifs")}
        </button>
      </div>
      <div class="bg-detail bg-color">
        <span class="bg-color-label">{t("settings.tint")}</span>
        {@render colorField(imageColor, setImageColor)}
      </div>
    {:else if kind === "texture"}
      <div class="bg-detail bg-color">
        <span class="bg-color-label">{t("settings.textureOpacity")}</span>
        <input
          type="range"
          class="opacity-range"
          min="0.1"
          max="1"
          step="0.05"
          value={textureOpacity}
          oninput={(event) => setTextureOpacity(parseFloat(event.currentTarget.value))}
          use:sliderFill={textureOpacity}
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
    <h3>{t("settings.javaMemory")}</h3>
    <div class="setting">
      <div class="label">
        <span>{t("settings.managedJava")}</span>
        <small>{t("settings.managedJavaDesc")}</small>
      </div>
      <button class="btn primary" onclick={setupJava} disabled={javaBusy}>
        {javaBusy ? t("settings.installing") : t("settings.autoInstallJava")}
      </button>
    </div>
    {#if javaBusy || javaInstalled.length > 0 || javaError}
      <div class="java-status">
        {#if javaBusy}
          <ProgressBar label={javaLabel || t("settings.preparing")} pct={javaPct} />
        {:else if javaError}
          <p class="java-err">{javaError}</p>
        {:else}
          <p class="java-ok">{t("settings.javaReady", { list: javaInstalled.join(", ") })}</p>
        {/if}
      </div>
    {/if}
    <div class="setting">
      <div class="label">
        <span>{t("settings.javaRuntimes")}</span>
        <small>{t("settings.javaRuntimesDesc")}</small>
      </div>
    </div>
    {#each JAVA_MAJORS as major (major)}
      <div class="setting java-major">
        <div class="label">
          <span>{t("settings.javaVersion", { major })}</span>
          <small>
            {javaManaged(major)
              ? managedPaths[String(major)]
                ? t("settings.managedByCactus")
                : t("settings.downloadedWhenNeeded")
              : t("settings.usingOwnJava")}
          </small>
        </div>
        <label class="switch" title={t("settings.manageAutomatically")}>
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
        placeholder={t("settings.javaPathPlaceholder")}
        value={javaManaged(major)
          ? (managedPaths[String(major)] ?? "")
          : (draft.javaPaths[String(major)] ?? "")}
        oninput={(event) => setJavaPath(major, event.currentTarget.value)}
        spellcheck="false"
      />
    {/each}
    <div class="setting">
      <div class="label">
        <span>{t("settings.maxMemory")}</span>
        <small>{draft.maxMemoryMb} MB</small>
      </div>
      <div class="mem">
        <input
          type="range"
          min={MEM_MIN}
          max={MEM_MAX}
          step="512"
          bind:value={draft.maxMemoryMb}
          class="range stepped"
          style="--steps:{(MEM_MAX - MEM_MIN) / 1024}"
          use:sliderFill={draft.maxMemoryMb}
        />
        <div class="mem-ticks">
          {#each memTicks as tick}
            <span class="tick {tick.align}" style="left:{tick.pct}%">{tick.gb}<small>GB</small></span>
          {/each}
        </div>
      </div>
    </div>
    <div class="setting">
      <div class="label">
        <span>{t("settings.jvmArguments")}</span>
        <small>{t("settings.jvmArgumentsDesc")}</small>
      </div>
      <input
        class="input narrow"
        placeholder="-XX:+UseG1GC …"
        bind:value={draft.jvmArgs}
      />
    </div>
    <div class="setting">
      <div class="label">
        <span>{t("settings.concurrentDownloads")}</span>
        <small>{t("settings.concurrentDownloadsDesc", { count: draft.maxConcurrentDownloads })}<em>{t("settings.slower")}</em>.</small>
      </div>
      <input
        type="range"
        min="1"
        max="64"
        step="1"
        bind:value={draft.maxConcurrentDownloads}
        class="range"
        use:sliderFill={draft.maxConcurrentDownloads}
      />
    </div>
  </section>

  <section class="card-block">
    <h3>{t("settings.gameWindow")}</h3>
    <div class="setting">
      <div class="label"><span>{t("settings.defaultResolution")}</span></div>
      <div class="res">
        <input type="number" class="input tiny" bind:value={draft.gameWidth} />
        <span>×</span>
        <input type="number" class="input tiny" bind:value={draft.gameHeight} />
        <button
          class="btn ghost"
          onclick={resetResolution}
          disabled={draft.gameWidth === 854 && draft.gameHeight === 480}
          title={t("settings.resetResolutionTitle")}
        >
          <Icon name="refresh" size={14} /> {t("settings.reset")}
        </button>
      </div>
    </div>
  </section>

  <section class="card-block">
    <h3>{t("settings.servers")}</h3>
    <div class="setting">
      <div class="label">
        <span>{t("settings.ngrokAuthtoken")}</span>
        <small>
          {t("settings.ngrokDesc1")}
          <button class="linkish" onclick={() => openUrl("https://dashboard.ngrok.com/get-started/your-authtoken")}>
            ngrok.com
          </button>{t("settings.ngrokDesc2")}
        </small>
      </div>
      <input
        class="input narrow"
        type="password"
        placeholder={t("settings.ngrokPlaceholder")}
        bind:value={draft.ngrokAuthtoken}
        onblur={() => persistCredential("ngrokAuthtoken")}
        onkeydown={(event) => event.key === "Enter" && event.currentTarget.blur()}
        autocomplete="off"
        spellcheck="false"
      />
    </div>
  </section>

  <section class="card-block">
    <h3>{t("settings.storage")}</h3>
    <div class="setting">
      <div class="label">
        <span>{t("settings.appDataFolder")}</span>
        <small>{t("settings.appDataFolderDesc")}</small>
        <small class="path">{dataDir || "…"}</small>
      </div>
      <div class="folder-actions">
        <button class="btn ghost" onclick={browseDataDir} disabled={anyRunning}>{t("settings.move")}</button>
        <button class="btn ghost" onclick={resetDataDir} disabled={anyRunning}>{t("settings.reset")}</button>
      </div>
    </div>
    {#if anyRunning}
      <p class="muted-note">{t("settings.closeRunningInstances")}</p>
    {/if}
    <div class="setting">
      <div class="label">
        <span>{t("settings.instancesFolder")}</span>
        <small>{t("settings.instancesFolderDesc")}</small>
        <small class="path">{draft.instancesDir || t("settings.defaultAppDataFolder")}</small>
      </div>
      <div class="folder-actions">
        <button class="btn ghost" onclick={browseInstancesDir}>{t("settings.browse")}</button>
        {#if draft.instancesDir}
          <button class="btn ghost" onclick={() => (draft.instancesDir = "")}>{t("settings.reset")}</button>
        {/if}
      </div>
    </div>
    <div class="setting">
      <div class="label">
        <span>{t("settings.sharedContentCache")}</span>
        <small>{t("settings.sharedContentCacheDesc")}</small>
      </div>
      <div class="folder-actions">
        <button class="btn ghost" onclick={clearCache} disabled={cacheLoading || clearingCache}>
          {clearingCache ? t("settings.clearing") : t("settings.clearCache")}
        </button>
        <button class="btn ghost" onclick={loadCache} disabled={cacheLoading}>
          {cacheLoading ? "…" : t("settings.refresh")}
        </button>
      </div>
    </div>
    {#if cacheStats}
      <div class="stats">
        <div class="stat">
          <span class="n">{cacheStats.files}</span>
          <span class="l">{t("settings.uniqueFiles")}</span>
        </div>
        <div class="stat">
          <span class="n">{formatBytes(cacheStats.bytes)}</span>
          <span class="l">{t("settings.onDisk")}</span>
        </div>
        <div class="stat save">
          <span class="n">{formatBytes(cacheStats.savedBytes)}</span>
          <span class="l">{t("settings.savedBySharing")}</span>
        </div>
      </div>
    {/if}

    <div class="setting danger-row">
      <div class="label">
        <span>{t("settings.resetEverything")}</span>
        <small>{t("settings.resetEverythingDesc")}</small>
      </div>
      <button class="btn danger" onclick={() => (resetOpen = true)}>
        <Icon name="trash" size={14} /> {t("settings.resetEllipsis")}
      </button>
    </div>
  </section>

  <section class="card-block">
    <h3>{t("settings.about")}</h3>
    <p class="about-app">{t("settings.aboutTagline")}</p>
    <div class="setting">
      <div class="label">
        <span>{t("settings.updates")}</span>
        <small>{appVersion ? t("settings.onVersion", { version: appVersion }) : t("settings.onCurrentVersion")}</small>
      </div>
      <button class="btn ghost sm" onclick={() => updater.check(true)} disabled={updater.checking}>
        <Icon name="refresh" size={14} /> {updater.checking ? t("settings.checking") : t("settings.checkForUpdates")}
      </button>
    </div>
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
      <button class="btn ghost sm" onclick={() => ui.openChangelog()}>
        <Icon name="clock" size={14} /> {t("settings.changelog")}
      </button>
    </div>
    <p class="about-credit">
      {t("settings.createdBy")}
      {#if LINKS.website}
        <button class="linkish" onclick={() => openUrl(LINKS.website)}>{LINKS.authorName}</button>
      {:else}
        <strong>{LINKS.authorName}</strong>
      {/if}
    </p>
  </section>

  <div class="save-bar">
    {#if saved}<span class="saved">{t("settings.saved")}</span>{/if}
    <button class="btn primary" onclick={save} disabled={saving}>
      {saving ? t("settings.saving") : t("settings.saveChanges")}
    </button>
  </div>

  <p class="app-version">Cactus Launcher{appVersion ? ` v${appVersion}` : ""}</p>
</div>

<Modal title={t("settings.moveDataTitle")} open={dataMoveOpen} onClose={() => (dataMoveOpen = false)} width={440}>
  <p class="reset-warn">
    {t("settings.moveDataWarn1")}
    <strong>{pendingDataDir ?? t("settings.theDefaultFolder")}</strong>
    {t("settings.moveDataWarn2")}
  </p>
  <div class="reset-actions">
    <button class="btn ghost" onclick={() => (dataMoveOpen = false)}>{t("common.cancel")}</button>
    <button class="btn primary" onclick={confirmMoveData} disabled={movingData}>
      {movingData ? t("settings.moving") : t("settings.moveAndReload")}
    </button>
  </div>
</Modal>

<Modal title={t("settings.resetEverythingTitle")} open={resetOpen} onClose={() => (resetOpen = false)} width={430}>
  <p class="reset-warn">
    {t("settings.resetWarn1")} <strong>{t("settings.resetWarnBold")}</strong>
    {t("settings.resetWarn2")}
  </p>
  <div class="reset-actions">
    <button class="btn ghost" onclick={() => (resetOpen = false)}>{t("common.cancel")}</button>
    <button class="btn danger" onclick={resetEverything} disabled={resetting}>
      {resetting ? t("settings.resetting") : t("settings.deleteEverything")}
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
    border-top: 1px solid var(--border-subtle);
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
  .app-version {
    margin: 20px 0 0;
    text-align: center;
    font-size: 11px;
    color: var(--text-muted);
  }
  .muted-note {
    margin: 0 0 4px;
    font-size: 12px;
    color: var(--text-muted);
  }
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
  }
  .mem {
    width: 240px;
  }
  .mem .range {
    width: 100%;
  }
  .mem-ticks {
    position: relative;
    height: 14px;
    margin-top: 4px;
  }
  .mem-ticks .tick {
    position: absolute;
    transform: translateX(-50%);
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .mem-ticks .tick small {
    font-size: 8px;
    opacity: 0.7;
    margin-left: 1px;
  }
  .mem-ticks .tick.start {
    transform: none;
  }
  .mem-ticks .tick.end {
    transform: translateX(-100%);
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
