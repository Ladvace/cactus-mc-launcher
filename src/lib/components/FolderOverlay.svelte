<script lang="ts">
  import Modal from "./Modal.svelte";
  import InstanceCard from "./InstanceCard.svelte";
  import Icon from "./Icon.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";

  interface Props {
    name: string | null; // null = closed
    onClose: () => void;
  }
  let { name, onClose }: Props = $props();

  // Track the current group name locally so renames don't lose the view.
  let current = $state("");
  $effect(() => {
    if (name) current = name;
  });

  const instances = $derived(
    current ? instancesStore.instances.filter((i) => i.group === current) : []
  );
  let editName = $state("");
  $effect(() => {
    editName = current;
  });

  async function rename() {
    const next = editName.trim();
    if (!next || next === current) return;
    const members = instancesStore.instances.filter((i) => i.group === current);
    for (const i of members) await instancesStore.update(i.id, { group: next });
    current = next;
  }

  async function removeFromFolder(id: string) {
    await instancesStore.update(id, { group: "" });
  }

  // Drag an instance out onto the remove zone to ungroup it.
  let draggingId = $state<string | null>(null);
  let overRemove = $state(false);

  function onCardDragStart(e: DragEvent, id: string) {
    draggingId = id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", id);
      const el = e.currentTarget as HTMLElement;
      const r = el.getBoundingClientRect();
      e.dataTransfer.setDragImage(el, e.clientX - r.left, e.clientY - r.top);
    }
  }
  async function onRemoveDrop(e: DragEvent) {
    e.preventDefault();
    overRemove = false;
    const id = draggingId;
    draggingId = null;
    if (id) await removeFromFolder(id);
  }

  async function ungroupAll() {
    const members = instancesStore.instances.filter((i) => i.group === current);
    for (const i of members) await instancesStore.update(i.id, { group: "" });
    onClose();
  }
</script>

<Modal title="Folder" open={!!name} {onClose} width={640}>
  <div class="folder-head">
    <input
      class="input name"
      bind:value={editName}
      onblur={rename}
      onkeydown={(e) => e.key === "Enter" && rename()}
      aria-label="Folder name"
    />
    <button class="btn danger sm" onclick={ungroupAll}>Ungroup all</button>
  </div>

  {#if instances.length === 0}
    <p class="empty">This folder is empty.</p>
  {:else}
    <div class="grid">
      {#each instances as inst (inst.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="cell"
          draggable="true"
          ondragstart={(e) => onCardDragStart(e, inst.id)}
          ondragend={() => (draggingId = null)}
        >
          <InstanceCard instance={inst} iconSize={64} fill />
          <button
            class="pop"
            title="Remove from folder"
            onclick={() => removeFromFolder(inst.id)}
          >
            <Icon name="trash" size={12} />
          </button>
        </div>
      {/each}
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="remove-zone"
      class:over={overRemove}
      class:armed={!!draggingId}
      ondragover={(e) => {
        e.preventDefault();
        overRemove = true;
      }}
      ondragleave={() => (overRemove = false)}
      ondrop={onRemoveDrop}
    >
      <Icon name="trash" size={14} /> Drag here to remove from the folder
    </div>
  {/if}
  <p class="hint">Drag an instance onto this folder's tile to add it. Right-click for more.</p>
</Modal>

<style>
  .folder-head {
    display: flex;
    gap: 10px;
    margin-bottom: 16px;
  }
  .name {
    flex: 1;
    font-family: var(--font-pixel);
    font-size: 16px;
  }
  .btn.sm {
    padding: 6px 12px;
    font-size: 12px;
    flex-shrink: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }
  .cell {
    position: relative;
    height: 170px;
  }
  .pop {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 2;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    opacity: 0;
    transition: opacity 0.12s;
  }
  .cell:hover .pop {
    opacity: 1;
  }
  .pop:hover {
    border-color: var(--danger);
    color: var(--danger);
  }
  .cell {
    cursor: grab;
  }
  /* A drop target that appears while dragging a card, to ungroup it. */
  .remove-zone {
    display: none;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-top: 14px;
    padding: 16px;
    border: 2px dashed var(--border);
    color: var(--text-muted);
    font-size: 13px;
  }
  .remove-zone.armed {
    display: flex;
  }
  .remove-zone.over {
    border-color: var(--danger);
    color: var(--danger);
    background: rgba(255, 91, 110, 0.08);
  }
  .remove-zone :global(.hn) {
    color: inherit;
  }
  .empty {
    color: var(--text-muted);
    text-align: center;
    padding: 24px;
  }
  .hint {
    margin: 14px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
