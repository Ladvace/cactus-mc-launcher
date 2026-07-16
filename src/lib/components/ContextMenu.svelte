<script lang="ts" module>
  export interface MenuItem {
    label?: string;
    icon?: string;
    danger?: boolean;
    disabled?: boolean;
    separator?: boolean;
    onSelect?: () => void;
  }
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    x: number;
    y: number;
    items: MenuItem[];
    onClose: () => void;
  }
  let { x, y, items, onClose }: Props = $props();

  let menuEl = $state<HTMLDivElement>();
  let pos = $state<{ x: number; y: number } | null>(null);

  // Measure while hidden, then nudge the menu so it stays inside the viewport.
  $effect(() => {
    const el = menuEl;
    if (!el) return;
    const r = el.getBoundingClientRect();
    let nx = x;
    let ny = y;
    if (x + r.width > window.innerWidth - 8) nx = window.innerWidth - r.width - 8;
    if (y + r.height > window.innerHeight - 8)
      ny = window.innerHeight - r.height - 8;
    pos = { x: Math.max(8, nx), y: Math.max(8, ny) };
  });

  function select(item: MenuItem) {
    if (item.disabled || item.separator) return;
    item.onSelect?.();
    onClose();
  }

  function onWindowPointerDown(e: PointerEvent) {
    if (menuEl && !menuEl.contains(e.target as Node)) onClose();
  }
</script>

<svelte:window
  onpointerdown={onWindowPointerDown}
  onkeydown={(e) => e.key === "Escape" && onClose()}
  onwheel={onClose}
  onresize={onClose}
/>

<div
  bind:this={menuEl}
  class="menu"
  style="left:{pos?.x ?? x}px; top:{pos?.y ?? y}px; visibility:{pos
    ? 'visible'
    : 'hidden'};"
  role="menu"
  tabindex="-1"
>
  {#each items as item}
    {#if item.separator}
      <div class="sep"></div>
    {:else}
      <button
        class="item"
        class:danger={item.danger}
        disabled={item.disabled}
        role="menuitem"
        onclick={() => select(item)}
      >
        {#if item.icon}<Icon name={item.icon} size={14} />{/if}
        <span>{item.label}</span>
      </button>
    {/if}
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 200;
    min-width: 176px;
    padding: 5px;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: var(--shadow-md),
      inset 2px 2px 0 rgba(255, 255, 255, 0.05),
      inset -2px -2px 0 rgba(0, 0, 0, 0.3);
    animation: pop 0.1s ease;
  }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    text-align: left;
  }
  .item :global(.hn) {
    color: var(--text-muted);
  }
  .item:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text);
  }
  .item:hover:not(:disabled) :global(.hn) {
    color: var(--accent);
  }
  .item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .item.danger {
    color: var(--danger);
  }
  .item.danger:hover:not(:disabled) {
    background: rgba(255, 91, 110, 0.12);
    color: var(--danger);
  }
  .item.danger:hover:not(:disabled) :global(.hn) {
    color: var(--danger);
  }
  .sep {
    height: 2px;
    margin: 5px 4px;
    background: var(--border-subtle);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
</style>
