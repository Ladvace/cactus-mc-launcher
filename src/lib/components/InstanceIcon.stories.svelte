<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import InstanceIcon from "./InstanceIcon.svelte";

  const base = { icon: null, mcVersion: "1.20.1", loaderVersion: null };

  const { Story } = defineMeta({
    title: "Components/InstanceIcon",
    component: InstanceIcon,
    tags: ["autodocs"],
    argTypes: { size: { control: { type: "range", min: 24, max: 128, step: 4 } } },
    args: { size: 64 },
  });
</script>

<Story name="Fabric fallback" args={{ instance: { ...base, name: "Cactus Pack", loader: "fabric" } }} />
<Story name="Forge fallback" args={{ instance: { ...base, name: "Nether Zombie", loader: "forge" } }} />
<Story name="Single word" args={{ instance: { ...base, name: "Vanilla", loader: "vanilla" } }} />
