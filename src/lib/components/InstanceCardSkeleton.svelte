<div class="card">
  <div class="art"><span class="skeleton icon"></span></div>
  <div class="meta">
    <span class="skeleton" style="width:72%;height:14px"></span>
    <span class="skeleton" style="width:48%;height:11px"></span>
  </div>
</div>

<style>
  .card {
    background: var(--bg-card);
    border: 2px solid var(--border);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
  }
  .art {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 0;
  }
  .icon {
    width: 72px;
    height: 72px;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
