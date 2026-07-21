<script lang="ts">
  import Icon from "./Icon.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { t } from "$lib/i18n";

  let copied = $state<number | null>(null);

  async function copy(id: number, message: string) {
    try {
      await navigator.clipboard.writeText(message);
      copied = id;
      setTimeout(() => {
        if (copied === id) copied = null;
      }, 1500);
    } catch {
    }
  }
</script>

<div class="toaster">
  {#each toast.toasts as notification (notification.id)}
    <div class="toast {notification.kind}" role={notification.kind === "error" ? "alert" : "status"}>
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
    display: flex;
    flex-direction: column-reverse;
    gap: 8px;
    max-width: min(400px, calc(100vw - 36px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 11px 12px;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    box-shadow: var(--shadow-md);
    animation: toast-in 0.16s ease;
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
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateX(12px);
    }
  }
</style>
