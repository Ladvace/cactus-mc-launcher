// Sets `--pct` (0–100%) on a range input from its value, so the CSS can paint
// the filled portion of the track. Pass the bound value as the action argument
// so it also updates on programmatic changes, not just drags.
export function sliderFill(node: HTMLInputElement, _value?: unknown) {
  const update = () => {
    const min = Number(node.min || 0);
    const max = Number(node.max || 100);
    const val = Number(node.value);
    const pct = max > min ? ((val - min) / (max - min)) * 100 : 0;
    node.style.setProperty("--pct", `${pct}%`);
  };
  update();
  node.addEventListener("input", update);
  return {
    update,
    destroy: () => node.removeEventListener("input", update),
  };
}
