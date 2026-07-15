<script lang="ts">
  // Pixel icons from @hackernoon/pixel-icon-library (an iconfont). We keep our
  // semantic names and map them to the library's `hn-*` classes.
  interface Props {
    name: string;
    size?: number;
    stroke?: number; // unused (font icons); kept for call-site compatibility
  }
  let { name, size = 20 }: Props = $props();

  const MAP: Record<string, string> = {
    home: "home",
    compass: "globe", // Browse / discover
    library: "grid",
    settings: "cog",
    plus: "plus",
    play: "play",
    stop: "pause",
    trash: "trash",
    cube: "archive",
    folder: "folder",
    edit: "edit",
    user: "user",
    search: "search",
    clock: "clock",
    package: "archive",
  };

  const cls = $derived(`hn hn-${MAP[name] ?? "grid"}`);
</script>

<i
  class={cls}
  style="font-size:{size}px; line-height:1; display:inline-flex; align-items:center; justify-content:center;"
  aria-hidden="true"
></i>
