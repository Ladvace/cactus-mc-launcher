<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { api } from "$lib/api";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { emojiToDataUri } from "$lib/image";
  import { DECOR_SPRITES, spriteToSquareIcon } from "$lib/decor";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Sticker } from "$lib/types";

  const picker = $derived(ui.stickerPicker);
  const open = $derived(!!picker);

  // Stickers are enabled only once the user provides a Giphy API key.
  const enabled = $derived(settingsStore.settings.giphyApiKey.trim().length > 0);

  let tab = $state<"stickers" | "emoji" | "decor">("decor");
  let keyDraft = $state("");
  let savingKey = $state(false);
  let editingKey = $state(false); // re-entering a key even though one is set
  let query = $state("");
  let debounced = $state("");
  let stickers = $state<Sticker[]>([]);
  let loading = $state(false);
  let loadingMore = $state(false);
  let hasMore = $state(true);
  let offset = $state(0);
  let error = $state<string | null>(null);
  let applying = $state(false);

  const LIMIT = 30; // must match the backend page size
  const COLS = 3;

  // Distribute stickers across fixed columns (round-robin) so the masonry
  // scrolls vertically instead of overflowing into more columns sideways.
  const columns = $derived.by(() => {
    const cols: Sticker[][] = Array.from({ length: COLS }, () => []);
    stickers.forEach((sticker, index) => cols[index % COLS].push(sticker));
    return cols;
  });

  // Pick the default tab each time the picker opens (without fighting the user
  // switching tabs while it's open).
  let wasOpen = false;
  $effect(() => {
    if (open && !wasOpen) tab = "decor";
    wasOpen = open;
  });

  async function chooseDecor(url: string) {
    const onPick = picker?.onPick;
    if (!onPick || applying) return;
    applying = true;
    try {
      onPick(await spriteToSquareIcon(url));
      ui.closeStickerPicker();
    } catch (err) {
      error = String(err);
    } finally {
      applying = false;
    }
  }

  async function saveKey() {
    const key = keyDraft.trim();
    if (!key) return;
    savingKey = true;
    try {
      await settingsStore.save({ ...settingsStore.settings, giphyApiKey: key });
      keyDraft = "";
      editingKey = false;
      error = null;
      tab = "stickers";
    } catch (err) {
      error = String(err);
    } finally {
      savingKey = false;
    }
  }

  $effect(() => {
    if (!open) {
      query = "";
      debounced = "";
      stickers = [];
      offset = 0;
      hasMore = true;
      error = null;
      applying = false;
    }
  });

  $effect(() => {
    const queryText = query;
    const timer = setTimeout(() => (debounced = queryText), 350);
    return () => clearTimeout(timer);
  });

  // (Re)load from the top when open on the stickers tab and the query changes.
  $effect(() => {
    if (open && enabled && tab === "stickers") {
      void debounced;
      resetAndLoad();
    }
  });

  async function resetAndLoad() {
    loading = true;
    error = null;
    offset = 0;
    hasMore = true;
    try {
      const results = await api.searchStickers(debounced, 0);
      stickers = results;
      offset = results.length;
      hasMore = results.length >= LIMIT;
    } catch (err) {
      error = String(err);
      stickers = [];
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (loading || loadingMore || !hasMore) return;
    loadingMore = true;
    try {
      const results = await api.searchStickers(debounced, offset);
      const seen = new Set(stickers.map((sticker) => sticker.id));
      stickers = [...stickers, ...results.filter((sticker) => !seen.has(sticker.id))];
      offset += results.length;
      hasMore = results.length >= LIMIT;
    } catch {
      hasMore = false; // stop trying on error, keep what we have
    } finally {
      loadingMore = false;
    }
  }

  function onScroll(event: Event) {
    const el = event.currentTarget as HTMLElement;
    if (el.scrollHeight - el.scrollTop - el.clientHeight < 140) loadMore();
  }

  async function chooseSticker(sticker: Sticker) {
    const onPick = picker?.onPick;
    if (!onPick || applying) return;
    applying = true;
    try {
      const uri = await api.downloadImage(sticker.full);
      onPick(uri);
      ui.closeStickerPicker();
    } catch (err) {
      error = String(err);
    } finally {
      applying = false;
    }
  }

  function chooseEmoji(emoji: string) {
    const onPick = picker?.onPick;
    if (!onPick) return;
    onPick(emojiToDataUri(emoji));
    ui.closeStickerPicker();
  }

  const GROUPS = [
    {
      name: "Blocks & tools",
      emoji: ["⛏️", "🗡️", "🪓", "⚒️", "🛡️", "🏹", "🧨", "💎", "💠", "🧱", "🗺️", "🧭", "🔥", "💧", "🌳", "🌸"],
    },
    {
      name: "Mobs",
      emoji: ["🐷", "🐑", "🐮", "🐔", "🐺", "🐝", "🦊", "🐢", "🐼", "🐉", "👾", "🧟", "💀", "👻", "🕷️", "🦇"],
    },
    {
      name: "Faces",
      emoji: ["😀", "😎", "🤠", "🤖", "😈", "🥳", "🤩", "😴", "🤯", "🫠", "👽", "🎃", "🥸", "😤", "🤓", "🫡"],
    },
    {
      name: "Symbols",
      emoji: ["⭐", "🌈", "⚡", "❤️", "🏆", "🎮", "✨", "🔷", "🟢", "🟣", "🟡", "🔶", "☠️", "🌙", "☀️", "🎯"],
    },
  ];
</script>

<Modal
  title={picker?.title ?? "Choose an image"}
  {open}
  onClose={() => ui.closeStickerPicker()}
  width={480}
>
  {#if picker}
    <div class="tabs">
      <button class="tab" class:active={tab === "decor"} onclick={() => (tab = "decor")}>
        Cactus
      </button>
      <button
        class="tab"
        class:active={tab === "stickers"}
        onclick={() => (tab = "stickers")}
      >
        <Icon name="sparkles" size={14} /> Stickers
      </button>
      <button
        class="tab"
        class:active={tab === "emoji"}
        onclick={() => (tab = "emoji")}
      >
        Emoji
      </button>
    </div>

    {#if tab === "decor"}
      <div class="decor-grid" class:busy={applying}>
        {#each DECOR_SPRITES as url (url)}
          <button class="decor-cell" disabled={applying} onclick={() => chooseDecor(url)}>
            <img src={url} alt="decoration" loading="lazy" />
          </button>
        {/each}
      </div>
    {:else if tab === "stickers"}
      {#if !enabled || editingKey}
        <div class="notice">
          <p>
            <strong>{editingKey ? "Update your Giphy API key" : "Animated stickers are off."}</strong>
          </p>
          <p>
            Paste a free Giphy API key to turn them on (grab one at
            <button class="linkish" onclick={() => openUrl("https://developers.giphy.com")}>
              developers.giphy.com</button
            >). The Emoji tab always works.
          </p>
          <div class="key-row">
            <input
              class="key-input"
              type="password"
              placeholder="Giphy API key"
              autocomplete="off"
              spellcheck="false"
              bind:value={keyDraft}
              onkeydown={(event) => event.key === "Enter" && saveKey()}
            />
            <button class="btn primary sm" disabled={savingKey || !keyDraft.trim()} onclick={saveKey}>
              {savingKey ? "Saving…" : "Save"}
            </button>
            {#if editingKey}
              <button class="btn ghost sm" onclick={() => (editingKey = false)}>Cancel</button>
            {/if}
          </div>
          <p class="tiny">You can also change this later in Settings → Interface.</p>
        </div>
      {:else}
        <div class="search">
          <Icon name="search" size={16} />
          <input
            class="search-input"
            placeholder="Search stickers…"
            bind:value={query}
          />
        </div>

        {#if loading}
          <div class="sticker-grid">
            {#each Array(COLS) as _, colIndex (colIndex)}
              <div class="col">
                {#each Array(4) as _, rowIndex (rowIndex)}
                  <span class="skeleton cell"></span>
                {/each}
              </div>
            {/each}
          </div>
        {:else if error}
          <div class="notice error">
            <p>{error}</p>
            <button
              class="btn ghost sm"
              onclick={() => {
                keyDraft = "";
                editingKey = true;
              }}
            >
              Change API key
            </button>
          </div>
        {:else if stickers.length === 0}
          <p class="muted">No stickers found.</p>
        {:else}
          <div class="sticker-grid" class:busy={applying} onscroll={onScroll}>
            {#each columns as col, colIndex (colIndex)}
              <div class="col">
                {#each col as sticker (sticker.id)}
                  <button
                    class="cell"
                    title="Use this sticker"
                    disabled={applying}
                    onclick={() => chooseSticker(sticker)}
                  >
                    <img src={sticker.preview} alt="sticker" loading="lazy" />
                  </button>
                {/each}
                {#if loadingMore}
                  <span class="skeleton cell"></span>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
        <div class="giphy-mark" title="Powered By GIPHY">
          <span class="pb">Powered By</span>
          <span class="wordmark">GIPHY</span>
        </div>
      {/if}
    {:else}
      {#each GROUPS as group}
        <section class="group">
          <h4>{group.name}</h4>
          <div class="emoji-grid">
            {#each group.emoji as emoji}
              <button
                class="emoji"
                title={`Use ${emoji}`}
                onclick={() => chooseEmoji(emoji)}
              >
                {emoji}
              </button>
            {/each}
          </div>
        </section>
      {/each}
    {/if}
  {/if}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 16px;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-bottom: 3px solid transparent;
    color: var(--text-secondary);
    font-family: var(--font-pixel);
    font-size: 13px;
    font-weight: 500;
    margin-bottom: -1px;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .search {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: 12px;
  }
  .search :global(.hn) {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 9px 12px 9px 36px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.28);
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  /* Masonry via real columns: a flex row of vertical stacks. Each sticker keeps
     its natural aspect ratio (stickers are often wide banners), and the whole
     grid scrolls vertically. */
  .sticker-grid {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    max-height: 340px;
    overflow-y: auto;
    overflow-x: hidden;
    padding-right: 2px;
  }
  .col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sticker-grid.busy {
    opacity: 0.5;
    pointer-events: none;
  }
  .cell {
    display: block;
    width: 100%;
    margin: 0;
    padding: 0;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    overflow: hidden;
    transition: border-color 0.1s, transform 0.08s;
  }
  button.cell:hover {
    border-color: var(--accent);
    transform: scale(1.03);
  }
  button.cell:active {
    transform: scale(0.97);
  }
  .cell img {
    display: block;
    width: 100%;
    height: auto;
  }
  .skeleton.cell {
    height: 96px;
  }
  .skeleton.cell:nth-child(3n) {
    height: 128px;
  }
  .skeleton.cell:nth-child(3n + 2) {
    height: 72px;
  }
  /* Required "Powered By GIPHY" attribution mark. */
  .giphy-mark {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 12px 0 0;
    padding: 5px 9px;
    background: #000;
    border-radius: 3px;
  }
  .giphy-mark .pb {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #b7b7b7;
  }
  .giphy-mark .wordmark {
    font-family: "Arial Black", var(--font-pixel), sans-serif;
    font-size: 13px;
    font-weight: 900;
    letter-spacing: 0.02em;
    line-height: 1;
    /* GIPHY's signature pink→green wordmark. */
    background: linear-gradient(90deg, #ff6666 0%, #9933ff 50%, #00ff99 100%);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    color: transparent;
  }

  .notice {
    padding: 14px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.5;
  }
  .notice p {
    margin: 0 0 6px;
  }
  .notice p:last-child {
    margin: 0;
  }
  .notice.error {
    border-color: var(--danger);
    color: var(--danger);
  }
  .notice.error .btn {
    margin-top: 8px;
  }
  .linkish {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
  .key-row {
    display: flex;
    gap: 8px;
    margin: 10px 0 4px;
  }
  .key-input {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    background: var(--bg-app);
    border: 2px solid var(--border);
    color: var(--text);
    font-size: 13px;
  }
  .key-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .tiny {
    font-size: 11.5px;
    color: var(--text-muted);
    margin: 0;
  }
  .muted {
    color: var(--text-muted);
    text-align: center;
    padding: 24px;
  }

  .group + .group {
    margin-top: 18px;
  }
  .group h4 {
    margin: 0 0 8px;
    font-family: var(--font-pixel);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .emoji-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 6px;
  }
  .emoji {
    aspect-ratio: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    line-height: 1;
    background: var(--bg-input);
    border: 2px solid var(--border);
    border-radius: 0;
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.22);
    transition: transform 0.08s, border-color 0.1s, background 0.1s;
  }
  .emoji:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    transform: scale(1.12);
  }
  .emoji:active {
    transform: scale(0.94);
  }

  .decor-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
    max-height: 360px;
    overflow-y: auto;
    padding-right: 2px;
  }
  .decor-grid.busy {
    opacity: 0.5;
    pointer-events: none;
  }
  .decor-cell {
    aspect-ratio: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    transition: border-color 0.1s, transform 0.08s;
  }
  .decor-cell:hover {
    border-color: var(--accent);
    transform: scale(1.05);
  }
  .decor-cell:active {
    transform: scale(0.96);
  }
  .decor-cell img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
</style>
