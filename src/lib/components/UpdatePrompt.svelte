<script lang="ts">
  import { onMount } from "svelte";
  import { updater } from "$lib/stores/updater.svelte";
  import Icon from "./Icon.svelte";

  // Silent check once on launch (a missing endpoint / offline just no-ops).
  onMount(() => updater.check(false));
</script>

{#if updater.phase !== "idle" && updater.update}
  <div class="update" role="status">
    {#if updater.phase === "available"}
      <div class="row">
        <Icon name="download" size={16} />
        <div class="text">
          <strong>Update available</strong>
          <span class="ver">v{updater.update.version}</span>
        </div>
      </div>
      <div class="actions">
        <button class="btn ghost sm" onclick={() => updater.dismiss()}>Later</button>
        <button class="btn primary sm" onclick={() => updater.install()}>Install &amp; restart</button>
      </div>
    {:else}
      <div class="row">
        <span class="spinner" aria-hidden="true"></span>
        <div class="text">
          <strong>{updater.phase === "installing" ? "Installing…" : "Downloading update…"}</strong>
          {#if updater.phase === "downloading" && updater.pct !== null}
            <span class="ver">{updater.pct}%</span>
          {/if}
        </div>
      </div>
      {#if updater.phase === "downloading"}
        <div class="bar"><div class="fill" style="width:{updater.pct ?? 0}%"></div></div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .update {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 900;
    width: min(320px, calc(100vw - 36px));
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.85rem 1rem;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius, 10px);
    box-shadow: 0 12px 34px rgba(0, 0, 0, 0.4);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .text {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .ver {
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 12%, transparent);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }
  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid color-mix(in srgb, var(--accent) 25%, transparent);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: update-spin 0.7s linear infinite;
  }
  @keyframes update-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
