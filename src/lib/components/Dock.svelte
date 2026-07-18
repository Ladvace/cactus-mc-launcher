<script lang="ts">
  import { browser } from "$app/environment";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import Icon from "./Icon.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { installStore } from "$lib/stores/install.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { skinFace } from "$lib/skin";
  import { ui } from "$lib/stores/ui.svelte";
  import { DECOR_THEMES } from "$lib/themes";
  import type { Instance } from "$lib/types";

  interface Props {
    onCreate: () => void;
  }
  let { onCreate }: Props = $props();

  const pos = $derived(settingsStore.settings.dockPosition ?? "bottom");
  const horizontal = $derived(pos === "top" || pos === "bottom");
  const magnify = $derived(settingsStore.settings.dockMagnify ?? true);
  // A themed sprite perched on the dock's corner, from the active decor theme.
  const peek = $derived(
    DECOR_THEMES.find((decorTheme) => decorTheme.id === (settingsStore.settings.decorTheme ?? ""))?.peek
  );

  const path = $derived($page.url.pathname);

  // Track viewport height so a vertical (left/right) dock fits the space
  // instead of running off the top/bottom.
  let winH = $state(browser ? window.innerHeight : 900);

  // How many instance tiles to pin. Top/bottom keep the flat cap of 7; left/
  // right compute how many fit in the height, reserving one slot for the
  // overflow tile when it's needed so the bar never overflows the screen.
  const cap = $derived.by(() => {
    if (horizontal) return 7;
    const chrome = 52; // dock padding + border + a little breathing room
    const perTile = 56; // ITEM (48) + GAP (8)
    const fixed = 6 * perTile + 2 * 14; // 6 fixed tiles + 2 separators
    const slots = Math.max(1, Math.min(7, Math.floor((winH - chrome - fixed) / perTile)));
    const total = instancesStore.instances.length;
    return total <= slots ? slots : Math.max(1, slots - 1);
  });
  const pinned = $derived(instancesStore.instances.slice(0, cap));
  const overflow = $derived(Math.max(0, instancesStore.instances.length - cap));
  const overflowList = $derived(instancesStore.instances.slice(cap));

  // Popover listing the instances that don't fit on the dock. Its position is
  // computed to open away from the docked edge, so an inline style string.
  let overflowMenu = $state<string | null>(null);
  $effect(() => {
    if (overflow === 0) overflowMenu = null;
  });
  function toggleOverflow(event: MouseEvent) {
    if (overflowMenu) {
      overflowMenu = null;
      return;
    }
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    const gap = 10;
    if (pos === "bottom")
      overflowMenu = `left:${centerX}px; bottom:${window.innerHeight - rect.top + gap}px; transform:translateX(-50%);`;
    else if (pos === "top")
      overflowMenu = `left:${centerX}px; top:${rect.bottom + gap}px; transform:translateX(-50%);`;
    else if (pos === "left")
      overflowMenu = `left:${rect.right + gap}px; top:${centerY}px; transform:translateY(-50%);`;
    else
      overflowMenu = `right:${window.innerWidth - rect.left + gap}px; top:${centerY}px; transform:translateY(-50%);`;
  }

  type Item =
    | { kind: "nav"; href: string; icon: string; label: string }
    | { kind: "sep" }
    | { kind: "instance"; instance: Instance; label: string }
    | { kind: "overflow"; count: number; label: string }
    | { kind: "downloads"; pct: number | null; targetId: string | null; label: string }
    | { kind: "add"; label: string }
    | { kind: "settings"; href: string; label: string }
    | { kind: "account"; label: string };

  // A persistent modpack-download indicator, shown on every page while an
  // install runs — except when the installing instance is already a pinned tile
  // (its own tile shows the progress), which would be redundant.
  const dlId = $derived(installStore.primaryInstanceId());
  const showDownloads = $derived(
    installStore.anyActive() && !(dlId && pinned.some((instance) => instance.id === dlId))
  );

  const items = $derived<Item[]>([
    { kind: "nav", href: "/", icon: "home", label: "Home" },
    { kind: "nav", href: "/browse", icon: "compass", label: "Browse" },
    { kind: "nav", href: "/servers", icon: "globe", label: "Servers" },
    { kind: "nav", href: "/share", icon: "users", label: "Community" },
    { kind: "sep" },
    ...pinned.map(
      (instance): Item => ({ kind: "instance", instance: instance, label: instance.name })
    ),
    ...(overflow > 0
      ? [
          {
            kind: "overflow" as const,
            count: overflow,
            label: `${overflow} more on Home`,
          },
        ]
      : []),
    ...(showDownloads
      ? [
          {
            kind: "downloads" as const,
            pct: installStore.overallPct(),
            targetId: dlId,
            label: installStore.overallMessage(),
          },
        ]
      : []),
    { kind: "add", label: "New instance" },
    { kind: "sep" },
    { kind: "settings", href: "/settings", label: "Settings" },
    { kind: "account", label: accountsStore.activeName },
  ]);

  // --- macOS-style magnification ---
  const ITEM = 48;
  const SEP = 6;
  const GAP = 8;
  const PAD = 14; // dock border (2) + padding-left (12)
  const MAX = 1.28; // subtle magnification
  const RANGE = 100;

  // Resting center X of each item, relative to the dock's content start.
  const centers = $derived.by(() => {
    let cx = 0;
    const positions: number[] = [];
    for (const item of items) {
      const width = item.kind === "sep" ? SEP : ITEM;
      positions.push(cx + width / 2);
      cx += width + GAP;
    }
    return positions;
  });

  let dockEl = $state<HTMLElement>();
  let dockStart = 0; // captured on enter (resting frame) to avoid feedback drift
  let scales = $state<number[]>([]);
  // Throttle to one update per animation frame — mousemove can fire far more
  // often than the display refreshes, which otherwise thrashes layout.
  let mouseX = 0;
  let mouseY = 0;
  let inside = false;
  let rafId = 0;

  function apply() {
    rafId = 0;
    if (!inside) return;
    if (!magnify) {
      if (scales.length) scales = [];
      return;
    }
    const base = dockStart + PAD;
    const pointer = horizontal ? mouseX : mouseY;
    scales = centers.map((center) => {
      const distance = Math.abs(pointer - (base + center));
      return distance > RANGE ? 1 : 1 + (MAX - 1) * (1 - distance / RANGE);
    });
  }
  function onEnter() {
    if (dockEl) {
      const rect = dockEl.getBoundingClientRect();
      dockStart = horizontal ? rect.left : rect.top;
    }
    inside = true;
  }
  function onMove(event: MouseEvent) {
    mouseX = event.clientX;
    mouseY = event.clientY;
    if (!rafId) rafId = requestAnimationFrame(apply);
  }
  function reset() {
    inside = false;
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    scales = [];
  }

  function isActive(href: string) {
    return href === "/" ? path === "/" : path.startsWith(href);
  }
  function activate(item: Item, event: MouseEvent) {
    if (item.kind === "nav" || item.kind === "settings") goto(item.href);
    else if (item.kind === "instance") goto(`/instance/${item.instance.id}`);
    else if (item.kind === "overflow") toggleOverflow(event);
    else if (item.kind === "downloads") {
      if (item.targetId) goto(`/instance/${item.targetId}`);
    } else if (item.kind === "add") onCreate();
    else if (item.kind === "account") ui.openAccounts();
  }

  function openOverflowInstance(id: string) {
    overflowMenu = null;
    goto(`/instance/${id}`);
  }
