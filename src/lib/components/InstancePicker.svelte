<script lang="ts">
  import InstanceIcon from "./InstanceIcon.svelte";
  import Icon from "./Icon.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import type { Instance } from "$lib/types";

  let {
    value = $bindable(""),
    placeholder = "Select an instance",
  }: { value?: string; placeholder?: string } = $props();

  let open = $state(false);
  let query = $state("");
  let rootEl = $state<HTMLElement>();
  let triggerEl = $state<HTMLButtonElement>();
  let searchEl = $state<HTMLInputElement>();
  // Fixed-position anchor for the dropdown so it doesn't inflate a scrollable
  // parent (e.g. a modal body) and cause a second scrollbar.
  let pos = $state<{ left: number; top: number; width: number } | null>(null);

  function place() {
    if (!triggerEl) return;
    const r = triggerEl.getBoundingClientRect();
    pos = { left: r.left, top: r.bottom + 4, width: r.width };
  }
  function toggle() {
    if (open) {
      open = false;
    } else {
      place();
      open = true;
    }
  }

  const all = $derived(instancesStore.instances);
  const selected = $derived(all.find((i) => i.id === value) ?? null);

  function matches(i: Instance, q: string): boolean {
    if (!q) return true;
    return (
      i.name.toLowerCase().includes(q) ||
      i.mcVersion.toLowerCase().includes(q) ||
      i.loader.includes(q)
    );
  }

  // Filtered view split into ungrouped instances + one section per folder,
  // mirroring how instances are grouped on Home.
  const view = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const ungrouped: Instance[] = [];
    const byGroup = new Map<string, Instance[]>();
    for (const i of all) {
      if (!matches(i, q)) continue;
      if (i.group) {
        (byGroup.get(i.group) ?? byGroup.set(i.group, []).get(i.group)!).push(i);
      } else {
        ungrouped.push(i);
      }
    }
    return { ungrouped, groups: [...byGroup.entries()] };
  });

  const isEmpty = $derived(view.ungrouped.length === 0 && view.groups.length === 0);

  // Focus the search box when the dropdown opens.
  $effect(() => {
    if (open) searchEl?.focus();
  });

  function choose(id: string) {
    value = id;
    open = false;
    query = "";
  }
  function onWindowPointerDown(e: PointerEvent) {
    if (open && rootEl && !rootEl.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window
  onpointerdown={onWindowPointerDown}
  onkeydown={(e) => e.key === "Escape" && (open = false)}
  onresize={() => (open = false)}
/>

{#snippet row(i: Instance)}
  <button type="button" class="opt" class:sel={i.id === value} onclick={() => choose(i.id)}>
    <InstanceIcon instance={i} size={22} />
    <span class="opt-name">{i.name}</span>
    <span class="opt-sub">{i.loader} {i.mcVersion}</span>
  </button>
{/snippet}

<div class="picker" bind:this={rootEl}>
  <button type="button" class="trigger" class:open bind:this={triggerEl} onclick={toggle}>
    {#if selected}
      <InstanceIcon instance={selected} size={22} />
      <span class="t-name">{selected.name}</span>
      <span class="t-sub">{selected.loader} {selected.mcVersion}</span>
    {:else}
      <span class="t-name muted">{placeholder}</span>
    {/if}
    <span class="caret"></span>
  </button>

  {#if open && pos}
    <div
      class="pop"
      style="left:{pos.left}px; top:{pos.top}px; width:{pos.width}px;"
    >
      <div class="search">
        <Icon name="search" size={14} />
        <input
          bind:this={searchEl}
          bind:value={query}
          placeholder="Search instances…"
          spellcheck="false"
        />
      </div>
      <div class="list">
        {#if isEmpty}
          <p class="none">No instances match.</p>
        {:else}
          {#each view.ungrouped as i (i.id)}
            {@render row(i)}
          {/each}
          {#each view.groups as [name, items] (name)}
            <div class="group-head"><Icon name="folder" size={11} /> {name}</div>
            {#each items as i (i.id)}
              {@render row(i)}
            {/each}
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    width: 100%;
  }
  /* Match the recessed .select look. */
  .trigger {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
    text-align: left;
  }
  .trigger.open,
  .trigger:hover {
    border-color: var(--accent);
  }
  .t-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .t-sub {
    color: var(--text-muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .muted {
    color: var(--text-muted);
    font-weight: 400;
  }
  .caret {
    margin-left: auto;
    width: 0;
    height: 0;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid var(--text-muted);
  }

  .pop {
    position: fixed;
    z-index: 200;
    background: var(--bg-raised);
    border: 2px solid var(--border);
    box-shadow: var(--shadow-md);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 2px solid var(--border-subtle);
  }
  .search :global(.hn) {
    color: var(--text-muted);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 13px;
  }
  .search input:focus {
    outline: none;
  }
  .list {
    max-height: 240px;
    overflow-y: auto;
    padding: 4px;
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 8px 8px 4px;
    font-family: var(--font-pixel);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  .group-head :global(.hn) {
    color: var(--text-muted);
  }
  .opt {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    text-align: left;
  }
  .opt:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .opt.sel {
    color: var(--accent);
  }
  .opt-name {
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opt-sub {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 11.5px;
    color: var(--text-muted);
  }
  .none {
    padding: 14px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12.5px;
    margin: 0;
  }
</style>
