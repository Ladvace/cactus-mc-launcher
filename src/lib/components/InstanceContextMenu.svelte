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
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  const shareOnline = boardApi.configured();

  let fileInput = $state<HTMLInputElement>();
  let pendingId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let toast = $state<string | null>(null);
  let sharing = $state(false);
  let sharedCode = $state<string | null>(null);
  // Cache per-instance shareability (opt-out CurseForge mods block code sharing).
  let shareChecks = $state<Record<string, { ok: boolean; optOut: string[] }>>({});

  // When a menu opens, check whether the instance can be shared by code.
  $effect(() => {
    const m = ui.instanceMenu;
    if (!m || !shareOnline || shareChecks[m.instance.id]) return;
    api
      .instanceShareCheck(m.instance.id)
      .then((r) => (shareChecks = { ...shareChecks, [m.instance.id]: r }))
      .catch(() => {});
  });

  function flash(msg: string) {
    toast = msg;
    setTimeout(() => (toast = null), 4000);
  }
  function flashErr(msg: string) {
    error = msg;
    setTimeout(() => (error = null), 4000);
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
        onSelect: () => exportSetup(inst.id, "drakepack"),
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
      flashErr("The boards service isn't set up in this build.");
      return;
    }
    if (!boardAuth.signedIn && !accountsStore.active) {
      flashErr("Add a Microsoft account first.");
      return;
    }
    sharing = true;
    try {
      const check = await api.instanceShareCheck(id);
      shareChecks = { ...shareChecks, [id]: check };
      if (!check.ok) {
        flashErr(`Can't share — these mods can't be re-downloaded: ${check.optOut.join(", ")}`);
        return;
      }
      if (!boardAuth.signedIn) await boardAuth.login();
      const token = boardAuth.token;
      if (!token) {
        flashErr(boardAuth.error ?? "Couldn't sign in.");
        return;
      }
      const snapshotId = await boardApi.publish(id, "drakepack", token, { name });
      const { code } = await boardApi.mintCode(token, snapshotId);
      sharedCode = code;
    } catch (e) {
      flashErr(String(e));
    } finally {
      sharing = false;
    }
  }

  async function copyCode() {
    if (!sharedCode) return;
    try {
      await navigator.clipboard.writeText(sharedCode);
      flash("Copied!");
    } catch {
      /* clipboard may be unavailable */
    }
  }

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

{#if sharing}
  <div class="toast ok" role="status">Creating a share code…</div>
{:else if error}
  <div class="toast" role="alert">{error}</div>
{:else if toast}
  <div class="toast ok" role="status">{toast}</div>
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
