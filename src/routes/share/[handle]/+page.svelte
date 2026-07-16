<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import BoardView from "$lib/components/BoardView.svelte";

  const handle = $derived($page.params.handle);
</script>

<div class="page">
  <button class="back" onclick={() => goto("/share")}>← Community</button>
  {#if handle}
    <BoardView {handle} />
  {/if}
</div>

<style>
  .page {
    padding: 24px 32px;
    max-width: 900px;
    margin: 0 auto;
  }
  .back {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    padding: 4px 0;
    margin-bottom: 12px;
  }
  .back:hover {
    color: var(--accent);
  }
</style>
