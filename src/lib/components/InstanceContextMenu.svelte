<script lang="ts">
  import { goto } from "$app/navigation";
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { api } from "$lib/api";
  import { fileToIconDataUri } from "$lib/image";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  let fileInput = $state<HTMLInputElement>();
  let pendingId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let toast = $state<string | null>(null);

  function flash(msg: string) {
    toast = msg;
    setTimeout(() => (toast = null), 4000);
  }

  const menu = $derived(ui.instanceMenu);

  const items = $derived.by<MenuItem[]>(() => {
    const m = ui.instanceMenu;
    if (!m) return [];
    const inst = m.instance;
    const running = launchStore.isRunning(inst.id);
    const busy = launchStore.isBusy(inst.id);
    return [
      {
        label: running ? "Stop" : busy ? "Preparing…" : "Play",
        icon: running ? "stop" : "play",
        disabled: busy && !running,
        onSelect: () =>
          running ? launchStore.stop(inst.id) : launchStore.launch(inst.id),
      },
      { label: "Open", icon: "folder", onSelect: () => goto(`/instance/${inst.id}`) },
      { separator: true },
      { label: "Upload image…", icon: "edit", onSelect: () => pickFile(inst.id) },
      {
        label: "Stickers & emoji…",
        icon: "sparkles",
        onSelect: () =>
          ui.openStickerPicker(`Image for ${inst.name}`, (uri) =>
            instancesStore.setIcon(inst.id, uri)
          ),
      },
      {
        label: inst.coverImage ? "Shrink image to icon" : "Fill tile with image",
        icon: "expand",
        disabled: !inst.icon,
        onSelect: () =>
          instancesStore.update(inst.id, { coverImage: !inst.coverImage }),
      },
      {
        label: "Reset image",
        icon: "refresh",
        disabled: !inst.icon,
        onSelect: () => instancesStore.resetIcon(inst.id),
      },
      { separator: true },
      {
        label: "Export setup…",
        icon: "share",
        onSelect: () => exportSetup(inst.id, "drakepack"),
      },
      {
        label: "Export as .mrpack…",
        icon: "upload",
        onSelect: () => exportSetup(inst.id, "mrpack"),
      },
    ];
  });

  function pickFile(id: string) {
    pendingId = id;
    error = null;
    fileInput?.click();
  }

  async function exportSetup(id: string, format: "drakepack" | "mrpack") {
    try {
      const res = await api.exportSetup(id, format);
      flash(
        res.skipped.length
          ? `Exported — ${res.skipped.length} non-Modrinth item(s) skipped.`
          : "Setup exported."
      );
      try {
        await revealItemInDir(res.path);
      } catch {
        /* reveal is best-effort */
      }
    } catch (err) {
      error = String(err);
      setTimeout(() => (error = null), 4000);
    }
  }

  async function onFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = ""; // allow re-picking the same file later
    if (!file || !pendingId) return;
    const id = pendingId;
    pendingId = null;
    try {
      const uri = await fileToIconDataUri(file);
      await instancesStore.setIcon(id, uri);
    } catch (err) {
      error = String(err);
      console.error("icon upload failed", err);
      setTimeout(() => (error = null), 4000);
    }
  }
</script>

<input
  bind:this={fileInput}
  type="file"
  accept="image/png,image/jpeg,image/gif,image/webp"
  style="display:none"
  onchange={onFile}
/>

{#if menu}
  {#key `${menu.instance.id}:${menu.x},${menu.y}`}
    <ContextMenu
      x={menu.x}
      y={menu.y}
      {items}
      onClose={() => ui.closeInstanceMenu()}
    />
  {/key}
{/if}

{#if error}
  <div class="toast" role="alert">{error}</div>
{:else if toast}
  <div class="toast ok" role="status">{toast}</div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 100px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 300;
    padding: 10px 16px;
    background: var(--bg-raised);
    border: 2px solid var(--danger);
    color: var(--danger);
    font-size: 13px;
    box-shadow: var(--shadow-md);
  }
  .toast.ok {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
