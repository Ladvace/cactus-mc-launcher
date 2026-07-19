<script lang="ts">
  import { browser } from "$app/environment";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { toast } from "$lib/stores/toast.svelte";
  import Icon from "./Icon.svelte";

  type Phase = "idle" | "available" | "downloading" | "installing";

  let phase = $state<Phase>("idle");
  let update = $state<Update | null>(null);
  let downloaded = $state(0);
  let total = $state(0);

  const pct = $derived(total > 0 ? Math.round((downloaded / total) * 100) : null);

  // Only meaningful inside the Tauri shell (skipped in the browser / SSR / tests).
  const inTauri = browser && "__TAURI_INTERNALS__" in window;

  $effect(() => {
    if (!inTauri) return;
    let cancelled = false;
    check()
      .then((found) => {
        if (cancelled || !found) return;
        update = found;
        phase = "available";
      })
      // Silent on failure — a missing endpoint / offline shouldn't nag the user.
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  });

  async function install() {
    if (!update) return;
    try {
      phase = "downloading";
      downloaded = 0;
      total = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
        } else if (event.event === "Finished") {
          phase = "installing";
        }
      });
      await relaunch();
    } catch (err) {
      toast.error(`Update failed: ${String(err)}`);
      phase = "available";
    }
  }
</script>

{#if phase !== "idle" && update}
  <div class="update" role="status">
    {#if phase === "available"}
      <div class="row">
        <Icon name="download" size={16} />
        <div class="text">
          <strong>Update available</strong>
          <span class="ver">v{update.version}</span>
        </div>
      </div>
      <div class="actions">
        <button class="btn ghost sm" onclick={() => (phase = "idle")}>Later</button>
        <button class="btn primary sm" onclick={install}>Install &amp; restart</button>
      </div>
    {:else}
      <div class="row">
        <span class="spinner" aria-hidden="true"></span>
        <div class="text">
          <strong>{phase === "installing" ? "Installing…" : "Downloading update…"}</strong>
          {#if phase === "downloading" && pct !== null}
            <span class="ver">{pct}%</span>
          {/if}
        </div>
      </div>
      {#if phase === "downloading"}
        <div class="bar"><div class="fill" style="width:{pct ?? 0}%"></div></div>
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
