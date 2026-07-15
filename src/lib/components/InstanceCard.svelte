<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "./Icon.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import LoaderIcon from "./LoaderIcon.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { MOD_LOADERS, type Instance } from "$lib/types";

  interface Props {
    instance: Instance;
    iconSize?: number;
    fill?: boolean;
  }
  let { instance, iconSize = 72, fill = false }: Props = $props();

  const loaderLabel = $derived(
    MOD_LOADERS.find((l) => l.value === instance.loader)?.label ?? instance.loader
  );
  const busy = $derived(launchStore.isBusy(instance.id));
  const running = $derived(launchStore.isRunning(instance.id));

  function open() {
    goto(`/instance/${instance.id}`);
  }

  function play(e: MouseEvent) {
    e.stopPropagation();
    if (running) {
      launchStore.stop(instance.id);
    } else if (!busy) {
      launchStore.launch(instance.id);
    }
  }
</script>

<div
  class="card"
  class:fill
  role="button"
  tabindex="0"
  onclick={open}
  onkeydown={(e) => e.key === "Enter" && open()}
>
  <div class="art">
    <InstanceIcon {instance} size={iconSize} />
    <button
      class="play"
      class:visible={busy || running}
      class:is-running={running}
      onclick={play}
      aria-label={running ? "Stop" : "Play"}
      title={running ? "Running" : busy ? "Preparing…" : "Play"}
    >
      {#if busy}
        <span class="spinner"></span>
      {:else if running}
        <Icon name="stop" size={16} />
      {:else}
        <Icon name="play" size={18} />
      {/if}
    </button>
  </div>
  <div class="meta">
    <span class="name" title={instance.name}>{instance.name}</span>
    <span class="sub">
      <LoaderIcon loader={instance.loader} size={13} />
      {loaderLabel} · {instance.mcVersion}
    </span>
  </div>
</div>

<style>
  .card {
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-radius: 0;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    transition: border-color 0.12s, transform 0.12s, background 0.12s;
  }
  .card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    transform: translateY(-2px);
  }
  /* When filling a grid tile, grow the art to center the (larger) icon and
     keep the meta pinned to the bottom. */
  .card.fill {
    height: 100%;
  }
  .card.fill .art {
    flex: 1;
    padding: 0;
  }
  .art {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 0;
  }
  .play {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 34px;
    height: 34px;
    border-radius: 0;
    border: 2px solid #b98a1e;
    background: var(--accent);
    color: var(--accent-contrast);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transform: translateY(4px);
    transition: opacity 0.12s, transform 0.12s;
    box-shadow: 0 3px 0 rgba(0, 0, 0, 0.4),
      inset 1px 1px 0 rgba(255, 255, 255, 0.4);
  }
  .card:hover .play,
  .play.visible {
    opacity: 1;
    transform: translateY(0);
  }
  .play.is-running {
    background: var(--danger);
    color: #fff;
  }
  .spinner {
    width: 15px;
    height: 15px;
    border: 2px solid rgba(255, 255, 255, 0.35);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
