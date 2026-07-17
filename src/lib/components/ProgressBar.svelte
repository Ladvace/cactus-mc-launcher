<script lang="ts">
  interface Props {
    label: string;
    /** 0–100 for a determinate bar, or null for an indeterminate one. */
    pct: number | null;
  }
  let { label, pct }: Props = $props();
</script>

<div class="progress-head">
  <span>{label}</span>
  {#if pct !== null}<span>{pct}%</span>{/if}
</div>
<div class="bar">
  <div
    class="bar-fill"
    class:indeterminate={pct === null}
    style={pct !== null ? `width:${pct}%` : ""}
  ></div>
</div>

<style>
  .progress-head {
    display: flex;
    justify-content: space-between;
    font-size: 12.5px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .bar {
    height: 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    overflow: hidden;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.3);
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    background-image: repeating-linear-gradient(
      90deg,
      rgba(0, 0, 0, 0.18) 0 2px,
      transparent 2px 8px
    );
    transition: width 0.2s steps(16);
  }
  .bar-fill.indeterminate {
    width: 35%;
    animation: slide 1.1s steps(8) infinite;
  }
  @keyframes slide {
    0% {
      margin-left: -35%;
    }
    100% {
      margin-left: 100%;
    }
  }
</style>
