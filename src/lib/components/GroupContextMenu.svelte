<script lang="ts">
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { groupCovers } from "$lib/stores/groupCovers.svelte";
  import { fileToIconDataUri } from "$lib/image";
  import { toast } from "$lib/stores/toast.svelte";
  import { t } from "$lib/i18n";

  let { onOpenFolder }: { onOpenFolder: (name: string) => void } = $props();

  const menu = $derived(ui.groupMenu);

  let fileInput = $state<HTMLInputElement>();
  let pendingName = $state<string | null>(null);

  const items = $derived.by<MenuItem[]>(() => {
    const groupMenu = ui.groupMenu;
    if (!groupMenu) return [];
    const name = groupMenu.name;
    const hasCover = !!groupCovers.get(name);
    return [
      { label: t("group.openFolder"), icon: "folder", onSelect: () => onOpenFolder(name) },
      { separator: true },
      { label: t("group.uploadCover"), icon: "edit", onSelect: () => pickFile(name) },
      {
        label: t("group.stickersEmoji"),
        icon: "sparkles",
        onSelect: () =>
          ui.openStickerPicker(t("group.coverFor", { name }), (uri) => groupCovers.set(name, uri)),
      },
      ...(hasCover
        ? [{ label: t("group.removeCover"), icon: "refresh", onSelect: () => groupCovers.clear(name) }]
        : []),
      { separator: true },
      { label: t("group.ungroupAll"), icon: "trash", onSelect: () => ungroupAll(name) },
    ];
  });

  function pickFile(name: string) {
    pendingName = name;
    fileInput?.click();
  }

  async function onFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    const name = pendingName;
    pendingName = null;
    if (!file || !name) return;
    try {
      groupCovers.set(name, await fileToIconDataUri(file));
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function ungroupAll(name: string) {
    const members = instancesStore.instances.filter((instance) => instance.group === name);
    for (const instance of members) await instancesStore.update(instance.id, { group: "" });
    groupCovers.clear(name);
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
  {#key `${menu.name}:${menu.x},${menu.y}`}
    <ContextMenu x={menu.x} y={menu.y} {items} onClose={() => ui.closeGroupMenu()} />
  {/key}
{/if}
