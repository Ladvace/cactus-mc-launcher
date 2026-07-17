<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import Icon from "./Icon.svelte";

  const { Story } = defineMeta({
    title: "Components/Icon",
    component: Icon,
    tags: ["autodocs"],
    argTypes: {
      name: {
        control: "select",
        options: [
          "home", "compass", "users", "globe", "play", "stop", "plus",
          "trash", "folder", "edit", "settings", "search", "copy",
          "sparkles", "refresh", "download", "upload", "share", "check",
        ],
      },
      size: { control: { type: "range", min: 12, max: 64, step: 2 } },
    },
    args: { name: "play", size: 24 },
  });
</script>

<Story name="Play" args={{ name: "play" }} />
<Story name="Folder" args={{ name: "folder", size: 32 }} />
<Story name="Trash" args={{ name: "trash" }} />
<Story name="Large sparkles" args={{ name: "sparkles", size: 48 }} />
