<script lang="ts">
  import { LOCALES, type LocaleCode } from "$lib/i18n";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import Flag from "./Flag.svelte";
  import Icon from "./Icon.svelte";

  let { direction = "down" }: { direction?: "down" | "up" } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();

  const current = $derived(
    LOCALES.find((l) => l.code === settingsStore.settings.language) ?? LOCALES[0]
  );

  function choose(code: LocaleCode) {
    open = false;
    root?.querySelector<HTMLButtonElement>(".trigger")?.focus();
    if (code !== settingsStore.settings.language) {
      settingsStore.save({ ...settingsStore.settings, language: code });
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.stopPropagation();
      open = false;
      return;
    }
    if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      open = true;
      queueMicrotask(() =>
        root?.querySelector<HTMLButtonElement>(".opt[aria-selected='true']")?.focus()
      );
      return;
    }
    if (open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      const opts = [...(root?.querySelectorAll<HTMLButtonElement>(".opt") ?? [])];
      const idx = opts.findIndex((o) => o === document.activeElement);
      const next =
        event.key === "ArrowDown"
          ? Math.min(idx + 1, opts.length - 1)
          : Math.max(idx - 1, 0);
      opts[next < 0 ? 0 : next]?.focus();
    }
  }
</script>

<svelte:window
  onclick={(event) => {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }}
/>

<div class="lang" class:up={direction === "up"} bind:this={root}>
  <button
    type="button"
    class="trigger"
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open = !open)}
    onkeydown={onKeydown}
  >
    <Flag code={current.code} />
    <span class="name">{current.label}</span>
    <span class="chev" class:flip={open}><Icon name="chevron-down" size={12} /></span>
  </button>

  {#if open}
    <ul class="menu" role="listbox" tabindex="-1" onkeydown={onKeydown}>
      {#each LOCALES as locale (locale.code)}
        <li>
          <button
            type="button"
            class="opt"
            role="option"
            aria-selected={locale.code === current.code}
            class:sel={locale.code === current.code}
            onclick={() => choose(locale.code)}
          >
            <Flag code={locale.code} />
            <span class="name">{locale.label}</span>
            {#if locale.code === current.code}<span class="check">✓</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .lang {
    position: relative;
    display: inline-block;
    min-width: 172px;
  }
  .trigger,
  .opt {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
    transition: border-color 0.12s, color 0.12s, background 0.12s;
  }
  .trigger:hover,
  .trigger:focus-visible {
    border-color: var(--accent);
    outline: none;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chev {
    display: inline-flex;
    align-items: center;
    color: var(--text-muted);
    transition: transform 0.15s ease;
  }
  .chev.flip {
    transform: rotate(180deg);
  }
  .menu {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    z-index: 60;
    margin: 0;
    padding: 4px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--bg-raised, var(--bg-card));
    border: 2px solid var(--border);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5);
  }
  .up .menu {
    top: auto;
    bottom: calc(100% + 4px);
  }
  .opt {
    border: 2px solid transparent;
    box-shadow: none;
    background: transparent;
  }
  .opt:hover,
  .opt:focus-visible {
    background: var(--bg-hover);
    border-color: var(--accent);
    outline: none;
  }
  .opt.sel {
    color: var(--accent);
  }
  .check {
    color: var(--accent);
    font-size: 12px;
  }
</style>
