<script lang="ts">
  import { goto } from "$app/navigation";
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import Modal from "./Modal.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { boardApi } from "$lib/boardApi";
  import { api } from "$lib/api";
  import { fileToIconDataUri } from "$lib/image";
  import { toast } from "$lib/stores/toast.svelte";
  import { localServerAddress } from "$lib/serverAddress";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  const shareOnline = boardApi.configured();

  let fileInput = $state<HTMLInputElement>();
  let pendingId = $state<string | null>(null);
  let sharing = $state(false);
  let sharedCode = $state<string | null>(null);
  // Cache per-instance shareability (opt-out CurseForge mods block code sharing).
  let shareChecks = $state<Record<string, { ok: boolean; optOut: string[] }>>({});

  $effect(() => {
    const activeMenu = ui.instanceMenu;
    if (!activeMenu || !shareOnline || shareChecks[activeMenu.instance.id]) return;
    api
      .instanceShareCheck(activeMenu.instance.id)
      .then((result) => (shareChecks = { ...shareChecks, [activeMenu.instance.id]: result }))
      .catch(() => {});
  });

  const menu = $derived(ui.instanceMenu);

  const items = $derived.by<MenuItem[]>(() => {
    const activeMenu = ui.instanceMenu;
    if (!activeMenu) return [];
    const inst = activeMenu.instance;
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
      { label: "Open folder", icon: "archive", onSelect: () => openFolder(inst.id) },
      ...(inst.kind !== "server"
        ? [
            {
              label: "Create server",
              icon: "globe",
              onSelect: () => createServer(inst.id),
            },
          ]
        : []),
      ...(inst.kind === "server"
        ? [
            {
              label: "Copy server address",
              icon: "copy",
              onSelect: () => copyServerAddress(inst.id),
            },
          ]
        : []),
      {
        label: inst.group ? `Group: ${inst.group}…` : "Move to group…",
        icon: "folder",
        onSelect: () => ui.openGroupPicker(inst),
      },
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
      ...(shareOnline
        ? [
            {
              label:
                shareChecks[inst.id] && !shareChecks[inst.id].ok
                  ? "Can't share (opt-out mods)"
                  : "Share via code…",
              icon: "share",
              disabled: !!(shareChecks[inst.id] && !shareChecks[inst.id].ok),
              onSelect: () => shareViaCode(inst.id, inst.name),
            },
          ]
        : []),
      {
        label: "Export to file…",
        icon: "download",
        onSelect: () => exportSetup(inst.id, "cactuspack"),
      },
      {
        label: "Export as .mrpack…",
        icon: "upload",
        onSelect: () => exportSetup(inst.id, "mrpack"),
      },
    ];
  });

  async function shareViaCode(id: string, name: string) {
    if (!shareOnline) {
      toast.error("The boards service isn't set up in this build.");
      return;
    }
    if (!boardAuth.signedIn && !accountsStore.active) {
      toast.error("Add a Microsoft account first.");
      return;
    }
    sharing = true;
    try {
      const check = await api.instanceShareCheck(id);
      shareChecks = { ...shareChecks, [id]: check };
      if (!check.ok) {
        toast.error(`Can't share — these mods can't be re-downloaded: ${check.optOut.join(", ")}`);
        return;
      }
      if (!boardAuth.signedIn) await boardAuth.login();
      const token = boardAuth.token;
      if (!token) {
        toast.error(boardAuth.error ?? "Couldn't sign in.");
        return;
      }
      const snapshotId = await boardApi.publish(id, "cactuspack", token, { name });
      const { code } = await boardApi.mintCode(token, snapshotId);
      sharedCode = code;
    } catch (error) {
      toast.error(String(error));
    } finally {
      sharing = false;
    }
  }

  async function copyCode() {
    if (!sharedCode) return;
    try {
      await navigator.clipboard.writeText(sharedCode);
      toast.success("Copied!");
    } catch {
      /* clipboard may be unavailable */
    }
  }

  async function createServer(id: string) {
    try {
      const server = await api.createServerFrom(id);
      await instancesStore.refresh();
      toast.success("Server instance created.");
      goto(`/instance/${server.id}`);
    } catch (error) {
      toast.error(String(error));
    }
  }

  async function openFolder(id: string) {
    try {
      await revealItemInDir(await api.instanceFolder(id));
    } catch (error) {
      toast.error(String(error));
    }
  }

  async function copyServerAddress(id: string) {
    try {
      const addr = await localServerAddress(id);
      await navigator.clipboard.writeText(addr);
      toast.success(`Copied ${addr}`);
    } catch (error) {
      toast.error(String(error));
    }
  }

  function pickFile(id: string) {
    pendingId = id;
    fileInput?.click();
  }

  async function exportSetup(id: string, format: "cactuspack" | "mrpack") {
    try {
      const res = await api.exportSetup(id, format);
      toast.success(
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
      toast.error(String(err));
    }
  }

  async function onFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = ""; // allow re-picking the same file later
    if (!file || !pendingId) return;
    const id = pendingId;
    pendingId = null;
    try {
      const uri = await fileToIconDataUri(file);
      await instancesStore.setIcon(id, uri);
    } catch (err) {
      toast.error(String(err));
      console.error("icon upload failed", err);
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

{#if sharing}
  <div class="toast ok" role="status">Creating a share code…</div>
{/if}

<Modal
  title="Share this instance"
  open={!!sharedCode}
  onClose={() => (sharedCode = null)}
  width={360}
>
  <p class="share-hint">
    Anyone can import it from <strong>Home → Import → “from a code”</strong>:
  </p>
  <div class="codebox">
    <span class="code">{sharedCode}</span>
    <button class="btn ghost" onclick={copyCode}>Copy</button>
  </div>
</Modal>

<style>
  .share-hint {
    margin: 0 0 12px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .codebox {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .code {
    flex: 1;
    font-family: var(--font-pixel);
    font-size: 24px;
    letter-spacing: 0.08em;
    text-align: center;
    color: var(--accent);
    background: var(--bg-input);
    border: 2px solid var(--border);
    padding: 10px;
    user-select: all;
  }
  .toast {
    position: fixed;
    bottom: 100px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 300;
    padding: 10px 16px;
    background: var(--bg-raised);
    border: 2px solid var(--accent);
    color: var(--accent);
    font-size: 13px;
    box-shadow: var(--shadow-md);
  }
</style>
