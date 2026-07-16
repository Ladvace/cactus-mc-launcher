<script lang="ts" module>
  import type { Instance } from "$lib/types";
  export type Entry =
    | { kind: "instance"; id: string; instance: Instance }
    | { kind: "folder"; id: string; name: string; instances: Instance[] };
</script>

<script lang="ts">
  import InstanceCard from "./InstanceCard.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import { instanceLayout } from "$lib/stores/instanceLayout.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";

  interface Props {
    entries: Entry[];
    arranging?: boolean;
    onOpenFolder: (name: string) => void;
  }
  let { entries, arranging = false, onOpenFolder }: Props = $props();

  const GAP = 16;
  const PITCH = 168 + GAP;
  const MAX_H = 4;

  let gridWidth = $state(0);
  const maxCols = $derived(Math.max(1, Math.floor((gridWidth + GAP) / PITCH)));
  const cellOf = (id: string) => instanceLayout.cellOf(id);
  const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));
  const iconFor = (w: number, h: number) =>
    Math.min(w, h) >= 2 ? 120 : Math.max(w, h) >= 2 ? 84 : 60;

  const ordered = $derived(
    [...entries].sort((a, b) => cellOf(a.id).order - cellOf(b.id).order)
  );

  // --- Drag: reorder (arrange mode) or group (normal mode) ---
  let draggingId = $state<string | null>(null);
  let dragKind = $state<"instance" | "folder" | null>(null);
  let dropTarget = $state<string | null>(null);

  function onDragStart(e: DragEvent, entry: Entry) {
    if (resizing) {
      e.preventDefault();
      return;
    }
    draggingId = entry.id;
    dragKind = entry.kind;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", entry.id);
      // Drag a snapshot of the whole tile, grabbed where the cursor is.
      const el = e.currentTarget as HTMLElement;
      const rect = el.getBoundingClientRect();
      e.dataTransfer.setDragImage(el, e.clientX - rect.left, e.clientY - rect.top);
    }
  }

  // Arrange mode: live reorder as you drag over other tiles.
  function onDragEnter(overId: string) {
    if (!arranging || !draggingId || draggingId === overId) return;
    const ids = ordered.map((x) => x.id);
    const from = ids.indexOf(draggingId);
    const to = ids.indexOf(overId);
    if (from < 0 || to < 0) return;
    ids.splice(to, 0, ids.splice(from, 1)[0]);
    instanceLayout.reorder(ids);
  }

  function onDragOver(e: DragEvent, entry: Entry) {
    if (arranging) {
      e.preventDefault();
      return;
    }
    // Normal mode: dropping an instance onto a folder or another instance groups.
    if (dragKind === "instance" && draggingId && entry.id !== draggingId) {
      e.preventDefault();
      e.stopPropagation(); // don't let the grid clear the target
      dropTarget = entry.id;
    }
  }

  // Over the grid gap (not a tile) — clear the target.
  function onGridDragOver() {
    if (!arranging) dropTarget = null;
  }

  function performGroup(draggedId: string, target: Entry) {
    if (target.id === draggedId) return;
    if (target.kind === "folder") {
      instancesStore.update(draggedId, { group: target.name });
    } else {
      const name = uniqueFolderName();
      instancesStore.update(draggedId, { group: name });
      instancesStore.update(target.instance.id, { group: name });
    }
  }

  function onDrop(e: DragEvent, entry: Entry) {
    if (arranging) return;
    e.preventDefault();
    e.stopPropagation();
    const dragged = draggingId;
    if (dragKind === "instance" && dragged) performGroup(dragged, entry);
    draggingId = null;
    dragKind = null;
    dropTarget = null;
  }

  function uniqueFolderName(): string {
    const existing = new Set(
      instancesStore.instances.map((i) => i.group).filter(Boolean)
    );
    let name = "New folder";
    let k = 2;
    while (existing.has(name)) name = `New folder ${k++}`;
    return name;
  }

  function onDragEnd() {
    // Fallback: some webviews don't deliver `drop` reliably — if we ended over a
    // valid target, group here.
    if (!arranging && draggingId && dragKind === "instance" && dropTarget) {
      const target = ordered.find((x) => x.id === dropTarget);
      if (target) performGroup(draggingId, target);
    }
    draggingId = null;
    dragKind = null;
    dropTarget = null;
  }

  // --- Resize by dragging an edge/corner (arrange mode) ---
  type Axis = "e" | "s" | "se";
  let resizing = $state<{
    id: string;
    axis: Axis;
    startX: number;
    startY: number;
    startW: number;
    startH: number;
  } | null>(null);

  function startResize(e: PointerEvent, id: string, axis: Axis) {
    e.preventDefault();
    e.stopPropagation();
    const c = cellOf(id);
    resizing = { id, axis, startX: e.clientX, startY: e.clientY, startW: c.w, startH: c.h };
  }
  function onPointerMove(e: PointerEvent) {
    const r = resizing;
    if (!r) return;
    let w = r.startW;
    let h = r.startH;
    if (r.axis === "e" || r.axis === "se")
      w = clamp(r.startW + Math.round((e.clientX - r.startX) / PITCH), 1, maxCols);
    if (r.axis === "s" || r.axis === "se")
      h = clamp(r.startH + Math.round((e.clientY - r.startY) / PITCH), 1, MAX_H);
    const cur = cellOf(r.id);
    if (cur.w !== w || cur.h !== h) instanceLayout.set(r.id, { w, h, order: cur.order });
  }
  function onPointerUp() {
    resizing = null;
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="grid"
  class:arranging
  bind:clientWidth={gridWidth}
  ondragover={onGridDragOver}
>
  {#each ordered as entry (entry.id)}
    {@const c = cellOf(entry.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tile"
      class:dragging={draggingId === entry.id}
      class:droptarget={dropTarget === entry.id}
      class:resizing={resizing?.id === entry.id}
      style="grid-column: span {c.w}; grid-row: span {c.h};"
      draggable="true"
      ondragstart={(e) => onDragStart(e, entry)}
      ondragenter={() => onDragEnter(entry.id)}
      ondragover={(e) => onDragOver(e, entry)}
      ondrop={(e) => onDrop(e, entry)}
      ondragend={onDragEnd}
    >
      {#if entry.kind === "instance"}
        <InstanceCard instance={entry.instance} iconSize={iconFor(c.w, c.h)} fill />
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="folder"
          role="button"
          tabindex="0"
          onclick={() => onOpenFolder(entry.name)}
          onkeydown={(e) => e.key === "Enter" && onOpenFolder(entry.name)}
        >
          <div class="folder-preview">
            {#each entry.instances.slice(0, 4) as inst (inst.id)}
              <InstanceIcon instance={inst} size={Math.min(48, iconFor(c.w, c.h) / 2.2)} />
            {/each}
          </div>
          <div class="folder-meta">
            <span class="folder-name" title={entry.name}>{entry.name}</span>
            <span class="folder-count">{entry.instances.length}</span>
          </div>
        </div>
      {/if}

      {#if arranging}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="handle e" onpointerdown={(e) => startResize(e, entry.id, "e")}></div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="handle s" onpointerdown={(e) => startResize(e, entry.id, "s")}></div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="handle se" onpointerdown={(e) => startResize(e, entry.id, "se")}></div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, 168px);
    grid-auto-rows: 168px;
    grid-auto-flow: row dense;
    gap: 16px;
    justify-content: start;
  }
  .tile {
    position: relative;
    min-width: 0;
    cursor: grab;
    transition: transform 0.12s;
  }
  .grid.arranging .tile {
    animation: jiggle 0.5s ease-in-out infinite;
  }
  .grid.arranging .tile:nth-child(even) {
    animation-delay: -0.25s;
  }
  .grid.arranging .tile:hover,
  .tile.resizing {
    animation: none !important;
  }
  .grid.arranging .tile :global(.card) {
    pointer-events: none;
    border-style: dashed;
  }
  .tile.dragging {
    opacity: 0.45;
    animation: none;
    transform: scale(0.95);
  }
  .tile.droptarget :global(.card),
  .tile.droptarget .folder {
    outline: 3px solid var(--accent);
    outline-offset: -3px;
  }
  @keyframes jiggle {
    0%,
    100% {
      transform: rotate(-0.5deg);
    }
    50% {
      transform: rotate(0.5deg);
    }
  }

  /* Folder tile */
  .folder {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    transition: border-color 0.12s, transform 0.12s;
  }
  .folder:hover {
    border-color: var(--accent);
    transform: translateY(-2px);
  }
  .folder-preview {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(2, auto);
    grid-auto-rows: min-content;
    gap: 6px;
    align-content: center;
    justify-content: center;
    background: var(--bg-input);
    border: 2px solid var(--border-subtle);
    padding: 8px;
  }
  .folder-meta {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .folder-name {
    flex: 1;
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .folder-count {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-input);
    border: 2px solid var(--border-subtle);
    padding: 0 6px;
  }

  .handle {
    position: absolute;
    z-index: 3;
    touch-action: none;
  }
  .handle.e {
    top: 0;
    right: -4px;
    width: 12px;
    height: 100%;
    cursor: ew-resize;
  }
  .handle.s {
    left: 0;
    bottom: -4px;
    height: 12px;
    width: 100%;
    cursor: ns-resize;
  }
  .handle.se {
    right: -4px;
    bottom: -4px;
    width: 18px;
    height: 18px;
    cursor: nwse-resize;
    background: linear-gradient(
      135deg,
      transparent 45%,
      var(--accent) 45%,
      var(--accent) 60%,
      transparent 60%,
      transparent 72%,
      var(--accent) 72%,
      var(--accent) 87%,
      transparent 87%
    );
    z-index: 4;
  }
</style>
