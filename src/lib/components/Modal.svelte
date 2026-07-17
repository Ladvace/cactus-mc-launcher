<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    open: boolean;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
    width?: number;
  }
  let { title, open, onClose, children, footer, width = 480 }: Props = $props();

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={open ? onKey : undefined} />

{#if open}
  <div
    class="overlay"
    role="button"
    tabindex="-1"
    onclick={onClose}
    onkeydown={() => {}}
  >
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      style="max-width:{width}px;"
      onclick={(event) => event.stopPropagation()}
      onkeydown={() => {}}
    >
      <header class="modal-head">
        <h3>{title}</h3>
        <button class="close" onclick={onClose} aria-label="Close">✕</button>
      </header>
      <div class="modal-body">
        {@render children()}
      </div>
      {#if footer}
        <footer class="modal-foot">
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 24px;
    animation: fade 0.12s ease;
  }
  .modal {
    width: 100%;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: var(--shadow-md),
      inset 2px 2px 0 rgba(255, 255, 255, 0.05),
      inset -2px -2px 0 rgba(0, 0, 0, 0.3);
    display: flex;
    flex-direction: column;
    max-height: 90vh;
    animation: pop 0.14s ease;
  }
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .modal-head h3 {
    font-size: 16px;
  }
  .close {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 15px;
    padding: 4px 8px;
    border-radius: 0;
  }
  .close:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .modal-body {
    padding: 18px;
    overflow-y: auto;
  }
  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 18px;
    border-top: 1px solid var(--border-subtle);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
</style>
