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
  const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));
  const iconFor = (w: number, h: number) =>
    Math.min(w, h) >= 2 ? 120 : Math.max(w, h) >= 2 ? 84 : 60;

  // Preview icon size for a folder tile of `w`×`h` cells. The 2×2 preview grid
  // grows with the tile so the contained instances scale up when it's resized.
  const CELL = 168;
  function folderIcon(w: number, h: number): number {
    const availW = w * CELL + (w - 1) * GAP - 28; // folder + preview padding/border
    const availH = h * CELL + (h - 1) * GAP - 60; // + meta row & gaps
    const perW = (availW - 4) / 2; // 2 columns, one 4px gap
    const perH = (availH - 4) / 2; // up to 2 rows
    return clamp(Math.floor(Math.min(perW, perH)), 28, 148);
  }

  const ordered = $derived(
    [...entries].sort((a, b) => cellOf(a.id).order - cellOf(b.id).order)
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

  function onTilePointerDown(e: PointerEvent, entry: Entry) {
    if (e.button !== 0 || resizing) return;
    const el = e.currentTarget as HTMLElement;
    const r = el.getBoundingClientRect();
    press = {
      id: entry.id,
      kind: entry.kind,
      el,
      startX: e.clientX,
      startY: e.clientY,
      offX: e.clientX - r.left,
      offY: e.clientY - r.top,
    };
    started = false;
  }

  function beginDrag(e: PointerEvent) {
    if (!press) return;
    started = true;
    draggingId = press.id;
    dragKind = press.kind;
    dropTarget = null;
    lastOver = null;
    clone = makeClone(press.el);
    moveClone(e);
  }

  function makeClone(el: HTMLElement): HTMLElement {
    const r = el.getBoundingClientRect();
    const c = el.cloneNode(true) as HTMLElement;
    c.querySelectorAll(".handle").forEach((h) => h.remove());
    c.classList.add("drag-clone");
    c.classList.remove("dragging");
    Object.assign(c.style, {
      position: "fixed",
      left: `${r.left}px`,
      top: `${r.top}px`,
      width: `${r.width}px`,
      height: `${r.height}px`,
      margin: "0",
      zIndex: "500",
      pointerEvents: "none",
      opacity: "1",
      transform: "scale(1.05)",
      transition: "transform 0.12s ease",
    });
    document.body.appendChild(c);
    return c;
  }

  function moveClone(e: PointerEvent) {
    if (!clone || !press) return;
    clone.style.left = `${e.clientX - press.offX}px`;
    clone.style.top = `${e.clientY - press.offY}px`;
  }

  function tileUnder(e: PointerEvent): string | null {
    const el = document
      .elementFromPoint(e.clientX, e.clientY)
      ?.closest<HTMLElement>("[data-entry-id]");
    return el?.dataset.entryId ?? null;
  }

  function reorderTo(overId: string) {
    if (!draggingId || draggingId === overId) return;
    const ids = ordered.map((x) => x.id);
    const from = ids.indexOf(draggingId);
    const to = ids.indexOf(overId);
    if (from < 0 || to < 0) return;
    ids.splice(to, 0, ids.splice(from, 1)[0]);
    instanceLayout.reorder(ids);
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

  function uniqueFolderName(): string {
    const existing = new Set(
      instancesStore.instances.map((i) => i.group).filter(Boolean)
    );
    let name = "New folder";
    let k = 2;
    while (existing.has(name)) name = `New folder ${k++}`;
    return name;
  }

  // Fly the clone into the target tile, then remove it — the "join" animation.
  // targetId set => glide + shrink into the group; null => snap back to source.
  function flyCloneInto(targetId: string | null) {
    const c = clone;
    clone = null;
    if (!c) return;
    const joining = !!targetId;
    const destEl = targetId
      ? document.querySelector<HTMLElement>(`[data-entry-id="${CSS.escape(targetId)}"]`)
      : press?.el ?? null;
    const dest = destEl?.getBoundingClientRect();
    if (dest) {
      const cr = c.getBoundingClientRect();
      const dx = dest.left + dest.width / 2 - (cr.left + cr.width / 2);
      const dy = dest.top + dest.height / 2 - (cr.top + cr.height / 2);
      const scale = joining ? 0.32 : 1;
      // Glide over ~0.44s; hold opacity a beat so the shrink reads before it fades.
      c.style.transition =
        "transform 0.44s cubic-bezier(0.32, 0.72, 0.3, 1), opacity 0.34s ease 0.16s";
      c.style.transform = `translate(${dx}px, ${dy}px) scale(${scale})`;
      c.style.opacity = "0";
      setTimeout(() => c.remove(), 480);
    } else {
      c.style.transition = "opacity 0.2s ease";
      c.style.opacity = "0";
      setTimeout(() => c.remove(), 220);
    }
  }

  function commitGroup(targetId: string) {
    const target = ordered.find((x) => x.id === targetId);
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
  function onPointerMove(e: PointerEvent) {
    if (resizing) return onResizeMove(e);
    if (!press) return;

    if (!started) {
      if (Math.hypot(e.clientX - press.startX, e.clientY - press.startY) < DRAG_THRESHOLD)
        return;
      beginDrag(e);
    }
    e.preventDefault();
    moveClone(e);

    const overId = tileUnder(e);

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

  function onPointerUp(e: PointerEvent) {
    if (resizing) return onResizeUp();
    if (!press) return;
    if (started && !arranging && dragKind === "instance" && dropTarget) {
      commitGroup(dropTarget); // drop-to-group without waiting for the dwell
      return;
    }
    endDrag();
  }

  function onGridClickCapture(e: MouseEvent) {
    // Swallow the click the browser fires after a drag so we don't also
    // navigate into the instance / open the folder.
    if (suppressClick) {
      e.stopPropagation();
      e.preventDefault();
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

  function startResize(e: PointerEvent, id: string, axis: Axis) {
    e.preventDefault();
    e.stopPropagation();
    const c = cellOf(id);
    resizing = { id, axis, startX: e.clientX, startY: e.clientY, startW: c.w, startH: c.h };
  }
  function onResizeMove(e: PointerEvent) {
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
  function onResizeUp() {
    resizing = null;
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="grid" class:arranging bind:clientWidth={gridWidth} onclickcapture={onGridClickCapture}>
  {#each ordered as entry (entry.id)}
    {@const c = cellOf(entry.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tile"
      class:dragging={draggingId === entry.id}
      class:droptarget={dropTarget === entry.id}
      class:resizing={resizing?.id === entry.id}
      data-entry-id={entry.id}
      style="grid-column: span {c.w}; grid-row: span {c.h};"
      onpointerdown={(e) => onTilePointerDown(e, entry)}
      animate:flip={{ duration: 180 }}
    >
      {#if entry.kind === "instance"}
        <InstanceCard instance={entry.instance} iconSize={iconFor(c.w, c.h)} fill />
      {:else}
        {@const cover = groupCovers.get(entry.name)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="folder"
          class:has-cover={!!cover}
          role="button"
          tabindex="0"
          onclick={() => onOpenFolder(entry.name)}
          onkeydown={(e) => e.key === "Enter" && onOpenFolder(entry.name)}
          oncontextmenu={(e) => {
            e.preventDefault();
            e.stopPropagation();
            ui.openGroupMenu(entry.name, e.clientX, e.clientY);
          }}
        >
          {#if cover}
            <img class="folder-cover" src={cover} alt={entry.name} />
            <div class="folder-scrim"></div>
          {:else}
            <div class="folder-preview">
              {#each entry.instances.slice(0, 4) as inst (inst.id)}
                <InstanceIcon instance={inst} size={folderIcon(c.w, c.h)} />
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
