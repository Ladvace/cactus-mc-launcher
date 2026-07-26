import type { Action } from "svelte/action";

/**
 * `use:tooltip={"text"}` — a hover/focus tooltip that works in the app's webview
 * (native `title` tooltips don't render there). Renders a fixed-position element
 * on `document.body`, so no ancestor `overflow` can clip it.
 */
export const tooltip: Action<HTMLElement, string | undefined> = (node, text) => {
  let current = text;
  let tip: HTMLDivElement | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function place() {
    if (!tip) return;
    const anchor = node.getBoundingClientRect();
    const rect = tip.getBoundingClientRect();
    const margin = 6;
    let left = anchor.left + anchor.width / 2 - rect.width / 2;
    left = Math.max(margin, Math.min(left, window.innerWidth - rect.width - margin));
    let top = anchor.top - rect.height - 8;
    if (top < margin) top = anchor.bottom + 8; // flip below when no room above
    tip.style.left = `${Math.round(left)}px`;
    tip.style.top = `${Math.round(top)}px`;
  }

  function show() {
    if (tip || !current) return;
    tip = document.createElement("div");
    tip.className = "app-tooltip";
    tip.textContent = current;
    document.body.appendChild(tip);
    place();
    requestAnimationFrame(() => tip?.classList.add("show"));
  }

  function scheduleShow() {
    timer = setTimeout(show, 350);
  }

  function hide() {
    clearTimeout(timer);
    tip?.remove();
    tip = null;
  }

  node.addEventListener("mouseenter", scheduleShow);
  node.addEventListener("mouseleave", hide);
  node.addEventListener("focus", show);
  node.addEventListener("blur", hide);
  node.addEventListener("click", hide);
  window.addEventListener("scroll", hide, true);

  return {
    update(next) {
      current = next;
      if (tip) {
        if (next) {
          tip.textContent = next;
          place();
        } else {
          hide();
        }
      }
    },
    destroy() {
      hide();
      node.removeEventListener("mouseenter", scheduleShow);
      node.removeEventListener("mouseleave", hide);
      node.removeEventListener("focus", show);
      node.removeEventListener("blur", hide);
      node.removeEventListener("click", hide);
      window.removeEventListener("scroll", hide, true);
    },
  };
};
