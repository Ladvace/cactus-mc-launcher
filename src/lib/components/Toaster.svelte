<script lang="ts">
  import Icon from "./Icon.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { t } from "$lib/i18n";

  // Newest first — index 0 is the front card (bottom of the stack).
  const list = $derived([...toast.toasts].reverse());

  let heights = $state<Record<number, number>>({});
  let expanded = $state(false);

  // Drop measured heights for toasts that have been dismissed (ids are
  // monotonic, so the map would otherwise grow for the whole session).
  $effect(() => {
    const alive = new Set(toast.toasts.map((notification) => notification.id));
    for (const key of Object.keys(heights)) {
      if (!alive.has(Number(key))) delete heights[Number(key)];
    }
  });

  const OFFSET = 14; // collapsed peek per card
  const GAP = 12; // expanded gap between cards
  const SCALE_STEP = 0.05;
  const MAX_VISIBLE = 3; // collapsed cards shown before fully hidden

  const height = (id: number) => heights[id] ?? 60;

  // Total height below card i when expanded (sum of the cards in front of it).
  function stackBelow(i: number): number {
    let sum = 0;
    for (let j = 0; j < i; j++) sum += height(list[j].id) + GAP;
    return sum;
  }

  function styleFor(i: number): string {
    const z = list.length - i;
    if (expanded) {
      return `transform: translateY(${-stackBelow(i)}px) scale(1); opacity: 1; z-index: ${z};`;
    }
    const scale = Math.max(1 - i * SCALE_STEP, 0.82);
    const opacity = i < MAX_VISIBLE ? 1 : 0;
    return `transform: translateY(${-i * OFFSET}px) scale(${scale}); opacity: ${opacity}; z-index: ${z}; pointer-events: ${i === 0 ? "auto" : "none"};`;
  }

  const boxHeight = $derived.by(() => {
    if (list.length === 0) return 0;
    if (expanded) {
      return list.reduce((sum, n) => sum + height(n.id) + GAP, 0);
    }
    return height(list[0].id) + Math.min(list.length - 1, MAX_VISIBLE - 1) * OFFSET;
  });

  let copied = $state<number | null>(null);
  async function copy(id: number, message: string) {
    try {
      await navigator.clipboard.writeText(message);
      copied = id;
      setTimeout(() => copied === id && (copied = null), 1500);
    } catch {
    }
  }
</script>

<div
  class="toaster"
  class:none={list.length === 0}
  style="height:{boxHeight}px"
  role="region"
  aria-label="Notifications"
  onmouseenter={() => {
    expanded = true;
    toast.pause();
  }}
  onmouseleave={() => {
    expanded = false;
    toast.resume();
  }}
>
  {#each list as notification, i (notification.id)}
    <div
      class="toast {notification.kind}"
      class:expanded
      class:front={i === 0}
      role={notification.kind === "error" ? "alert" : "status"}
      style={styleFor(i)}
      bind:clientHeight={heights[notification.id]}
    >
      <span class="badge">
        {#if notification.kind === "success"}
          <Icon name="check" size={13} />
        {:else if notification.kind === "error"}
          !
        {:else}
          <Icon name="clock" size={13} />
        {/if}
      </span>
      <span class="msg">{notification.message}</span>
      <div class="actions">
        {#if notification.action}
          <button class="act" onclick={() => toast.run(notification.id, notification.action!)}>
            {notification.action.label}
          </button>
        {/if}
        {#if notification.kind === "error"}
          <button class="act" onclick={() => copy(notification.id, notification.message)}>
            {copied === notification.id ? t("toaster.copied") : t("toaster.copy")}
          </button>
        {/if}
        <button class="close" aria-label={t("toaster.dismiss")} onclick={() => toast.dismiss(notification.id)}>✕</button>
      </div>
    </div>
  {/each}
</div>

<style>
  .toaster {
    position: fixed;
    right: 18px;
    bottom: 100px;
    z-index: 400;
    width: min(380px, calc(100vw - 36px));
    pointer-events: auto;
  }
  .toaster.none {
    pointer-events: none;
  }
  .toast {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 11px 12px;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    box-shadow: var(--shadow-md);
    transform-origin: bottom center;
    transition: transform 0.34s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.28s ease;
  }
  .toast.success {
    border-color: var(--accent);
  }
  .toast.error {
    border-color: var(--danger);
  }
  .badge {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-pixel);
    font-weight: 700;
  }
  .toast.success .badge {
    color: var(--accent);
  }
  .toast.error .badge {
    color: var(--danger);
  }
  .toast.info .badge {
    color: var(--text-muted);
  }
  .msg {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--text);
    line-height: 1.4;
    word-break: break-word;
    max-height: 8em;
    overflow-y: auto;
    user-select: text;
  }
  /* Only the front card is readable when collapsed; clamp the rest. */
  .toast:not(.expanded):not(.front) .msg {
    max-height: 2.8em;
    overflow: hidden;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .act {
    padding: 3px 8px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
  }
  .act:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 13px;
    padding: 2px 4px;
  }
  .close:hover {
    color: var(--text);
  }
  /* Fade the action buttons out when collapsed (behind cards). */
  .toast:not(.expanded):not(.front) .actions {
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s;
  }
</style>
