<script lang="ts" generics="T extends string = string">
  import Icon from "./Icon.svelte";

  type Option = { value: T; label: string; disabled?: boolean };

  interface Props {
    value: T;
    options: Option[];
    onchange?: (value: T) => void;
    disabled?: boolean;
    placeholder?: string;
    ariaLabel?: string;

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
    width = "100%",
    id,
  }: Props = $props();

  let open = $state(false);
  let dropUp = $state(false);
  let root = $state<HTMLElement>();

  const selected = $derived(options.find((o) => o.value === value));
  const label = $derived(selected?.label ?? placeholder);

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
      queueMicrotask(() =>
        root?.querySelector<HTMLButtonElement>(".opt[aria-selected='true'], .opt")?.focus()
      );
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
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const opts = [...(root?.querySelectorAll<HTMLButtonElement>(".opt:not(:disabled)") ?? [])];
      const idx = opts.findIndex((o) => o === document.activeElement);
      const next = event.key === "ArrowDown" ? Math.min(idx + 1, opts.length - 1) : Math.max(idx - 1, 0);
      opts[next < 0 ? 0 : next]?.focus();
    }
  }
</script>

<svelte:window
  onclick={(event) => {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }}
/>

<div class="sel" class:up={dropUp} class:disabled bind:this={root} style={`width:${width}`}>
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
    <ul class="menu" role="listbox" tabindex="-1" onkeydown={onMenuKey}>
      {#each options as option (option.value)}
        <li>
          <button
            type="button"
            class="opt"
            role="option"
            aria-selected={option.value === value}
            class:sel={option.value === value}
            disabled={option.disabled}
            onclick={() => choose(option)}
          >
            <span class="opt-label">{option.label}</span>
            {#if option.value === value}<span class="check"><Icon name="check" size={12} /></span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: inline-block;
    vertical-align: middle;
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
    padding: 4px;
    max-height: 260px;
    overflow-y: auto;
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
