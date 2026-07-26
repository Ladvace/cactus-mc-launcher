<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import Flag from "./Flag.svelte";

  const { Story } = defineMeta({
    title: "Components/Flag",
    component: Flag,
    tags: ["autodocs"],
    argTypes: {
      code: {
        control: "select",
        options: ["en", "fr", "de", "es", "pt", "pl", "ru", "tr", "ar", "ja", "ko", "zh"],
      },
      width: { control: { type: "range", min: 12, max: 64, step: 2 } },
    },
    args: { code: "en", width: 28 },
  });
</script>

<Story name="English" args={{ code: "en" }} />
<Story name="Japanese" args={{ code: "ja" }} />
<Story name="Large German" args={{ code: "de", width: 48 }} />
