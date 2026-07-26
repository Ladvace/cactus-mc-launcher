<script module>
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import Modal from "./Modal.svelte";

  const { Story } = defineMeta({
    title: "Components/Modal",
    component: Modal,
  });
</script>

<Story name="With footer">
  <Modal title="Delete instance?" open={true} onClose={() => {}}>
    <p>This permanently deletes the instance and all of its files. This can't be undone.</p>
    {#snippet footer()}
      <button class="btn ghost">Cancel</button>
      <button class="btn danger">Delete</button>
    {/snippet}
  </Modal>
</Story>

<Story name="Body only">
  <Modal title="About" open={true} onClose={() => {}}>
    <p>Cactus Launcher — a free, open-source Minecraft launcher.</p>
  </Modal>
</Story>
