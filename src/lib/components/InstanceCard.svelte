<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "./Icon.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import LoaderIcon from "./LoaderIcon.svelte";
  import { launchStore } from "$lib/stores/launch.svelte";
  import { installStore } from "$lib/stores/install.svelte";
  import { ui } from "$lib/stores/ui.svelte";
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
  // Cover mode: the icon fills the whole tile behind the label.
  const cover = $derived(instance.coverImage && !!instance.icon);
  // Modpack download in progress.
  const installing = $derived(installStore.isInstalling(instance.id));
  const installPct = $derived(installStore.pct(instance.id));
  const installMsg = $derived(installStore.progressFor(instance.id)?.message ?? "");

  function open() {
    goto(`/instance/${instance.id}`);
  }

  function contextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation(); // don't also open the page-level menu
    ui.openInstanceMenu(instance, e.clientX, e.clientY);
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

{#snippet playButton()}
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
{/snippet}

{#snippet meta()}
  <span class="name" title={instance.name}>{instance.name}</span>
  <span class="sub">
    <LoaderIcon loader={instance.loader} size={13} />
    {loaderLabel} · {instance.mcVersion}
  </span>
{/snippet}

<div
  class="card"
  class:fill
  class:cover
  role="button"
  tabindex="0"
  onclick={open}
  oncontextmenu={contextMenu}
  onkeydown={(e) => e.key === "Enter" && open()}
>
  {#if installing}
    <div class="install-overlay">
      <span class="dl-spinner"></span>
      {#if installPct !== null}
        <span class="dl-pct">{installPct}%</span>
      {/if}
      <span class="dl-msg">{installMsg || "Downloading…"}</span>
    </div>
  {/if}
  {#if instance.kind === "server"}
    <span class="kind-badge" title="Dedicated server">SERVER</span>
  {/if}
  {#if cover}
    <img class="cover-img" src={instance.icon} alt={instance.name} />
    <div class="cover-scrim"></div>
    {@render playButton()}
    <div class="meta on-cover">{@render meta()}</div>
  {:else}
    <div class="art">
      <InstanceIcon {instance} size={iconSize} />
    </div>
    {@render playButton()}
    <div class="meta">{@render meta()}</div>
  {/if}
</div>

<style>
  .card {
    position: relative;
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
  /* Marks a dedicated-server tile. */
  .kind-badge {
    position: absolute;
    top: 6px;
    left: 6px;
    z-index: 3;
    padding: 2px 6px;
    font-family: var(--font-pixel);
    font-size: 8px;
    letter-spacing: 0.08em;
    color: var(--bg-app);
    background: var(--accent);
    pointer-events: none;
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
  /* Cover mode: full-bleed icon behind an overlaid label. */
  .card.cover {
    padding: 0;
    overflow: hidden;
    min-height: 140px;
    justify-content: flex-end;
  }
  .cover-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    image-rendering: pixelated;
  }
  .cover-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to top,
      rgba(0, 0, 0, 0.8) 0%,
      rgba(0, 0, 0, 0.35) 32%,
      rgba(0, 0, 0, 0) 60%
    );
    pointer-events: none;
  }
  .meta.on-cover {
    position: relative;
    z-index: 1;
    padding: 10px 12px;
  }
  .meta.on-cover .name {
    color: #fff;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
  }
  .meta.on-cover .sub {
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
  }
  /* Modpack download overlay — visible on the tile while installing. */
  .install-overlay {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px;
    background: rgba(10, 9, 8, 0.82);
    text-align: center;
  }
  .dl-spinner {
    width: 30px;
    height: 30px;
    border: 3px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  .dl-pct {
    font-family: var(--font-pixel);
    font-size: 16px;
    color: var(--accent);
  }
  .dl-msg {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
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
    top: 8px;
    right: 8px;
    z-index: 4;
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
