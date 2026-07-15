<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import Icon from "./Icon.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { skinFace } from "$lib/skin";
  import { ui } from "$lib/stores/ui.svelte";
  import type { Instance } from "$lib/types";

  interface Props {
    onCreate: () => void;
  }
  let { onCreate }: Props = $props();

  const path = $derived($page.url.pathname);
  const pinned = $derived(instancesStore.instances.slice(0, 8));

  type Item =
    | { kind: "nav"; href: string; icon: string; label: string }
    | { kind: "sep" }
    | { kind: "instance"; instance: Instance; label: string }
    | { kind: "add"; label: string }
    | { kind: "settings"; href: string; label: string }
    | { kind: "account"; label: string };

  const items = $derived<Item[]>([
    { kind: "nav", href: "/", icon: "home", label: "Home" },
    { kind: "nav", href: "/browse", icon: "compass", label: "Browse" },
    { kind: "nav", href: "/library", icon: "library", label: "Library" },
    { kind: "sep" },
    ...pinned.map(
      (i): Item => ({ kind: "instance", instance: i, label: i.name })
    ),
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
    const arr: number[] = [];
    for (const it of items) {
      const w = it.kind === "sep" ? SEP : ITEM;
      arr.push(cx + w / 2);
      cx += w + GAP;
    }
    return arr;
  });

  let dockEl = $state<HTMLElement>();
  let dockLeft = 0; // captured on enter (resting frame) to avoid feedback drift
  let scales = $state<number[]>([]);
  // Throttle to one update per animation frame — mousemove can fire far more
  // often than the display refreshes, which otherwise thrashes layout.
  let mouseX = 0;
  let inside = false;
  let rafId = 0;

  function apply() {
    rafId = 0;
    if (!inside) return;
    const base = dockLeft + PAD;
    scales = centers.map((c) => {
      const d = Math.abs(mouseX - (base + c));
      return d > RANGE ? 1 : 1 + (MAX - 1) * (1 - d / RANGE);
    });
  }
  function onEnter() {
    if (dockEl) dockLeft = dockEl.getBoundingClientRect().left;
    inside = true;
  }
  function onMove(e: MouseEvent) {
    mouseX = e.clientX;
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
  function activate(item: Item) {
    if (item.kind === "nav" || item.kind === "settings") goto(item.href);
    else if (item.kind === "instance") goto(`/instance/${item.instance.id}`);
    else if (item.kind === "add") onCreate();
    else if (item.kind === "account") ui.openAccounts();
  }
</script>

<div class="dock-wrap">
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
    {#each items as item, i (i)}
      {#if item.kind === "sep"}
        <div class="dock-sep"></div>
      {:else}
        {@const active =
          (item.kind === "nav" || item.kind === "settings") &&
          isActive(item.href)}
        {@const activeInstance =
          item.kind === "instance" && path === `/instance/${item.instance.id}`}
        <button
          class="dock-item"
          class:active={active || activeInstance}
          style="--s:{scales[i] ?? 1}"
          onclick={() => activate(item)}
          aria-label={item.label}
        >
          <span class="tip">{item.label}</span>
          <span class="glyph">
            {#if item.kind === "nav" || item.kind === "settings"}
              <Icon
                name={item.kind === "settings" ? "settings" : item.icon}
                size={22}
              />
            {:else if item.kind === "add"}
              <Icon name="plus" size={22} />
            {:else if item.kind === "instance"}
              <InstanceIcon instance={item.instance} size={40} />
            {:else if item.kind === "account"}
              {#if accountsStore.active}
                <img
                  class="acc"
                  src={skinFace(accountsStore.active.uuid, 40)}
                  alt={item.label}
                />
              {:else}
                <Icon name="user" size={20} />
              {/if}
            {/if}
          </span>
          {#if active || activeInstance}<span class="dot"></span>{/if}
        </button>
      {/if}
    {/each}
  </div>
</div>

<style>
  .dock-wrap {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 14px;
    display: flex;
    justify-content: center;
    pointer-events: none;
    z-index: 50;
  }
  .dock {
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
  .dock-sep {
    width: 2px;
    align-self: stretch;
    margin: 4px 2px;
    background: var(--border);
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
    /* transform grows the icon upward without changing the dock's height;
       the margin reserves horizontal space so neighbors never overlap. */
    transform: scale(var(--s));
    transform-origin: bottom center;
    margin: 0 calc((var(--s) - 1) * 24px);
    will-change: transform, margin;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.05),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    transition: transform 0.18s ease, margin 0.18s ease, border-color 0.12s,
      color 0.12s;
  }
  .dock-item:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .dock-item.active {
    color: var(--accent);
    border-color: var(--border);
  }
  .glyph {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .acc {
    width: 40px;
    height: 40px;
    object-fit: cover;
    image-rendering: pixelated;
    border: 2px solid rgba(0, 0, 0, 0.3);
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
  .dock-item:hover .tip {
    opacity: 1;
  }
</style>
