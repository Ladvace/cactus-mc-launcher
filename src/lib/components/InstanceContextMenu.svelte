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
  import { copyText } from "$lib/clipboard";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { t } from "$lib/i18n";

  const shareOnline = boardApi.configured();

  let fileInput = $state<HTMLInputElement>();
  let pendingId = $state<string | null>(null);
  let sharing = $state(false);
  let sharedCode = $state<string | null>(null);
  // Opt-out CurseForge mods block code sharing.
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
        label: running ? t("instanceMenu.stop") : busy ? t("instanceMenu.preparing") : t("instanceMenu.play"),
        icon: running ? "stop" : "play",
        disabled: busy && !running,
        onSelect: () =>
          running ? launchStore.stop(inst.id) : launchStore.launch(inst.id),
      },
      { label: t("instanceMenu.open"), icon: "folder", onSelect: () => goto(`/instance/${inst.id}`) },
      { label: t("instanceMenu.openFolder"), icon: "archive", onSelect: () => openFolder(inst.id) },
      ...(inst.kind !== "server"
        ? [
            {
              label: t("instanceMenu.createServer"),
              icon: "globe",
              onSelect: () => createServer(inst.id),
            },
          ]
        : []),
      ...(inst.kind === "server"
        ? [
            {
              label: t("instanceMenu.copyServerAddress"),
              icon: "copy",
              onSelect: () => copyServerAddress(inst.id),
            },
          ]
        : []),
      {
        label: inst.group ? t("instanceMenu.groupLabel", { group: inst.group }) : t("instanceMenu.moveToGroup"),
        icon: "folder",
        onSelect: () => ui.openGroupPicker(inst),
      },
      { separator: true },
      { label: t("instanceMenu.uploadImage"), icon: "edit", onSelect: () => pickFile(inst.id) },
      {
        label: t("instanceMenu.stickersEmoji"),
        icon: "sparkles",
        onSelect: () =>
          ui.openStickerPicker(t("instanceMenu.imageFor", { name: inst.name }), (uri) =>
            instancesStore.setIcon(inst.id, uri)
          ),
      },
      {
        label: inst.coverImage ? t("instanceMenu.shrinkImage") : t("instanceMenu.fillTile"),
        icon: "expand",
        disabled: !inst.icon,
        onSelect: () =>
          instancesStore
            .update(inst.id, { coverImage: !inst.coverImage })
            .catch((e) => toast.error(String(e))),
      },
      {
        label: t("instanceMenu.resetImage"),
        icon: "refresh",
        disabled: !inst.icon,
        onSelect: () =>
          instancesStore.resetIcon(inst.id).catch((e) => toast.error(String(e))),
      },
      { separator: true },
      ...(shareOnline
        ? [
            {
              label:
                shareChecks[inst.id] && !shareChecks[inst.id].ok
                  ? t("instanceMenu.cantShare")
                  : t("instanceMenu.shareViaCode"),
              icon: "share",
              disabled: !!(shareChecks[inst.id] && !shareChecks[inst.id].ok),
              onSelect: () => shareViaCode(inst.id, inst.name),
            },
          ]
        : []),
      {
        label: t("instanceMenu.exportToFile"),
        icon: "download",
        onSelect: () => exportSetup(inst.id, "cactuspack"),
      },
      {
        label: t("instanceMenu.exportMrpack"),
        icon: "upload",
        onSelect: () => exportSetup(inst.id, "mrpack"),
      },
    ];
  });

  async function shareViaCode(id: string, name: string) {
    if (!shareOnline) {
      toast.error(t("instanceMenu.boardsNotSetUp"));
      return;
    }
    if (!boardAuth.signedIn && !accountsStore.active) {
      toast.error(t("instanceMenu.addAccountFirst"));
      return;
    }
    sharing = true;
    try {
      const check = await api.instanceShareCheck(id);
      shareChecks = { ...shareChecks, [id]: check };
      if (!check.ok) {
        toast.error(t("instanceMenu.cantShareMods", { mods: check.optOut.join(", ") }));
        return;
      }
      if (!boardAuth.signedIn) await boardAuth.login();
      const token = boardAuth.token;
      if (!token) {
        toast.error(boardAuth.error ?? t("instanceMenu.couldntSignIn"));
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

  function copyCode() {
    if (sharedCode) copyText(sharedCode, t("instanceMenu.copied"));
  }

  async function createServer(id: string) {
    try {
      const server = await api.createServerFrom(id);
      await instancesStore.refresh();
      toast.success(t("instanceMenu.serverCreated"));
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
      await copyText(addr, t("instanceMenu.copiedAddr", { addr }));
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
      const result = await api.exportSetup(id, format);
      toast.success(
        result.skipped.length
          ? t("instanceMenu.exportedSkipped", { count: result.skipped.length })
          : t("instanceMenu.setupExported")
      );
      try {
        await revealItemInDir(result.path);
      } catch {
      }
    } catch (error) {
      toast.error(String(error));
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
    } catch (error) {
      toast.error(String(error));
      console.error("icon upload failed", error);
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
  <div class="toast ok" role="status">{t("instanceMenu.creatingShareCode")}</div>
{/if}

<Modal
  title={t("instanceMenu.shareTitle")}
  open={!!sharedCode}
  onClose={() => (sharedCode = null)}
  width={360}
>
  <p class="share-hint">
    {t("instanceMenu.shareHintBefore")} <strong>{t("instanceMenu.shareHintLocation")}</strong>:
  </p>
  <div class="codebox">
    <span class="code">{sharedCode}</span>
    <button class="btn ghost" onclick={copyCode}>{t("instanceMenu.copy")}</button>
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
