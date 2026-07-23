<script lang="ts" generics="T extends string = string">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import { t } from "$lib/i18n";

  type Option = { value: T; label: string; disabled?: boolean; muted?: boolean };

  interface Props {
    value: T;
    options: Option[];
    onchange?: (value: T) => void;
    disabled?: boolean;
    placeholder?: string;
    ariaLabel?: string;
    /** Show a filter box at the top of the menu (useful for long lists). */
    searchable?: boolean;
    /** Extra control rendered on the search row (e.g. a filter toggle). */
    header?: Snippet;

    width?: string;
    id?: string;
  }
  let {
    value = $bindable(),
    options,
    onchange,
    disabled = false,
    placeholder = "",
    ariaLabel,
    searchable = false,
    header,
    width = "100%",
    id,
  }: Props = $props();

  let open = $state(false);
  let dropUp = $state(false);
  let root = $state<HTMLElement>();
  let query = $state("");

  const selected = $derived(options.find((o) => o.value === value));
  const label = $derived(selected?.label ?? placeholder);

  const filtered = $derived(
    searchable && query.trim()
      ? options.filter((o) => o.label.toLowerCase().includes(query.trim().toLowerCase()))
      : options
  );

  function toggle() {
    if (disabled) return;
    if (!open) {

      const rect = root?.getBoundingClientRect();
      if (rect) {
        const below = window.innerHeight - rect.bottom;
        dropUp = below < Math.min(260, options.length * 36 + 12) && rect.top > below;
      }
    }
    open = !open;
    if (open) {
      query = "";
      queueMicrotask(() => {
        if (searchable) {
          root?.querySelector<HTMLInputElement>(".sel-search")?.focus();
        } else {
          const sel =
            root?.querySelector<HTMLButtonElement>(".opt[aria-selected='true']") ??
            root?.querySelector<HTMLButtonElement>(".opt");
          sel?.focus();
          sel?.scrollIntoView({ block: "nearest" });
        }
      });
    }
  }

  function optionButtons(): HTMLButtonElement[] {
    return [...(root?.querySelectorAll<HTMLButtonElement>(".opt:not(:disabled)") ?? [])];
  }
  function focusOptionAt(opts: HTMLButtonElement[], idx: number) {
    const el = opts[Math.max(0, Math.min(idx, opts.length - 1))];
    el?.focus();
    el?.scrollIntoView({ block: "nearest" });
  }
  function focusFirstOption() {
    focusOptionAt(optionButtons(), 0);
  }

  function onSearchKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      open = false;
      root?.querySelector<HTMLButtonElement>(".trigger")?.focus();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      focusFirstOption();
    } else if (event.key === "Enter") {
      event.preventDefault();
      const first = filtered.find((o) => !o.disabled);
      if (first) choose(first);
    }
  }

  function choose(option: Option) {
    if (option.disabled) return;
    open = false;
    root?.querySelector<HTMLButtonElement>(".trigger")?.focus();
    if (option.value !== value) {
      value = option.value;
      onchange?.(option.value);
    }
  }

  function onTriggerKey(event: KeyboardEvent) {
    if (disabled) return;
    if (event.key === "Escape") {
      open = false;
    } else if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Enter" || event.key === " ")) {
      event.preventDefault();
      toggle();
    }
  }

  function onMenuKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      open = false;
      root?.querySelector<HTMLButtonElement>(".trigger")?.focus();
      return;
    }
    const opts = optionButtons();
    if (opts.length === 0) return;
    const idx = opts.findIndex((o) => o === document.activeElement);
    const here = idx < 0 ? 0 : idx;
    const PAGE = 8;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusOptionAt(opts, here + 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        // From the top option, arrow up returns to the search box (if any).
        if (idx <= 0 && searchable) root?.querySelector<HTMLInputElement>(".sel-search")?.focus();
        else focusOptionAt(opts, here - 1);
        break;
      case "Home":
        event.preventDefault();
        focusOptionAt(opts, 0);
        break;
      case "End":
        event.preventDefault();
        focusOptionAt(opts, opts.length - 1);
        break;
      case "PageDown":
        event.preventDefault();
        focusOptionAt(opts, here + PAGE);
        break;
      case "PageUp":
        event.preventDefault();
        focusOptionAt(opts, here - PAGE);
        break;
    }
  }
</script>

<svelte:window
  onclick={(event) => {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }}
/>

<div
  class="sel"
  class:up={dropUp}
  class:disabled
  class:spotlight={open && searchable}
  bind:this={root}
  style={`width:${width}`}
