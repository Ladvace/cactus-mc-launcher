<script lang="ts">
  // A tiny inline line chart. `null` values break the line (e.g. server downtime).
  let {
    values,
    width = 200,
    height = 40,
  }: { values: (number | null)[]; width?: number; height?: number } = $props();

  const max = $derived(Math.max(1, ...values.map((value) => value ?? 0)));

  const points = $derived(
    values.map((value, index) => ({
      x: values.length > 1 ? (index / (values.length - 1)) * width : width / 2,
      y: value == null ? null : height - 2 - (value / max) * (height - 4),
    }))
  );

  // Line path, lifting the pen across null gaps.
  const linePath = $derived.by(() => {
    let path = "";
    let drawing = false;
    for (const point of points) {
      if (point.y == null) {
        drawing = false;
        continue;
      }
      path += `${drawing ? "L" : "M"}${point.x.toFixed(1)} ${point.y.toFixed(1)} `;
      drawing = true;
    }
    return path.trim();
  });
</script>

<svg class="spark" viewBox="0 0 {width} {height}" preserveAspectRatio="none" role="img">
  <path d={linePath} fill="none" stroke="var(--accent)" stroke-width="1.5" />
</svg>

<style>
  .spark {
    width: 100%;
    height: auto;
    display: block;
    overflow: visible;
  }
</style>
