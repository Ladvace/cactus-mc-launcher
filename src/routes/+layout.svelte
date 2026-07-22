<script lang="ts">
  import "../app.css";
  import "@hackernoon/pixel-icon-library/fonts/iconfont.css";
  import Dock from "$lib/components/Dock.svelte";
  import CreateInstanceModal from "$lib/components/CreateInstanceModal.svelte";
  import AccountModal from "$lib/components/AccountModal.svelte";
  import InstanceContextMenu from "$lib/components/InstanceContextMenu.svelte";
  import StickerPicker from "$lib/components/StickerPicker.svelte";
  import GroupPicker from "$lib/components/GroupPicker.svelte";
  import DecorLayer from "$lib/components/DecorLayer.svelte";
  import Toaster from "$lib/components/Toaster.svelte";
  import UpdatePrompt from "$lib/components/UpdatePrompt.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import ChangelogModal from "$lib/components/ChangelogModal.svelte";
  import Onboarding from "$lib/components/Onboarding.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { installStore } from "$lib/stores/install.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { initBoardApi } from "$lib/boardApi";
  import { ui } from "$lib/stores/ui.svelte";
  import { currentLocale, RTL_LOCALES } from "$lib/i18n";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { backgroundCss } from "$lib/background";
  import { readJson } from "$lib/storage";
  import { playClick } from "$lib/sound";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  let onboarded = $state(readJson<boolean>("cactus:onboarded", false));

  const bg = $derived(backgroundCss(settingsStore.settings.background ?? ""));

  const dockPos = $derived(settingsStore.settings.dockPosition ?? "bottom");
  const dockPad = $derived(
    ({ bottom: "padding-bottom", top: "padding-top", left: "padding-left", right: "padding-right" } as const)[
      dockPos as "bottom" | "top" | "left" | "right"
    ] ?? "padding-bottom"
  );

  let splash = $state(true);

  $effect(() => {
    instancesStore.ensureLoaded();
    settingsStore.ensureLoaded();
    launchStore.init();
    installStore.init();
    accountsStore.init();
    initBoardApi();
  });

  // Keep the document language + direction in sync with the chosen UI locale.
  $effect(() => {
    const locale = currentLocale();
    document.documentElement.lang = locale;
    document.documentElement.dir = RTL_LOCALES.includes(locale) ? "rtl" : "ltr";
  });

  // Apply the accessibility / customization settings to the document root; the
  // matching CSS lives in app.css.
  $effect(() => {
    const s = settingsStore.settings;
    const root = document.documentElement;
    root.dataset.reduceMotion = String(!!s.reduceMotion);
    root.dataset.readableFont = String(!!s.readableFont);
    root.dataset.highContrast = String(!!s.highContrast);
    root.dataset.reduceTransparency = String(!!s.reduceTransparency);
    root.dataset.focus = String(!!s.alwaysShowFocus);
    root.dataset.accent = s.accent || "";
  });

  // Whole-UI zoom via the real webview zoom (handles fixed layout correctly).
  $effect(() => {
    const scale = (settingsStore.settings.uiScale ?? 100) / 100;
    getCurrentWebview().setZoom(scale).catch(() => {});
  });

  function onContextMenu(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    const tag = target?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || target?.isContentEditable) return;
    event.preventDefault();
  }

  function onGlobalKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      ui.toggleCommandPalette();
    }
  }

  // Capture phase so it fires even when a handler stops propagation.
  function onGlobalClick(event: MouseEvent) {
    if (!settingsStore.settings.uiSounds) return;
    const target = event.target as HTMLElement | null;
    const button = target?.closest?.("button, .btn") as HTMLElement | null;
    if (!button) return;
    if ((button as HTMLButtonElement).disabled) return;
    if (button.getAttribute("aria-disabled") === "true") return;
    playClick();
  }
</script>

<svelte:window oncontextmenu={onContextMenu} onclickcapture={onGlobalClick} onkeydown={onGlobalKeydown} />

<div class="app">
  <div class="bg-layer" style="background: {bg};"></div>
  <main class="content" style="{dockPad}: 90px;">
    {@render children()}
  </main>
  <DecorLayer />
  <Dock onCreate={() => ui.openCreateInstance()} />
</div>

<CreateInstanceModal
  open={ui.createInstanceOpen}
  onClose={() => ui.closeCreateInstance()}
/>

<AccountModal open={ui.accountsOpen} onClose={() => ui.closeAccounts()} />

<InstanceContextMenu />
<StickerPicker />
<GroupPicker />
<Toaster />
<UpdatePrompt />
<CommandPalette />
<ChangelogModal open={ui.changelogOpen} onClose={() => ui.closeChangelog()} />

{#if !onboarded && settingsStore.loaded}
  <Onboarding onDone={() => (onboarded = true)} />
{/if}

{#if splash}
  <div
    class="splash"
    onanimationend={(event) => {
      if (event.animationName === "splash-fade") splash = false;
    }}
  >
    <img src="/cactus-logo.png" alt="Cactus Launcher" class="splash-logo" />
  </div>
{/if}

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-app);
    animation: splash-fade 1.8s ease forwards;
  }
  @keyframes splash-fade {
    0%,
    68% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      visibility: hidden;
    }
  }
  .splash-logo {
    width: min(60%, 380px);
    height: auto;
    image-rendering: pixelated;
    animation: splash-pop 0.5s ease both;
  }
  @keyframes splash-pop {
    from {
      transform: scale(0.9);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
  .app {
    position: relative;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }
  .bg-layer {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
  }
  .content {
    /* No z-index here on purpose: a stacking context would trap fixed overlays
       rendered by pages (modals, context menus, the instance picker) *below*
       the Dock. Tree order already paints .content above the fixed .bg-layer. */
    position: relative;
    height: 100vh;
    width: 100%;
    overflow-y: auto;
    /* Always reserve the scrollbar gutter so switching between scrolling and
       non-scrolling views doesn't nudge the layout sideways. */
    scrollbar-gutter: stable;
    background: transparent;
  }
</style>
