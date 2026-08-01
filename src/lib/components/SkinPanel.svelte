<script lang="ts">
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { skinCache } from "$lib/stores/skins.svelte";
  import { t } from "$lib/i18n";
  import type { AccountInfo } from "$lib/types";
  import Icon from "./Icon.svelte";
  import Select from "./Select.svelte";
  import SkinViewer from "./SkinViewer.svelte";
  import SkinEditor from "./SkinEditor.svelte";

  let { account }: { account: AccountInfo } = $props();

  type Cape = { id: string; alias: string; url: string; active: boolean };

  let mode = $state<"3d" | "2d">("3d");
  let variant = $state<"classic" | "slim">("classic");
  let skinData = $state("");
  let capeData = $state("");
  let capes = $state<Cape[]>([]);
  let capeThumbs = $state<Record<string, string>>({});
  let capesLoading = $state(true);
  let capesError = $state<string | null>(null);
  let skinInput = $state<HTMLInputElement>();
  let changing = $state(false);
  let dragOver = $state(false);
  let skinMsg = $state<string | null>(null);
  let editorOpen = $state(false);

  const VARIANTS = $derived<{ value: "classic" | "slim"; label: string }[]>([
    { value: "classic", label: t("account.modelClassic") },
    { value: "slim", label: t("account.modelSlim") },
  ]);

  $effect(() => {
    const cached = skinCache.getFull(account.uuid);
    if (cached) {
      skinData = cached;
      return;
    }
    skinData = "";
    let cancelled = false;
    api
      .downloadImage(`https://minotar.net/skin/${account.uuid}`)
      .then((dataUri) => !cancelled && (skinData = dataUri))
      .catch(() => !cancelled && (skinData = ""));
    return () => (cancelled = true);
  });

  $effect(() => {
    void account.uuid;
    let cancelled = false;
    capesLoading = true;
    capesError = null;
    capeThumbs = {};
    api
      .getCapes()
      .then(async (owned) => {
        if (cancelled) return;
        capes = owned;
        loadActiveCape(owned);
        const thumbs = await buildCapeThumbs(owned);
        if (!cancelled) capeThumbs = thumbs;
      })
      .catch((err) => {
        if (cancelled) return;
        capes = [];
        capesError = String(err);
      })
      .finally(() => !cancelled && (capesLoading = false));
    return () => (cancelled = true);
  });

  function loadActiveCape(list: Cape[]) {
    const active = list.find((cape) => cape.active);
    if (active) {
      api.downloadImage(active.url).then((uri) => (capeData = uri)).catch(() => (capeData = ""));
    } else {
      capeData = "";
    }
  }

  // Cape textures are a 64×32 sheet; the front face sits at (1,1) sized 10×16.
  // Crop just that region (scaled to the actual sheet width so legacy sizes
  // still map correctly) into a small pixelated thumbnail for each cape button.
  function cropCapeFront(src: string): Promise<string> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        const unit = img.width / 64;
        const canvas = document.createElement("canvas");
        canvas.width = 10;
        canvas.height = 16;
        const ctx = canvas.getContext("2d");
        if (!ctx) return resolve("");
        ctx.imageSmoothingEnabled = false;
        ctx.drawImage(img, unit, unit, 10 * unit, 16 * unit, 0, 0, 10, 16);
        resolve(canvas.toDataURL("image/png"));
      };
      img.onerror = () => resolve("");
      img.src = src;
    });
  }

  async function buildCapeThumbs(list: Cape[]): Promise<Record<string, string>> {
    const entries = await Promise.all(
      list.map(async (cape) => {
        try {
          const thumb = await cropCapeFront(await api.downloadImage(cape.url));
          return [cape.id, thumb] as const;
        } catch {
          return [cape.id, ""] as const;
        }
      })
    );
    return Object.fromEntries(entries.filter(([, thumb]) => thumb));
  }

  async function chooseCape(id: string | null) {
    try {
      await api.setCape(id);
      capes = capes.map((cape) => ({ ...cape, active: cape.id === id }));
      loadActiveCape(capes);
    } catch (error) {
      toast.error(String(error));
    }
  }

  function fileToDataUrl(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  async function applySkinFile(file: File) {
    changing = true;
    skinMsg = null;
    try {
      const buffer = await file.arrayBuffer();
      await api.setSkin(Array.from(new Uint8Array(buffer)), variant);
      skinData = await fileToDataUrl(file);
      skinCache.setSkin(account.uuid, skinData);
      skinMsg = t("account.skinUpdated");
      setTimeout(() => (skinMsg = null), 4000);
    } catch (err) {
      toast.error(String(err));
    } finally {
      changing = false;
    }
  }

  async function resetSkin() {
    changing = true;
    skinMsg = null;
    try {
      await api.resetSkin();
      skinCache.clear(account.uuid);
      skinData = await api
        .downloadImage(`https://minotar.net/skin/${account.uuid}?ts=${Date.now()}`)
        .catch(() => "");
      skinMsg = t("account.skinReset");
      setTimeout(() => (skinMsg = null), 4000);
    } catch (err) {
      toast.error(String(err));
    } finally {
      changing = false;
    }
  }

  function onSkinFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) applySkinFile(file);
  }

  function onSkinDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;
    const file = event.dataTransfer?.files?.[0];
    if (file && file.type === "image/png") applySkinFile(file);
  }
</script>

