<script lang="ts">
  import "../app.css";
  import "@hackernoon/pixel-icon-library/fonts/iconfont.css";
  import Dock from "$lib/components/Dock.svelte";
  import CreateInstanceModal from "$lib/components/CreateInstanceModal.svelte";
  import AccountModal from "$lib/components/AccountModal.svelte";
  import InstanceContextMenu from "$lib/components/InstanceContextMenu.svelte";
  import StickerPicker from "$lib/components/StickerPicker.svelte";
  import GroupPicker from "$lib/components/GroupPicker.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { backgroundCss } from "$lib/background";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  const bg = $derived(backgroundCss(settingsStore.settings.background ?? ""));

  // Load core data and subscribe to events once on startup.
  $effect(() => {
    instancesStore.ensureLoaded();
    settingsStore.ensureLoaded();
    launchStore.init();
    accountsStore.init();
  });

  // Suppress the OS/browser right-click menu everywhere (it looks out of place
  // in a native app) — except in text fields, where paste is useful. Our own
  // context menus call preventDefault + open regardless.
  function onContextMenu(e: MouseEvent) {
    const t = e.target as HTMLElement | null;
    const tag = t?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || t?.isContentEditable) return;
    e.preventDefault();
  }
</script>

<svelte:window oncontextmenu={onContextMenu} />

<div class="app">
  <div class="bg-layer" style="background: {bg};"></div>
  <main class="content">
    {@render children()}
  </main>
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

<style>
  .app {
    position: relative;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }
  /* Fixed layer behind everything; carries the (customizable) app background. */
  .bg-layer {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
  }
  .content {
    position: relative;
    z-index: 1;
    height: 100vh;
    width: 100%;
    overflow-y: auto;
    background: transparent;
    /* leave room for the floating dock */
    padding-bottom: 90px;
  }
</style>
