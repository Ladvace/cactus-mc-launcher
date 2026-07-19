<script lang="ts">
  import { goto } from "$app/navigation";
  import { ui } from "$lib/stores/ui.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { updater } from "$lib/stores/updater.svelte";
  import type { Instance } from "$lib/types";
  import Icon from "./Icon.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";

  type Cmd = {
    id: string;
    label: string;
    hint?: string;
    icon?: string;
    instance?: Instance;
    run: () => void;
  };

  let query = $state("");
  let selected = $state(0);
  let inputEl = $state<HTMLInputElement>();

  const open = $derived(ui.commandPaletteOpen);

  function run(cmd: Cmd) {
    ui.closeCommandPalette();
    cmd.run();
  }

  const base = $derived<Cmd[]>([
    { id: "nav-home", label: "Home", hint: "Go to", icon: "home", run: () => goto("/") },
    { id: "nav-browse", label: "Browse mods", hint: "Go to", icon: "compass", run: () => goto("/browse") },
    { id: "nav-servers", label: "Servers", hint: "Go to", icon: "globe", run: () => goto("/servers") },
    { id: "nav-achievements", label: "Achievements", hint: "Go to", icon: "trophy", run: () => goto("/achievements") },
    { id: "nav-community", label: "Community", hint: "Go to", icon: "users", run: () => goto("/share") },
    { id: "nav-settings", label: "Settings", hint: "Go to", icon: "cog", run: () => goto("/settings") },
    { id: "act-create", label: "Create instance", hint: "Action", icon: "plus", run: () => ui.openCreateInstance() },
    { id: "act-accounts", label: "Accounts", hint: "Action", icon: "user", run: () => ui.openAccounts() },
    { id: "act-update", label: "Check for updates", hint: "Action", icon: "refresh", run: () => updater.check(true) },
    { id: "act-changelog", label: "What's new", hint: "Action", icon: "clock", run: () => ui.openChangelog() },
  ]);

  const instanceCmds = $derived<Cmd[]>(
    instancesStore.instances.map((instance) => ({
      id: `inst-${instance.id}`,
      label: instance.name,
      hint: `${instance.loader} ${instance.mcVersion}`,
      instance,
      run: () => goto(`/instance/${instance.id}`),
    })),
  );

  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const all = [...base, ...instanceCmds];
    if (!q) return all;
    return all
      .map((cmd) => ({ cmd, score: cmd.label.toLowerCase().indexOf(q) }))
      .filter((r) => r.score >= 0)
      .sort((a, b) => a.score - b.score)
      .map((r) => r.cmd);
  });

  // Keep the selection in range whenever the result set changes.
  $effect(() => {
    results;
    selected = 0;
  });

  // Focus the input when the palette opens.
  $effect(() => {
    if (open) {
      query = "";
      queueMicrotask(() => inputEl?.focus());
    }
  });

  function onKey(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = Math.min(selected + 1, results.length - 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const cmd = results[selected];
      if (cmd) run(cmd);
    } else if (event.key === "Escape") {
      event.preventDefault();
      ui.closeCommandPalette();
    }
  }
</script>

{#if open}
  <div class="overlay" role="button" tabindex="-1" onclick={() => ui.closeCommandPalette()} onkeydown={() => {}}>
    <div class="palette" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <div class="search">
        <Icon name="search" size={16} />
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onKey}
          placeholder="Search instances, pages, actions…"
          spellcheck="false"
        />
      </div>
      <div class="list" role="listbox" tabindex="-1">
        {#each results as cmd, i (cmd.id)}
          <button
            type="button"
            class="row"
            class:sel={i === selected}
            role="option"
            aria-selected={i === selected}
            onmouseenter={() => (selected = i)}
            onclick={() => run(cmd)}
          >
            <span class="ic">
              {#if cmd.instance}
                <InstanceIcon instance={cmd.instance} size={22} />
              {:else}
                <Icon name={cmd.icon ?? "grid"} size={16} />
              {/if}
            </span>
            <span class="label">{cmd.label}</span>
            {#if cmd.hint}<span class="hint">{cmd.hint}</span>{/if}
          </button>
        {/each}
        {#if results.length === 0}
          <p class="empty">No matches</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 400;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(2px);
  }
  .palette {
    width: min(560px, calc(100vw - 32px));
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.75rem 0.9rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
  }
  .search input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
    font: inherit;
    font-size: 0.95rem;
  }
  .list {
    padding: 0.35rem;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.5rem 0.6rem;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row.sel {
    background: var(--accent-soft, color-mix(in srgb, var(--accent) 16%, transparent));
  }
  .ic {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    color: var(--text-muted);
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    font-size: 0.72rem;
    color: var(--text-muted);
    font-family: var(--font-mono, monospace);
    flex-shrink: 0;
  }
  .empty {
    color: var(--text-muted);
    cursor: default;
    justify-content: center;
  }
</style>