</script>

<div class="dock-wrap" data-pos={pos}>
  <div
    class="dock"
    bind:this={dockEl}
    role="toolbar"
    tabindex="-1"
    aria-label="Navigation dock"
    onmouseenter={onEnter}
    onmousemove={onMove}
    onmouseleave={reset}
  >
    {#each items as item, index (index)}
      {#if item.kind === "sep"}
        <div class="dock-sep"></div>
      {:else}
        {@const active =
          (item.kind === "nav" || item.kind === "settings") &&
          isActive(item.href)}
        {@const activeInstance =
          item.kind === "instance" && path === `/instance/${item.instance.id}`}
        {@const running =
          item.kind === "instance" && launchStore.isRunning(item.instance.id)}
        {@const preparing =
          item.kind === "instance" && launchStore.isBusy(item.instance.id) && !running}
        <button
          class="dock-item"
          class:active={active || activeInstance}
          class:running
          style="--s:{scales[index] ?? 1}"
          onclick={(event) => activate(item, event)}
          aria-label={item.label}
        >
          <span class="tip">{item.label}</span>
          <span class="glyph">
            {#if item.kind === "nav" || item.kind === "settings"}
              <Icon
                name={item.kind === "settings" ? "settings" : item.icon}
                size={24}
              />
            {:else if item.kind === "overflow"}
              <span class="overflow">+{item.count}</span>
            {:else if item.kind === "downloads"}
              <span class="dl-glyph">
                <span class="dock-spinner"></span>
                {#if item.pct !== null}<span class="dock-pct">{item.pct}%</span>{/if}
              </span>
            {:else if item.kind === "add"}
              <Icon name="plus" size={24} />
            {:else if item.kind === "instance"}
              <InstanceIcon instance={item.instance} size={44} />
              {#if item.instance.kind === "server"}
                <span class="kind-badge" title="Dedicated server">S</span>
              {/if}
              {#if running}
                <span class="run-dot" title="Running"></span>
              {:else if preparing}
                <span class="run-dot preparing" title="Preparing…"></span>
              {/if}
              {#if installStore.isInstalling(item.instance.id)}
                <span class="dock-dl">
                  <span class="dock-spinner"></span>
                  {#if installStore.pct(item.instance.id) !== null}
                    <span class="dock-pct">{installStore.pct(item.instance.id)}%</span>
                  {/if}
                </span>
              {/if}
            {:else if item.kind === "account"}
              {#if accountsStore.active}
                <img
                  class="acc"
                  src={skinFace(accountsStore.active.uuid, 44)}
                  alt={item.label}
                />
              {:else}
                <Icon name="user" size={24} />
              {/if}
            {/if}
          </span>
          {#if active || activeInstance}<span class="dot"></span>{/if}
        </button>
      {/if}
    {/each}
    {#if peek}
      <img class="dock-peek" src={peek} alt="" />
    {/if}
  </div>
</div>

<svelte:window
  onkeydown={(event) => event.key === "Escape" && (overflowMenu = null)}
  onresize={() => {
    winH = window.innerHeight;
    overflowMenu = null;
  }}
/>

{#if overflowMenu}
  <button class="ov-backdrop" aria-label="Close menu" onclick={() => (overflowMenu = null)}></button>
  <div class="ov-menu" style={overflowMenu}>
    {#each overflowList as inst (inst.id)}
      <button class="ov-item" onclick={() => openOverflowInstance(inst.id)}>
        <InstanceIcon instance={inst} size={24} />
        <span class="ov-name">{inst.name}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .dock-wrap {
    position: fixed;
    display: flex;
    pointer-events: none;
    z-index: 50;
  }
  .dock-wrap[data-pos="bottom"] {
    left: 0;
    right: 0;
    bottom: 14px;
    justify-content: center;
  }
  .dock-wrap[data-pos="top"] {
    left: 0;
    right: 0;
    top: 14px;
    justify-content: center;
  }
  .dock-wrap[data-pos="left"] {
    top: 0;
    bottom: 0;
    left: 14px;
    align-items: center;
  }
  .dock-wrap[data-pos="right"] {
    top: 0;
    bottom: 0;
    right: 14px;
    align-items: center;
  }
  .dock {
    position: relative;
    pointer-events: auto;
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 8px 12px;
    overflow: visible;
    background: color-mix(in srgb, var(--bg-raised) 92%, transparent);
    border: 2px solid var(--border);
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.5),
      inset 2px 2px 0 rgba(255, 255, 255, 0.05),
      inset -2px -2px 0 rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(8px);
  }
  [data-pos="top"] .dock {
    align-items: flex-start;
  }
  /* Vertical docks: stack items in a column. */
  [data-pos="left"] .dock,
  [data-pos="right"] .dock {
    flex-direction: column;
    padding: 12px 8px;
  }
  [data-pos="left"] .dock {
    align-items: flex-start;
  }
  [data-pos="right"] .dock {
    align-items: flex-end;
  }
  .dock-sep {
    width: 2px;
    align-self: stretch;
    margin: 4px 2px;
    background: var(--border);
  }
  /* Themed sprite that perches on / peeks over the dock's outer corner. */
  .dock-peek {
    position: absolute;
    width: 40px;
    height: auto;
    pointer-events: none;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.45));
  }
  [data-pos="bottom"] .dock-peek {
    bottom: calc(100% - 14px);
    right: 14px;
  }
  [data-pos="top"] .dock-peek {
    top: calc(100% - 14px);
    right: 14px;
  }
  [data-pos="left"] .dock-peek {
    left: calc(100% - 14px);
    top: 10px;
  }
  [data-pos="right"] .dock-peek {
    right: calc(100% - 14px);
    top: 10px;
  }
  [data-pos="left"] .dock-sep,
  [data-pos="right"] .dock-sep {
    width: auto;
    height: 2px;
    margin: 2px 4px;
  }
  .dock-item {
    --s: 1;
    position: relative;
    width: 48px;
    height: 48px;
    padding: 0;
    flex-shrink: 0;
    border: 2px solid transparent;
    border-radius: 0;
    background: var(--bg-card);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    /* transform grows the icon away from the docked edge without resizing the
       bar; the margin reserves flow-axis space so neighbours never overlap. */
    transform: scale(var(--s));
    transform-origin: bottom center;
    margin: 0 calc((var(--s) - 1) * 24px);
    will-change: transform, margin;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.05),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    transition: transform 0.18s ease, margin 0.18s ease, border-color 0.12s,
      color 0.12s;
  }
  [data-pos="top"] .dock-item {
    transform-origin: top center;
  }
  [data-pos="left"] .dock-item {
    transform-origin: left center;
    margin: calc((var(--s) - 1) * 24px) 0;
  }
  [data-pos="right"] .dock-item {
    transform-origin: right center;
    margin: calc((var(--s) - 1) * 24px) 0;
  }
  .dock-item:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .dock-item.active {
    color: var(--accent);
    border-color: var(--border);
  }
  /* A running instance gets a green border + a glowing corner dot. */
  .dock-item.running {
    border-color: #57c84a;
  }
  .run-dot {
    position: absolute;
    top: 2px;
    right: 2px;
    z-index: 3;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #57c84a;
    border: 2px solid var(--bg-card);
    box-shadow: 0 0 6px rgba(87, 200, 74, 0.9);
    pointer-events: none;
  }
  .run-dot.preparing {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
    animation: run-pulse 1s ease-in-out infinite;
  }
  @keyframes run-pulse {
    50% {
      opacity: 0.35;
    }
  }
  /* A fixed inner area so every item — line icon, instance icon, avatar —
     occupies the same centered footprint and lines up across the row. */
  .glyph {
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  /* The dock tile already provides the frame; drop the icon's own border so
     instance/account tiles don't read as a box inside a box. */
  .glyph :global(.icon-img),
  .glyph :global(.icon-fallback) {
    border: none;
  }
  .acc {
    width: 44px;
    height: 44px;
    object-fit: cover;
    image-rendering: pixelated;
  }
  /* Marks a dedicated-server tile, mirroring the "SERVER" badge on Home. */
  .kind-badge {
    position: absolute;
    top: 2px;
    left: 2px;
    z-index: 3;
    padding: 1px 3px;
    font-family: var(--font-pixel);
    font-size: 8px;
    line-height: 1;
    color: var(--bg-app);
    background: var(--accent);
    pointer-events: none;
  }
  .overflow {
    font-family: var(--font-pixel);
    font-size: 15px;
    font-weight: 700;
    color: var(--text-secondary);
  }
  .dock-item:hover .overflow {
    color: var(--accent);
  }
  .ov-backdrop {
    position: fixed;
    inset: 0;
    z-index: 55;
    background: transparent;
    border: none;
    padding: 0;
    cursor: default;
  }
  .ov-menu {
    position: fixed;
    z-index: 56;
    max-height: 320px;
    overflow-y: auto;
    min-width: 200px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5);
  }
  .ov-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    text-align: left;
  }
  .ov-item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .ov-name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dock-dl {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    background: rgba(10, 9, 8, 0.78);
  }
  /* Standalone dock download indicator (pre-creation / non-pinned installs). */
  .dl-glyph {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
  }
  .dock-spinner {
    width: 15px;
    height: 15px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: dock-spin 0.7s linear infinite;
  }
  .dock-pct {
    font-family: var(--font-pixel);
    font-size: 8px;
    color: var(--accent);
  }
  @keyframes dock-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .dot {
    position: absolute;
    bottom: -6px;
    left: 50%;
    transform: translateX(-50%);
    width: 4px;
    height: 4px;
    background: var(--accent);
  }
  [data-pos="top"] .dot {
    bottom: auto;
    top: -6px;
  }
  [data-pos="left"] .dot {
    bottom: 50%;
    left: -6px;
    transform: translateY(50%);
  }
  [data-pos="right"] .dot {
    bottom: 50%;
    left: auto;
    right: -6px;
    transform: translateY(50%);
  }
  .tip {
    position: absolute;
    bottom: calc(100% + 9px);
    left: 50%;
    /* counter-scale so the label stays a normal size while the item magnifies */
    transform: translateX(-50%) scale(calc(1 / var(--s)));
    transform-origin: bottom center;
    padding: 3px 8px;
    background: var(--bg-app);
    border: 2px solid var(--border);
    color: var(--text);
    font-family: var(--font-pixel);
    font-size: 11px;
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.1s;
  }
  /* Tooltips point away from the docked edge. */
  [data-pos="top"] .tip {
    bottom: auto;
    top: calc(100% + 9px);
    transform-origin: top center;
  }
  [data-pos="left"] .tip {
    bottom: auto;
    top: 50%;
    left: calc(100% + 9px);
    transform: translateY(-50%) scale(calc(1 / var(--s)));
    transform-origin: left center;
  }
  [data-pos="right"] .tip {
    bottom: auto;
    top: 50%;
    left: auto;
    right: calc(100% + 9px);
    transform: translateY(-50%) scale(calc(1 / var(--s)));
    transform-origin: right center;
  }
  .dock-item:hover .tip {
    opacity: 1;
  }
</style>
