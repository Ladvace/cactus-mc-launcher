<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import ContextMenu from "./ContextMenu.svelte";

  const items = [
    { label: "Play", icon: "play" },
    { label: "Open folder", icon: "folder" },
    { separator: true },
    { label: "Export setup", icon: "upload" },
    { label: "Move to group", icon: "folder", disabled: true },
    { separator: true },
    { label: "Delete", icon: "trash", danger: true },
  ];

  const { Story } = defineMeta({
    title: "Components/ContextMenu",
    component: ContextMenu,
    tags: ["autodocs"],
    args: { x: 40, y: 24, items, onClose: () => {} },
  });
</script>

<Story name="Default" />
