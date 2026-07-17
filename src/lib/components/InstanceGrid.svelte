<script lang="ts">
  import InstanceCard from "./InstanceCard.svelte";
  import { instanceLayout, type Cell } from "$lib/stores/instanceLayout.svelte";
  import type { Instance } from "$lib/types";

  interface Props {
    instances: Instance[];
    arranging?: boolean;
  }
  let { instances, arranging = false }: Props = $props();

  // Grid geometry — a fixed cell so spans produce consistent tiles.
  const CELL = 168;
  const GAP = 16;
  const PITCH = CELL + GAP; // distance between two cell origins
  const MAX_H = 4;

  export function resetLayout() {
    instanceLayout.reset();
  }

  // How many columns currently fit — clamps the max width a tile can grow to.
  let gridWidth = $state(0);
  const maxCols = $derived(Math.max(1, Math.floor((gridWidth + GAP) / PITCH)));

  const cellOf = (id: string): Cell => instanceLayout.cellOf(id);
  const clamp = (value: number, lo: number, hi: number) =>
    Math.min(hi, Math.max(lo, value));

  function iconFor(width: number, height: number): number {
    if (Math.min(width, height) >= 2) return 120;
    if (Math.max(width, height) >= 2) return 84;
    return 60;
  }

  // Instances sorted by their stored order (unplaced ones fall to the end).
  const ordered = $derived(
    [...instances].sort(
      (first, second) => cellOf(first.id).order - cellOf(second.id).order
    )
  );

  // --- Drag to reorder (HTML5 DnD, live reflow) -------------------------------
  let draggingId = $state<string | null>(null);

  function onDragStart(event: DragEvent, id: string) {
    if (!arranging || resizing) {
      event.preventDefault();
      return;
    }
    draggingId = id;
    event.dataTransfer?.setData("text/plain", id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function onDragEnter(overId: string) {
    if (!draggingId || draggingId === overId) return;
    const ids = ordered.map((instance) => instance.id);
    const from = ids.indexOf(draggingId);
    const to = ids.indexOf(overId);
    if (from < 0 || to < 0) return;
    ids.splice(to, 0, ids.splice(from, 1)[0]);
    instanceLayout.reorder(ids);
  }

  function onDragEnd() {
    draggingId = null;
  }

  // --- Resize by dragging an edge/corner (pointer events) ---------------------
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
    resizing = {
      id,
      axis,
      startX: event.clientX,
      startY: event.clientY,
      startW: cell.w,
      startH: cell.h,
    };
  }

  function onPointerMove(event: PointerEvent) {
    const session = resizing;
    if (!session) return;
    let width = session.startW;
    let height = session.startH;
    if (session.axis === "e" || session.axis === "se") {
      width = clamp(session.startW + Math.round((event.clientX - session.startX) / PITCH), 1, maxCols);
    }
    if (session.axis === "s" || session.axis === "se") {
      height = clamp(session.startH + Math.round((event.clientY - session.startY) / PITCH), 1, MAX_H);
    }
    const cur = cellOf(session.id);
    if (cur.w !== width || cur.h !== height) {
      instanceLayout.set(session.id, { w: width, h: height, order: cur.order });
    }
  }

  function onPointerUp() {
    if (!resizing) return;
    resizing = null;
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<div class="grid" class:arranging bind:clientWidth={gridWidth}>
  {#each ordered as inst (inst.id)}
    {@const cell = cellOf(inst.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tile"
      class:dragging={draggingId === inst.id}
      class:resizing={resizing?.id === inst.id}
      style="grid-column: span {cell.w}; grid-row: span {cell.h};"
      draggable={arranging}
      ondragstart={(event) => onDragStart(event, inst.id)}
      ondragenter={() => onDragEnter(inst.id)}
      ondragover={(event) => arranging && event.preventDefault()}
      ondragend={onDragEnd}
    >
      <InstanceCard instance={inst} iconSize={iconFor(cell.w, cell.h)} fill />
      {#if arranging}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="handle e"
          title="Drag to resize width"
          onpointerdown={(event) => startResize(event, inst.id, "e")}
        ></div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="handle s"
          title="Drag to resize height"
          onpointerdown={(event) => startResize(event, inst.id, "s")}
        ></div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="handle se"
          title="Drag to resize"
          onpointerdown={(event) => startResize(event, inst.id, "se")}
        ></div>
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
    transition: transform 0.12s;
  }
  /* Arrange mode: tiles become draggable and gently jiggle (Android-style).
     Hovering a tile stops its jiggle so grabbing a border stays precise. */
  .grid.arranging .tile {
    cursor: grab;
    animation: jiggle 0.5s ease-in-out infinite;
  }
  .grid.arranging .tile:nth-child(even) {
    animation-delay: -0.25s;
  }
  .grid.arranging .tile:nth-child(3n) {
    animation-delay: -0.12s;
  }
  .grid.arranging .tile:hover,
  .tile.resizing {
    animation: none !important;
  }
  /* Clicks shouldn't navigate/launch while arranging — the tile owns the drag. */
  .grid.arranging .tile :global(.card) {
    pointer-events: none;
    border-style: dashed;
  }
  .grid.arranging .tile:hover :global(.card),
  .tile.resizing :global(.card) {
    border-color: var(--accent);
  }
  .tile.dragging {
    opacity: 0.45;
    animation: none;
    transform: scale(0.95);
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

  /* Invisible drag zones along the right edge, bottom edge, and corner. */
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
  /* Visible corner grip so the resize affordance is discoverable. */
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
