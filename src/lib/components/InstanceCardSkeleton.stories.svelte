<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import InstanceCardSkeleton from "./InstanceCardSkeleton.svelte";

  const { Story } = defineMeta({
    title: "Components/InstanceCardSkeleton",
    component: InstanceCardSkeleton,
    tags: ["autodocs"],
  });
</script>

<Story name="Default" />