>
  {#if open && searchable}
    <button
      type="button"
      class="sel-backdrop"
      aria-label={t("common.close")}
      onclick={() => (open = false)}
    ></button>
  {/if}
  <button
    type="button"
    class="trigger"
    {id}
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={ariaLabel}
    onclick={toggle}
    onkeydown={onTriggerKey}
  >
    <span class="label" class:placeholder={!selected}>{label}</span>
    <span class="chev" class:flip={open}><Icon name="chevron-down" size={12} /></span>
  </button>

  {#if open}
    <div class="menu">
      {#if searchable}
        <div class="sel-head">
          <input
            class="sel-search"
            type="text"
            placeholder={t("home.search")}
            bind:value={query}
            onkeydown={onSearchKey}
            aria-label={t("home.search")}
            autocomplete="off"
            spellcheck="false"
          />
          {#if header}<div class="sel-head-extra">{@render header()}</div>{/if}
        </div>
      {/if}
      <ul class="list" role="listbox" tabindex="-1" onkeydown={onMenuKey}>
        {#each filtered as option (option.value)}
          <li>
            <button
              type="button"
              class="opt"
              role="option"
              aria-selected={option.value === value}
              class:sel={option.value === value}
              class:muted={option.muted}
              disabled={option.disabled}
              onclick={() => choose(option)}
            >
              <span class="opt-label">{option.label}</span>
              {#if option.value === value}<span class="check"><Icon name="check" size={12} /></span>{/if}
            </button>
          </li>
        {/each}
        {#if filtered.length === 0}
          <li class="empty">{t("palette.noMatches")}</li>
        {/if}
      </ul>
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: inline-block;
    vertical-align: middle;
  }
  /* When a searchable dropdown is open, lift the whole control above the
     full-viewport blur backdrop so the trigger + menu stay crisp. */
  .sel.spotlight {
    z-index: 120;
  }
  .sel-backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    padding: 0;
    border: none;
    background: rgba(0, 0, 0, 0.32);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    cursor: default;
    animation: sel-backdrop-in 0.12s ease;
  }
  @keyframes sel-backdrop-in {
    from {
      opacity: 0;
    }
  }
  /* Honour "reduce transparency": drop the blur, keep a stronger plain dim. */
  :global([data-reduce-transparency="true"]) .sel-backdrop {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    background: rgba(0, 0, 0, 0.55);
  }
  .spotlight .trigger {
    position: relative;
    z-index: 1;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 11px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text);
    font-size: 13px;
    font-family: inherit;
    line-height: 1.2;
    text-align: left;
    cursor: pointer;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
    transition: border-color 0.12s;
  }
  .trigger:hover:not(:disabled),
  .trigger:focus-visible {
    border-color: var(--accent);
    outline: none;
  }
  .disabled .trigger {
    opacity: 0.55;
    cursor: default;
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .label.placeholder {
    color: var(--text-muted);
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
    background: var(--bg-raised, var(--bg-card));
    border: 2px solid var(--border);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5);
  }
  .up .menu {
    top: auto;
    bottom: calc(100% + 4px);
  }
  .sel-head {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-input);
    border-bottom: 2px solid var(--border);
  }
  .sel-head:focus-within {
    border-bottom-color: var(--accent);
  }
  .sel-head-extra {
    flex-shrink: 0;
    padding-right: 8px;
  }
  .sel-search {
    flex: 1;
    min-width: 0;
    padding: 9px 10px;
    background: transparent;
    border: none;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    box-sizing: border-box;
  }
  .sel-search:focus {
    outline: none;
  }
  .list {
    margin: 0;
    padding: 4px;
    max-height: 240px;
    overflow-y: auto;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .empty {
    padding: 10px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: 2px solid transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .opt:hover:not(:disabled),
  .opt:focus-visible {
    background: var(--bg-hover);
    border-color: var(--accent);
    outline: none;
  }
  /* Secondary entries (e.g. snapshots) — indented and dimmed so they read as
     sitting "under" the main stable versions. Full opacity on hover/focus. */
  .opt.muted {
    padding-left: 24px;
    opacity: 0.55;
  }
  .opt.muted:hover:not(:disabled),
  .opt.muted:focus-visible {
    opacity: 1;
  }
  .opt.sel {
    color: var(--accent);
  }
  .opt:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .opt-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .check {
    display: inline-flex;
    color: var(--accent);
  }
</style>
