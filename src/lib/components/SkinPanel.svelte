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
  let capesLoading = $state(true);
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
    api
      .getCapes()
      .then((owned) => {
        if (cancelled) return;
        capes = owned;
        loadActiveCape(owned);
      })
      .catch(() => !cancelled && (capes = []))
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

    {#if capesLoading}
      <span class="skin-label">{t("account.cape")}</span>
      <div class="cape-row">
        {#each Array(3) as _, index (index)}
          <span class="skeleton cape-skel"></span>
        {/each}
      </div>
    {:else if capes.length}
      <span class="skin-label">{t("account.cape")}</span>
      <div class="cape-row">
        <button class="cape-btn" class:on={!capes.some((cape) => cape.active)} onclick={() => chooseCape(null)}>
          {t("account.capeNone")}
        </button>
        {#each capes as cape (cape.id)}
          <button class="cape-btn" class:on={cape.active} onclick={() => chooseCape(cape.id)}>
            {cape.alias}
          </button>
        {/each}
      </div>
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
    gap: 5px;
  }
  .cape-skel {
    width: 54px;
    height: 25px;
  }
  .cape-btn {
    padding: 4px 8px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 11.5px;
  }
  .cape-btn.on {
    border-color: var(--accent);
    color: var(--accent);
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
