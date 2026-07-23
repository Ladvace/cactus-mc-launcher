<script lang="ts">
  interface Props {
    name: string;
    size?: number;
    stroke?: number; // unused (font icons); kept for call-site compatibility
  }
  let { name, size = 20 }: Props = $props();

  const MAP: Record<string, string> = {
    home: "home",
    compass: "search",
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
    shuffle: "shuffle",
    sparkles: "sparkles",
    expand: "expand",
    refresh: "refresh",
    check: "check",
    copy: "copy",
    share: "share",
    upload: "upload",
    download: "download",
    users: "users",
    video: "video-camera",
    globe: "globe",
    trophy: "trophy",
    star: "star",
    heart: "heart",
    moon: "moon",
    fire: "fire",
    flag: "flag-checkered",
    crown: "crown",
    bolt: "bolt",
    trending: "trending",
    book: "book",
    "chevron-left": "angle-left",
    "chevron-right": "angle-right",
    "chevron-down": "angle-down",
    "chevron-up": "angle-up",
    filter: "filter",
    close: "times",
    grid: "grid",
    list: "bullet-list",
  };

  const iconClass = $derived(`hn hn-${MAP[name] ?? "grid"}`);
</script>

<i
  class={iconClass}
  style="font-size:{size}px; line-height:1; display:inline-flex; align-items:center; justify-content:center;"
  aria-hidden="true"
></i>
