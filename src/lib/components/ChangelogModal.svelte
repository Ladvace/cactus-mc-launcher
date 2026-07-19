<script lang="ts">
  import { CHANGELOG, renderInline } from "$lib/changelog";
  import Modal from "./Modal.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();
</script>

<Modal title="What's new" {open} {onClose} width={560}>
  <div class="log">
    {#each CHANGELOG as release (release.version)}
      <section class="release">
        <div class="head">
          <h3>{release.version}</h3>
          {#if release.date}<span class="date">{release.date}</span>{/if}
        </div>
        {#each release.groups as group}
          {#if group.title}<h4 class="grp grp-{group.title.toLowerCase()}">{group.title}</h4>{/if}
          <ul>
            {#each group.items as item}
              <li>{@html renderInline(item)}</li>
            {/each}
          </ul>
        {/each}
      </section>
    {/each}
  </div>
</Modal>

<style>
  .log {
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
  }
  .release {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.35rem;
  }
  h3 {
    margin: 0;
    font-size: 1.15rem;
    color: var(--accent);
  }
  .date {
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  .grp {
    margin: 0.5rem 0 0.1rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .grp-added {
    color: var(--success, #3fb950);
  }
  .grp-fixed {
    color: var(--warning, #ffb454);
  }
  ul {
    margin: 0.2rem 0 0;
    padding-left: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  li {
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--text-secondary);
  }
  li :global(strong) {
    color: var(--text);
  }
  li :global(code) {
    font-family: var(--font-mono, monospace);
    font-size: 0.82em;
    background: color-mix(in srgb, var(--text) 10%, transparent);
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
  }
</style>
