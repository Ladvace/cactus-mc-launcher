<script lang="ts">
  // Lightweight inline icon set (Lucide-style paths). No external dependency.
  interface Props {
    name: string;
    size?: number;
    stroke?: number;
  }
  let { name, size = 20, stroke = 2 }: Props = $props();

  const paths: Record<string, string> = {
    home: "M3 9.5 12 3l9 6.5V20a1 1 0 0 1-1 1h-5v-6H10v6H4a1 1 0 0 1-1-1V9.5Z",
    compass: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20ZM16.2 7.8l-2 5.2-5.2 2 2-5.2 5.2-2Z",
    library: "M4 4h7v7H4V4Zm9 0h7v7h-7V4ZM4 13h7v7H4v-7Zm9 0h7v7h-7v-7Z",
    settings:
      "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm7.4-3a7.6 7.6 0 0 0-.1-1.3l2-1.6-2-3.4-2.4 1a7.3 7.3 0 0 0-2.2-1.3L14.2 2H9.8l-.5 2.5a7.3 7.3 0 0 0-2.2 1.3l-2.4-1-2 3.4 2 1.6a7.6 7.6 0 0 0 0 2.6l-2 1.6 2 3.4 2.4-1c.66.53 1.4.97 2.2 1.3l.5 2.5h4.4l.5-2.5c.8-.33 1.54-.77 2.2-1.3l2.4 1 2-3.4-2-1.6c.07-.43.1-.86.1-1.3Z",
    plus: "M12 5v14M5 12h14",
    play: "M6 4l14 8-14 8V4Z",
    trash: "M4 7h16M9 7V4h6v3m-8 0 1 13h8l1-13",
    cube: "M12 2 3 7v10l9 5 9-5V7l-9-5Zm0 0v20M3 7l9 5 9-5",
    folder: "M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6Z",
    edit: "M4 20h4L18.5 9.5a2.1 2.1 0 0 0-3-3L5 17v3ZM13.5 6.5l3 3",
    user: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8ZM4 21a8 8 0 0 1 16 0",
    search: "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16Zm10 2-4.35-4.35",
    clock: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20ZM12 6v6l4 2",
    package:
      "M21 8v8a2 2 0 0 1-1 1.73l-7 4a2 2 0 0 1-2 0l-7-4A2 2 0 0 1 3 16V8a2 2 0 0 1 1-1.73l7-4a2 2 0 0 1 2 0l7 4A2 2 0 0 1 21 8ZM3.3 7 12 12l8.7-5M12 22V12",
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name] ?? ""} />
</svg>
