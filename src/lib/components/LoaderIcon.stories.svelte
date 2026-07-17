<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import LoaderIcon from "./LoaderIcon.svelte";

  const { Story } = defineMeta({
    title: "Components/LoaderIcon",
    component: LoaderIcon,
    tags: ["autodocs"],
    argTypes: {
      loader: {
        control: "select",
        options: ["vanilla", "fabric", "quilt", "forge", "neoforge"],
      },
      size: { control: { type: "range", min: 12, max: 64, step: 2 } },
    },
    args: { loader: "fabric", size: 32 },
  });
</script>

<Story name="Fabric" args={{ loader: "fabric" }} />
<Story name="Quilt" args={{ loader: "quilt" }} />
<Story name="Forge" args={{ loader: "forge" }} />
<Story name="NeoForge" args={{ loader: "neoforge" }} />
<Story name="Vanilla" args={{ loader: "vanilla" }} />
