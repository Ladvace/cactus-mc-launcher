<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import ProgressBar from "./ProgressBar.svelte";

  const { Story } = defineMeta({
    title: "Components/ProgressBar",
    component: ProgressBar,
    tags: ["autodocs"],
    argTypes: {
      pct: { control: { type: "range", min: 0, max: 100, step: 1 } },
    },
    args: { label: "Downloading libraries", pct: 45 },
  });
</script>

<Story name="In progress" args={{ label: "Downloading libraries", pct: 45 }} />
<Story name="Almost done" args={{ label: "Extracting assets", pct: 92 }} />
<Story name="Complete" args={{ label: "Ready", pct: 100 }} />
<Story name="Indeterminate" args={{ label: "Preparing…", pct: null }} />
