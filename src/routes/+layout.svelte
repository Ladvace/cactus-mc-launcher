<script lang="ts">
  import "../app.css";
  import "@hackernoon/pixel-icon-library/fonts/iconfont.css";
  import Dock from "$lib/components/Dock.svelte";
  import CreateInstanceModal from "$lib/components/CreateInstanceModal.svelte";
  import AccountModal from "$lib/components/AccountModal.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  // Load core data and subscribe to events once on startup.
  $effect(() => {
    instancesStore.ensureLoaded();
    settingsStore.ensureLoaded();
    launchStore.init();
    accountsStore.init();
  });
</script>

<div class="app">
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

<style>
  .app {
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }
  .content {
    height: 100vh;
    width: 100%;
    overflow-y: auto;
    background: var(--bg-app);
    /* leave room for the floating dock */
    padding-bottom: 90px;
  }
</style>