<div class="skin-panel">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="skin-stage"
    class:drag={dragOver}
    ondragover={(event) => {
      event.preventDefault();
      dragOver = true;
    }}
    ondragleave={() => (dragOver = false)}
    ondrop={onSkinDrop}
  >
    {#if mode === "3d"}
      {#if skinData}
        <SkinViewer skin={skinData} model={variant} cape={capeData} width={170} height={250} />
      {:else}
        <span class="skeleton stage-skel"></span>
      {/if}
    {:else}
      <img
        class="skin-2d"
        src={skinCache.getBody(account.uuid) ?? `https://minotar.net/armor/body/${account.uuid}/210.png`}
        alt={t("account.skinAlt", { username: account.username })}
      />
    {/if}
    {#if dragOver}<div class="drop-hint">{t("account.dropPngSkin")}</div>{/if}
  </div>
  <div class="skin-controls">
    <div class="mode-toggle">
      <button class:on={mode === "3d"} onclick={() => (mode = "3d")}>3D</button>
      <button class:on={mode === "2d"} onclick={() => (mode = "2d")}>2D</button>
    </div>
    <span class="skin-label">{t("account.model")}</span>
    <Select bind:value={variant} options={VARIANTS} ariaLabel={t("account.model")} />

    <span class="skin-label">{t("account.cape")}</span>
    {#if capesLoading}
      <div class="cape-row">
        {#each Array(3) as _, index (index)}
          <span class="skeleton cape-skel"></span>
        {/each}
      </div>
    {:else if capesError}
      <p class="cape-note error">{capesError}</p>
    {:else if capes.length}
      <div class="cape-row">
        <button
          class="cape-btn"
          class:on={!capes.some((cape) => cape.active)}
          title={t("account.capeNone")}
          onclick={() => chooseCape(null)}
        >
          <span class="cape-thumb none"><Icon name="close" size={16} /></span>
          <span class="cape-name">{t("account.capeNone")}</span>
        </button>
        {#each capes as cape (cape.id)}
          <button class="cape-btn" class:on={cape.active} title={cape.alias} onclick={() => chooseCape(cape.id)}>
            {#if capeThumbs[cape.id]}
              <img class="cape-thumb" src={capeThumbs[cape.id]} alt={cape.alias} />
            {:else}
              <span class="cape-thumb"></span>
            {/if}
            <span class="cape-name">{cape.alias}</span>
          </button>
        {/each}
      </div>
    {:else}
      <p class="cape-note">{t("account.capeEmpty")}</p>
    {/if}

    <div class="skin-actions">
      <button class="btn primary" disabled={changing} onclick={() => skinInput?.click()}>
        <Icon name="upload" size={14} />
        {changing ? t("account.applying") : t("account.changeSkin")}
      </button>
      <button class="btn" onclick={() => (editorOpen = true)}>
        <Icon name="edit" size={14} /> {t("skinEditor.drawSkin")}
      </button>
      <button class="btn ghost sm" disabled={changing} onclick={resetSkin}>
        <Icon name="refresh" size={13} /> {t("account.resetSkin")}
      </button>
    </div>
  </div>
  <input bind:this={skinInput} type="file" accept="image/png" style="display:none" onchange={onSkinFile} />
</div>
{#if skinMsg}<p class="skin-msg">{skinMsg}</p>{/if}
<p class="skin-hint">{t("account.skinUploadHint")}</p>

<SkinEditor
  open={editorOpen}
  onClose={() => (editorOpen = false)}
  uuid={account.uuid}
  {variant}
  onApplied={(uri) => {
    skinData = uri;
    skinCache.setSkin(account.uuid, uri);
  }}
/>

<style>
  .skin-panel {
    display: flex;
    align-items: stretch;
    gap: 16px;
    padding: 14px;
    margin-bottom: 8px;
    background: var(--bg-input);
    border: 2px solid var(--border);
  }
  .skin-stage {
    position: relative;
    width: 170px;
    min-height: 250px;
    align-self: stretch;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: radial-gradient(circle at 50% 35%, var(--bg-card), var(--bg-app));
    border: 2px solid var(--border-subtle);
  }
  .skin-stage.drag {
    border-color: var(--accent);
    border-style: dashed;
  }
  .stage-skel {
    width: 110px;
    height: 200px;
    border-radius: 4px;
  }
  .drop-hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.55);
    color: var(--accent);
    font-size: 13px;
    pointer-events: none;
  }
  .skin-2d {
    max-height: 230px;
    image-rendering: pixelated;
  }
  .skin-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-width: 0;
    justify-content: center;
  }
  .skin-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mode-toggle {
    display: flex;
    border: 2px solid var(--border);
    align-self: flex-start;
  }
  .mode-toggle button {
    padding: 5px 14px;
    background: var(--bg-card);
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-pixel);
    font-size: 12px;
  }
  .mode-toggle button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .skin-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-top: 4px;
  }
  .cape-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .cape-skel {
    width: 44px;
    height: 62px;
  }
  .cape-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 44px;
    padding: 4px 4px 5px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 10px;
    cursor: pointer;
  }
  .cape-btn:hover {
    border-color: var(--accent);
  }
  .cape-btn.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .cape-thumb {
    width: 30px;
    height: 48px;
    image-rendering: pixelated;
    background: var(--bg-app);
    border: 1px solid var(--border-subtle);
  }
  .cape-thumb.none {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .cape-name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.2;
  }
  .cape-note {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
  }
  .cape-note.error {
    color: var(--danger, #e5484d);
  }
  .skin-msg {
    margin: 0 0 2px;
    font-size: 12px;
    color: var(--accent);
  }
  .skin-hint {
    margin: 0 0 16px;
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
