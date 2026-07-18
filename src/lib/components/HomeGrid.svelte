<script lang="ts" module>
  import type { Instance } from "$lib/types";
  export type Entry =
    | { kind: "instance"; id: string; instance: Instance }
    | { kind: "folder"; id: string; name: string; instances: Instance[] };
</script>

<script lang="ts">
  import { flip } from "svelte/animate";
  import InstanceCard from "./InstanceCard.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import { instanceLayout } from "$lib/stores/instanceLayout.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { groupCovers } from "$lib/stores/groupCovers.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { toast } from "$lib/stores/toast.svelte";

  interface Props {
    entries: Entry[];
    arranging?: boolean;
    onOpenFolder: (name: string) => void;
  }
  let { entries, arranging = false, onOpenFolder }: Props = $props();

  const GAP = 16;
  const PITCH = 168 + GAP;
  const MAX_H = 4;
  const DRAG_THRESHOLD = 6; // px before a press becomes a drag

  let gridWidth = $state(0);
  const maxCols = $derived(Math.max(1, Math.floor((gridWidth + GAP) / PITCH)));
  const cellOf = (id: string) => instanceLayout.cellOf(id);
  const clamp = (value: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, value));
  const iconFor = (width: number, height: number) =>
    Math.min(width, height) >= 2 ? 120 : Math.max(width, height) >= 2 ? 84 : 60;

  // Preview icon size for a folder tile of `w`×`h` cells. The 2×2 preview grid
  // grows with the tile so the contained instances scale up when it's resized.
  const CELL = 168;
  function folderIcon(width: number, height: number): number {
    const availW = width * CELL + (width - 1) * GAP - 28; // folder + preview padding/border
    const availH = height * CELL + (height - 1) * GAP - 60; // + meta row & gaps
    const perW = (availW - 4) / 2; // 2 columns, one 4px gap
    const perH = (availH - 4) / 2; // up to 2 rows
    return clamp(Math.floor(Math.min(perW, perH)), 28, 148);
  }

  const ordered = $derived(
    [...entries].sort((first, second) => cellOf(first.id).order - cellOf(second.id).order)
  );

  // --- Pointer-based drag: reorder (arrange mode) or group (normal mode) ---
  let draggingId = $state<string | null>(null);
  let dragKind = $state<"instance" | "folder" | null>(null);
  let dropTarget = $state<string | null>(null); // groupable tile under the cursor

  // Non-reactive pointer session bookkeeping.
  let press: {
    id: string;
    kind: "instance" | "folder";
    el: HTMLElement;
    startX: number;
    startY: number;
    offX: number; // cursor offset inside the tile
    offY: number;
  } | null = null;
  let started = false;
  let clone: HTMLElement | null = null;
  let suppressClick = false;
  let lastOver: string | null = null; // last tile hovered (arrange reorder guard)

  function onTilePointerDown(event: PointerEvent, entry: Entry) {
    if (event.button !== 0 || resizing) return;
    const el = event.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    press = {
      id: entry.id,
      kind: entry.kind,
      el,
      startX: event.clientX,
      startY: event.clientY,
      offX: event.clientX - rect.left,
      offY: event.clientY - rect.top,
    };
    started = false;
  }

  function beginDrag(event: PointerEvent) {
    if (!press) return;
    started = true;
    draggingId = press.id;
    dragKind = press.kind;
    dropTarget = null;
    lastOver = null;
    clone = makeClone(press.el);
    moveClone(event);
  }

  function makeClone(el: HTMLElement): HTMLElement {
    const rect = el.getBoundingClientRect();
    const cloneEl = el.cloneNode(true) as HTMLElement;
    cloneEl.querySelectorAll(".handle").forEach((handle) => handle.remove());
    cloneEl.classList.add("drag-clone");
    cloneEl.classList.remove("dragging");
    Object.assign(cloneEl.style, {
      position: "fixed",
      left: `${rect.left}px`,
      top: `${rect.top}px`,
      width: `${rect.width}px`,
      height: `${rect.height}px`,
      margin: "0",
      zIndex: "500",
      pointerEvents: "none",
      opacity: "1",
      transform: "scale(1.05)",
      transition: "transform 0.12s ease",
    });
    document.body.appendChild(cloneEl);
    return cloneEl;
  }

  function moveClone(event: PointerEvent) {
    if (!clone || !press) return;
    clone.style.left = `${event.clientX - press.offX}px`;
    clone.style.top = `${event.clientY - press.offY}px`;
  }

  function tileUnder(event: PointerEvent): string | null {
    const el = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>("[data-entry-id]");
    return el?.dataset.entryId ?? null;
  }

  function reorderTo(overId: string) {
    if (!draggingId || draggingId === overId) return;
    const ids = ordered.map((entry) => entry.id);
    const from = ids.indexOf(draggingId);
    const to = ids.indexOf(overId);
    if (from < 0 || to < 0) return;
    ids.splice(to, 0, ids.splice(from, 1)[0]);
    instanceLayout.reorder(ids);
  }

  function performGroup(draggedId: string, target: Entry) {
    if (target.id === draggedId) return;
    if (target.kind === "folder") {
      instancesStore
        .update(draggedId, { group: target.name })
        .catch((e) => toast.error(String(e)));
    } else {
      const name = uniqueFolderName();
      instancesStore
        .update(draggedId, { group: name })
        .catch((e) => toast.error(String(e)));
      instancesStore
        .update(target.instance.id, { group: name })
        .catch((e) => toast.error(String(e)));
    }
  }

  function uniqueFolderName(): string {
    const existing = new Set(
      instancesStore.instances.map((instance) => instance.group).filter(Boolean)
    );
    let name = "New folder";
    let suffix = 2;
    while (existing.has(name)) name = `New folder ${suffix++}`;
    return name;
  }

  // Fly the clone into the target tile, then remove it — the "join" animation.
  // targetId set => glide + shrink into the group; null => snap back to source.
  function flyCloneInto(targetId: string | null) {
    const cloneEl = clone;
    clone = null;
    if (!cloneEl) return;
    const joining = !!targetId;
    const destEl = targetId
      ? document.querySelector<HTMLElement>(`[data-entry-id="${CSS.escape(targetId)}"]`)
      : press?.el ?? null;
    const dest = destEl?.getBoundingClientRect();
    if (dest) {
      const cloneRect = cloneEl.getBoundingClientRect();
      const dx = dest.left + dest.width / 2 - (cloneRect.left + cloneRect.width / 2);
      const dy = dest.top + dest.height / 2 - (cloneRect.top + cloneRect.height / 2);
      const scale = joining ? 0.32 : 1;
      // Glide over ~0.44s; hold opacity a beat so the shrink reads before it fades.
      cloneEl.style.transition =
        "transform 0.44s cubic-bezier(0.32, 0.72, 0.3, 1), opacity 0.34s ease 0.16s";
      cloneEl.style.transform = `translate(${dx}px, ${dy}px) scale(${scale})`;
      cloneEl.style.opacity = "0";
      setTimeout(() => cloneEl.remove(), 480);
    } else {
      cloneEl.style.transition = "opacity 0.2s ease";
      cloneEl.style.opacity = "0";
      setTimeout(() => cloneEl.remove(), 220);
    }
  }

  function commitGroup(targetId: string) {
    const target = ordered.find((entry) => entry.id === targetId);
    if (!target || !draggingId || dragKind !== "instance") return;
    const dragged = draggingId;
    flyCloneInto(targetId);
    performGroup(dragged, target);
    endDrag();
  }

  function endDrag() {
    if (clone) {
      // Cancelled without grouping — snap back to where it came from.
      flyCloneInto(null);
    }
    if (started) suppressClick = true;
    started = false;
    press = null;
    draggingId = null;
    dragKind = null;
    dropTarget = null;
  }

  // --- Global pointer handlers (shared with resize below) ---
  function onPointerMove(event: PointerEvent) {
    if (resizing) return onResizeMove(event);
    if (!press) return;

    if (!started) {
      if (Math.hypot(event.clientX - press.startX, event.clientY - press.startY) < DRAG_THRESHOLD)
        return;
      beginDrag(event);
    }
    event.preventDefault();
    moveClone(event);

    const overId = tileUnder(event);

    if (arranging) {
      if (overId && overId !== draggingId && overId !== lastOver) reorderTo(overId);
      lastOver = overId;
      return;
    }

    // Normal mode: highlight a groupable tile under the cursor. Grouping only
    // happens when the button is released over it (see onPointerUp).
    if (dragKind !== "instance") return;
    dropTarget = overId && overId !== draggingId ? overId : null;
  }

  function onPointerUp(event: PointerEvent) {
    if (resizing) return onResizeUp();
    if (!press) return;
    if (started && !arranging && dragKind === "instance" && dropTarget) {
      commitGroup(dropTarget); // drop-to-group without waiting for the dwell
      return;
    }
    endDrag();
  }

  function onGridClickCapture(event: MouseEvent) {
    // Swallow the click the browser fires after a drag so we don't also
    // navigate into the instance / open the folder.
    if (suppressClick) {
      event.stopPropagation();
      event.preventDefault();
      suppressClick = false;
    }
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

  function startResize(event: PointerEvent, id: string, axis: Axis) {
    event.preventDefault();
    event.stopPropagation();
    const cell = cellOf(id);
    resizing = { id, axis, startX: event.clientX, startY: event.clientY, startW: cell.w, startH: cell.h };
  }
  function onResizeMove(event: PointerEvent) {
    const session = resizing;
    if (!session) return;
    let width = session.startW;
    let height = session.startH;
    if (session.axis === "e" || session.axis === "se")
      width = clamp(session.startW + Math.round((event.clientX - session.startX) / PITCH), 1, maxCols);
    if (session.axis === "s" || session.axis === "se")
      height = clamp(session.startH + Math.round((event.clientY - session.startY) / PITCH), 1, MAX_H);
    const cur = cellOf(session.id);
    if (cur.w !== width || cur.h !== height)
      instanceLayout.set(session.id, { w: width, h: height, order: cur.order });
  }
  function onResizeUp() {
    resizing = null;
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="grid" class:arranging bind:clientWidth={gridWidth} onclickcapture={onGridClickCapture}>
  {#each ordered as entry (entry.id)}
    {@const cell = cellOf(entry.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tile"
      class:dragging={draggingId === entry.id}
      class:droptarget={dropTarget === entry.id}
      class:resizing={resizing?.id === entry.id}
      data-entry-id={entry.id}
      style="grid-column: span {cell.w}; grid-row: span {cell.h};"
      onpointerdown={(event) => onTilePointerDown(event, entry)}
      animate:flip={{ duration: 180 }}
    >
      {#if entry.kind === "instance"}
        <InstanceCard instance={entry.instance} iconSize={iconFor(cell.w, cell.h)} fill />
      {:else}
        {@const cover = groupCovers.get(entry.name)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="folder"
          class:has-cover={!!cover}
          role="button"
          tabindex="0"
          onclick={() => onOpenFolder(entry.name)}
          onkeydown={(event) => event.key === "Enter" && onOpenFolder(entry.name)}
          oncontextmenu={(event) => {
            event.preventDefault();
            event.stopPropagation();
            ui.openGroupMenu(entry.name, event.clientX, event.clientY);
          }}
        >
          {#if cover}
            <img class="folder-cover" src={cover} alt={entry.name} />
            <div class="folder-scrim"></div>
          {:else}
            <div class="folder-preview">
              {#each entry.instances.slice(0, 4) as inst (inst.id)}
                <InstanceIcon instance={inst} size={folderIcon(cell.w, cell.h)} />
              {/each}
            </div>
          {/if}
          <div class="folder-meta" class:on-cover={!!cover}>
            <span class="folder-name" title={entry.name}>{entry.name}</span>
            <span class="folder-count">{entry.instances.length}</span>
          </div>
        </div>
      {/if}

      {#if arranging}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="handle e" onpointerdown={(event) => startResize(event, entry.id, "e")}></div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="handle s" onpointerdown={(event) => startResize(event, entry.id, "s")}></div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="handle se" onpointerdown={(event) => startResize(event, entry.id, "se")}></div>
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
    touch-action: none;
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
  /* Source tile becomes a faint "hole" — the opaque clone is what you drag. */
  .tile.dragging {
    opacity: 0.28;
    animation: none;
  }
  .tile.dragging :global(.card),
  .tile.dragging .folder {
    filter: grayscale(0.4);
  }
  .tile.droptarget :global(.card),
  .tile.droptarget .folder {
    outline: 3px solid var(--accent);
    outline-offset: -3px;
    transform: scale(1.03);
    transition: transform 0.12s ease;
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

  /* Floating drag clone (appended to <body>). */
  :global(.drag-clone) {
    cursor: grabbing;
    box-shadow: 0 16px 34px rgba(0, 0, 0, 0.5);
    will-change: transform, left, top;
  }

  /* Folder tile */
  .folder {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
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
  /* Cover mode: a full-bleed image behind the folder name. */
  .folder.has-cover {
    padding: 0;
    overflow: hidden;
    justify-content: flex-end;
  }
  .folder-cover {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    image-rendering: pixelated;
  }
  .folder-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to top,
      rgba(0, 0, 0, 0.82) 0%,
      rgba(0, 0, 0, 0.35) 34%,
      rgba(0, 0, 0, 0) 62%
    );
    pointer-events: none;
  }
  .folder-meta.on-cover {
    position: relative;
    z-index: 1;
    padding: 10px 12px;
  }
  .folder-meta.on-cover .folder-name {
    color: #fff;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
  }
  .folder-preview {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(2, auto);
    grid-auto-rows: min-content;
    gap: 4px;
    align-content: center;
    justify-content: center;
    background: var(--bg-input);
    border: 2px solid var(--border-subtle);
    padding: 4px;
    overflow: hidden;
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
