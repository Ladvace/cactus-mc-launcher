<script lang="ts">
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { groupCovers } from "$lib/stores/groupCovers.svelte";
  import { fileToIconDataUri } from "$lib/image";
  import { toast } from "$lib/stores/toast.svelte";

  let { onOpenFolder }: { onOpenFolder: (name: string) => void } = $props();

  const menu = $derived(ui.groupMenu);

  let fileInput = $state<HTMLInputElement>();
  let pendingName = $state<string | null>(null);

  const items = $derived.by<MenuItem[]>(() => {
    const m = ui.groupMenu;
    if (!m) return [];
    const name = m.name;
    const hasCover = !!groupCovers.get(name);
    return [
      { label: "Open folder", icon: "folder", onSelect: () => onOpenFolder(name) },
      { separator: true },
      { label: "Upload cover…", icon: "edit", onSelect: () => pickFile(name) },
      {
        label: "Stickers & emoji…",
        icon: "sparkles",
        onSelect: () =>
          ui.openStickerPicker(`Cover for ${name}`, (uri) => groupCovers.set(name, uri)),
      },
      ...(hasCover
        ? [{ label: "Remove cover", icon: "refresh", onSelect: () => groupCovers.clear(name) }]
        : []),
      { separator: true },
      { label: "Ungroup all", icon: "trash", onSelect: () => ungroupAll(name) },
    ];
  });

  function pickFile(name: string) {
    pendingName = name;
    fileInput?.click();
  }

  async function onFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
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
    const members = instancesStore.instances.filter((i) => i.group === name);
    for (const i of members) await instancesStore.update(i.id, { group: "" });
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
