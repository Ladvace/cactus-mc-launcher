<script lang="ts">
  import "../app.css";
  import "@hackernoon/pixel-icon-library/fonts/iconfont.css";
  import Dock from "$lib/components/Dock.svelte";
  import CreateInstanceModal from "$lib/components/CreateInstanceModal.svelte";
  import AccountModal from "$lib/components/AccountModal.svelte";
  import InstanceContextMenu from "$lib/components/InstanceContextMenu.svelte";
  import StickerPicker from "$lib/components/StickerPicker.svelte";
  import GroupPicker from "$lib/components/GroupPicker.svelte";
  import FolderOverlay from "$lib/components/FolderOverlay.svelte";
  import DecorLayer from "$lib/components/DecorLayer.svelte";
  import ReactiveGrid from "$lib/components/ReactiveGrid.svelte";
  import NoiseGrid from "$lib/components/NoiseGrid.svelte";
  import Toaster from "$lib/components/Toaster.svelte";
  import UpdatePrompt from "$lib/components/UpdatePrompt.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import ChangelogModal from "$lib/components/ChangelogModal.svelte";
  import CraftingTable from "$lib/components/CraftingTable.svelte";
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
  import { invoke } from "@tauri-apps/api/core";
  import { backgroundCss, animClass } from "$lib/background";
  import { readJson } from "$lib/storage";
  import { playClick } from "$lib/sound";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  let onboarded = $state(readJson<boolean>("cactus:onboarded", false));

  const bgSetting = $derived(settingsStore.settings.background ?? "");
  const bg = $derived(backgroundCss(bgSetting));
  const bgAnim = $derived(animClass(bgSetting));

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

  $effect(() => {
    const locale = currentLocale();
    document.documentElement.lang = locale;
    document.documentElement.dir = RTL_LOCALES.includes(locale) ? "rtl" : "ltr";
  });

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
    if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === "i") {
      event.preventDefault();
      invoke("toggle_devtools").catch(() => {});
    }
  }

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
  <div class="bg-layer {bgAnim}" style={bgAnim ? "" : `background: ${bg};`}></div>
  {#if bgSetting === "anim:circuit" && !settingsStore.settings.reduceMotion}
    <ReactiveGrid />
  {/if}
  {#if bgSetting === "anim:lattice" && !settingsStore.settings.reduceMotion}
    <NoiseGrid />
  {/if}
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
<FolderOverlay name={ui.folderOpen} onClose={() => ui.closeFolder()} />
<Toaster />
<UpdatePrompt />
<CommandPalette />
<ChangelogModal open={ui.changelogOpen} onClose={() => ui.closeChangelog()} />
<CraftingTable open={ui.craftingOpen} onClose={() => ui.closeCrafting()} />

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
    position: relative;
    height: 100vh;
    width: 100%;
    overflow-y: auto;
    scrollbar-gutter: stable;
    background: transparent;
  }
</style>
