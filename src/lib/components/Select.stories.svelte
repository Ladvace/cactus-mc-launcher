<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import Select from "./Select.svelte";

  const options = [
    { value: "vanilla", label: "Vanilla" },
    { value: "fabric", label: "Fabric" },
    { value: "forge", label: "Forge" },
    { value: "quilt", label: "Quilt" },
    { value: "neoforge", label: "NeoForge", disabled: true },
  ];

  const { Story } = defineMeta({
    title: "Components/Select",
    component: Select,
    tags: ["autodocs"],
    args: { value: "fabric", options, placeholder: "Choose a loader" },
  });
</script>

<Story name="Default" />
<Story name="Searchable" args={{ searchable: true }} />
<Story name="Nothing selected" args={{ value: "", placeholder: "Choose a loader" }} />
