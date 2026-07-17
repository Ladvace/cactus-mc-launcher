<script lang="ts">
  import Modal from "./Modal.svelte";
  import InstanceCard from "./InstanceCard.svelte";
  import Icon from "./Icon.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { groupCovers } from "$lib/stores/groupCovers.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { fileToIconDataUri } from "$lib/image";
  import { toast } from "$lib/stores/toast.svelte";

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
    current ? instancesStore.instances.filter((instance) => instance.group === current) : []
  );
  let editName = $state("");
  $effect(() => {
    editName = current;
  });

  // Reassign every instance currently in this folder to `group` ("" ungroups).
  async function setMembersGroup(group: string) {
    const members = instancesStore.instances.filter((instance) => instance.group === current);
    for (const instance of members) await instancesStore.update(instance.id, { group });
  }

  async function rename() {
    const next = editName.trim();
    if (!next || next === current) return;
    await setMembersGroup(next);
    groupCovers.rename(current, next);
    current = next;
  }

  // --- Folder cover image ---
  let coverInput = $state<HTMLInputElement>();
  const cover = $derived(current ? groupCovers.get(current) : null);

  async function onCoverFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file || !current) return;
    try {
      groupCovers.set(current, await fileToIconDataUri(file));
    } catch (err) {
      toast.error(String(err));
    }
  }
  function pickCoverSticker() {
    if (!current) return;
    const name = current;
    ui.openStickerPicker(`Cover for ${name}`, (uri) => groupCovers.set(name, uri));
  }
  function clearCover() {
    if (current) groupCovers.clear(current);
  }

  async function removeFromFolder(id: string) {
    await instancesStore.update(id, { group: "" });
  }

  // Drag an instance out of the folder to ungroup it (pointer-based, so the
  // dragged tile stays fully opaque and the drop is reliable in the webview).
  let draggingId = $state<string | null>(null);
  let overRemove = $state(false); // cursor is in "ungroup" territory
  let suppressClick = false;

  let press: {
    id: string;
    el: HTMLElement;
    startX: number;
    startY: number;
    offX: number;
    offY: number;
  } | null = null;
  let started = false;
  let clone: HTMLElement | null = null;

  function onCardPointerDown(event: PointerEvent, id: string) {
    if (event.button !== 0) return;
    const el = event.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    press = {
      id,
      el,
      startX: event.clientX,
      startY: event.clientY,
      offX: event.clientX - rect.left,
      offY: event.clientY - rect.top,
    };
    started = false;
  }

  function makeClone(el: HTMLElement): HTMLElement {
    const rect = el.getBoundingClientRect();
    const cloneEl = el.cloneNode(true) as HTMLElement;
    cloneEl.querySelectorAll(".pop").forEach((popover) => popover.remove());
    Object.assign(cloneEl.style, {
      position: "fixed",
      left: `${rect.left}px`,
      top: `${rect.top}px`,
      width: `${rect.width}px`,
      height: `${rect.height}px`,
      margin: "0",
      zIndex: "1200",
      pointerEvents: "none",
      opacity: "1",
      transform: "scale(1.05)",
      transition: "transform 0.12s ease",
      boxShadow: "0 16px 34px rgba(0, 0, 0, 0.5)",
    });
    document.body.appendChild(cloneEl);
    return cloneEl;
  }

  function insideGrid(event: PointerEvent): boolean {
    return !!document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest("[data-folder-grid]");
  }

  function onPointerMove(event: PointerEvent) {
    if (!press) return;
    if (!started) {
      if (Math.hypot(event.clientX - press.startX, event.clientY - press.startY) < 6) return;
      started = true;
      draggingId = press.id;
      clone = makeClone(press.el);
    }
    event.preventDefault();
    if (clone) {
      clone.style.left = `${event.clientX - press.offX}px`;
      clone.style.top = `${event.clientY - press.offY}px`;
    }
    // Anywhere outside the folder's own grid counts as "drop to ungroup".
    overRemove = !insideGrid(event);
  }

  async function onPointerUp() {
    if (!press) return;
    const doRemove = started && overRemove;
    const id = press.id;
    if (clone) {
      clone.style.transition = "opacity 0.18s ease";
      clone.style.opacity = "0";
      const cloneEl = clone;
      setTimeout(() => cloneEl.remove(), 200);
    }
    if (started) suppressClick = true;
    clone = null;
    started = false;
    press = null;
    draggingId = null;
    overRemove = false;
    if (doRemove) await removeFromFolder(id);
  }

  function onGridClickCapture(event: MouseEvent) {
    if (suppressClick) {
      event.stopPropagation();
      event.preventDefault();
      suppressClick = false;
    }
  }

  async function ungroupAll() {
    await setMembersGroup("");
    onClose();
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<Modal title="Folder" open={!!name} {onClose} width={640}>
  <div class="folder-head">
    <input
      class="input name"
      bind:value={editName}
      onblur={rename}
      onkeydown={(event) => event.key === "Enter" && rename()}
      aria-label="Folder name"
    />
    <button class="btn danger sm" onclick={ungroupAll}>Ungroup all</button>
  </div>

  <div class="cover-controls">
    <span class="cover-label">Cover</span>
    <button class="btn ghost sm" onclick={() => coverInput?.click()}>
      <Icon name="edit" size={13} /> Upload…
    </button>
    <button class="btn ghost sm" onclick={pickCoverSticker}>
      <Icon name="sparkles" size={13} /> Stickers…
    </button>
    {#if cover}
      <button class="btn ghost sm" onclick={clearCover}>Remove</button>
    {/if}
    <input
      bind:this={coverInput}
      type="file"
      accept="image/png,image/jpeg,image/gif,image/webp"
      style="display:none"
      onchange={onCoverFile}
    />
  </div>

  {#if instances.length === 0}
    <p class="empty">This folder is empty.</p>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="grid" data-folder-grid onclickcapture={onGridClickCapture}>
      {#each instances as inst (inst.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="cell"
          class:dragging={draggingId === inst.id}
          onpointerdown={(event) => onCardPointerDown(event, inst.id)}
        >
          <InstanceCard instance={inst} iconSize={64} fill />
        </div>
      {/each}
    </div>

    <div class="remove-zone" class:over={overRemove} class:armed={!!draggingId}>
      <Icon name="trash" size={14} />
      {overRemove ? "Release to remove from the folder" : "Drag a tile out here to remove it"}
    </div>
  {/if}
  <p class="hint">Drag an instance out of the folder to ungroup it. Right-click for more.</p>
</Modal>

<style>
  .folder-head {
    display: flex;
    gap: 10px;
    margin-bottom: 12px;
  }
  .cover-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
  }
  .cover-label {
    font-size: 12px;
    color: var(--text-muted);
  }
  .name {
    flex: 1;
    font-family: var(--font-pixel);
    font-size: 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }
  .cell {
    position: relative;
    height: 170px;
    touch-action: none;
  }
  .cell.dragging {
    opacity: 0.28;
    filter: grayscale(0.4);
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
