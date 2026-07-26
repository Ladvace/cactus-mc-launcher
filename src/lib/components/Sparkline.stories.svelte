<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import Sparkline from "./Sparkline.svelte";

  const { Story } = defineMeta({
    title: "Components/Sparkline",
    component: Sparkline,
    tags: ["autodocs"],
    argTypes: {
      width: { control: { type: "range", min: 80, max: 400, step: 10 } },
      height: { control: { type: "range", min: 20, max: 120, step: 4 } },
    },
    args: { values: [3, 5, 2, 8, 6, 9, 4, 7, 5, 10], width: 220, height: 48 },
  });
</script>

<Story name="Player counts" args={{ values: [3, 5, 2, 8, 6, 9, 4, 7, 5, 10] }} />
<Story name="With gaps" args={{ values: [2, 4, null, null, 7, 6, null, 9, 8] }} />
<Story name="Flat" args={{ values: [4, 4, 4, 4, 4, 4] }} />
