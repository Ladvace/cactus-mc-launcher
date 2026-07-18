<script lang="ts">
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { requiredJavaMajor } from "$lib/java";
  import type { Instance } from "$lib/types";

  let { instance, isServer = false }: { instance: Instance; isServer?: boolean } =
    $props();

  const globals = $derived(settingsStore.settings);

  // The Java this version actually uses, and where it comes from by default.
  const javaMajor = $derived(requiredJavaMajor(instance.mcVersion));
  const javaPlaceholder = $derived.by(() => {
    const perMajor = globals.javaPaths?.[String(javaMajor)]?.trim();
    if (perMajor) return `Java ${javaMajor} · ${perMajor}`;
    if (globals.javaPath?.trim()) return `Global · ${globals.javaPath}`;
    return `Managed Java ${javaMajor}`;
  });

  // Local string drafts (empty = "use global default").
  let maxMem = $state("");
  let minMem = $state("");
  let jvm = $state("");
  let javaPath = $state("");
  let width = $state("");
  let height = $state("");
  let saving = $state(false);

  // (Re)load the drafts when the instance changes.
  let lastId = "";
  $effect(() => {
    if (instance.id !== lastId) {
      lastId = instance.id;
      const legacy = isServer ? instance.serverMemoryMb : null;
      maxMem = str(instance.maxMemoryMb ?? legacy);
      minMem = str(instance.minMemoryMb);
      jvm = instance.jvmArgs ?? "";
      javaPath = instance.javaPath ?? "";
      width = str(instance.gameWidth);
      height = str(instance.gameHeight);
    }
  });

  function str(value: number | null | undefined): string {
    return value != null ? String(value) : "";
  }
  function num(text: string): number {
    const parsed = parseInt(text, 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 0; // 0 = clear the override
  }

  async function save() {
    saving = true;
    try {
      await instancesStore.update(instance.id, {
        maxMemoryMb: num(maxMem),
        minMemoryMb: num(minMem),
        jvmArgs: jvm.trim(),
        javaPath: javaPath.trim(),
        gameWidth: isServer ? undefined : num(width),
        gameHeight: isServer ? undefined : num(height),
        serverMemoryMb: 0, // migrate any legacy value into maxMemoryMb
      });
      toast.success("Instance settings saved.");
    } catch (error) {
      toast.error(String(error));
    } finally {
      saving = false;
    }
  }

  function reset() {
    maxMem = "";
    minMem = "";
    jvm = "";
    javaPath = "";
    width = "";
    height = "";
  }
</script>

<section class="card-block">
  <div class="head">
    <h3>Java &amp; memory</h3>
    <span class="muted">Overrides for this instance — leave blank to use the global setting.</span>
  </div>

  <div class="grid">
    <label class="field">
      <span>Max memory (MB)</span>
      <input class="input" type="number" min="512" step="512" placeholder={`Global · ${globals.maxMemoryMb}`} bind:value={maxMem} />
    </label>
    <label class="field">
      <span>Min memory (MB)</span>
      <input class="input" type="number" min="256" step="256" placeholder={`Global · ${globals.minMemoryMb}`} bind:value={minMem} />
    </label>

    <label class="field wide">
      <span>Extra JVM arguments</span>
      <input class="input" placeholder={globals.jvmArgs?.trim() ? `Global · ${globals.jvmArgs}` : "e.g. -XX:+UseG1GC"} bind:value={jvm} />
    </label>

    <label class="field wide">
      <span>Java path <small class="hint">· this version runs on Java {javaMajor}</small></span>
      <input class="input" placeholder={javaPlaceholder} bind:value={javaPath} spellcheck="false" />
    </label>

    {#if !isServer}
      <label class="field">
        <span>Window width</span>
        <input class="input" type="number" min="1" placeholder={`Global · ${globals.gameWidth}`} bind:value={width} />
      </label>
      <label class="field">
        <span>Window height</span>
        <input class="input" type="number" min="1" placeholder={`Global · ${globals.gameHeight}`} bind:value={height} />
      </label>
    {/if}
  </div>

  <div class="actions">
    <button class="btn ghost sm" onclick={reset} disabled={saving}>Clear all</button>
    <button class="btn primary sm" onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save"}
    </button>
  </div>
</section>

<style>
  .card-block {
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    padding: 18px 20px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 14px;
  }
  .head h3 {
    font-size: 14px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 16px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  .field.wide {
    grid-column: 1 / -1;
  }
  .hint {
    color: var(--text-muted);
    font-weight: 400;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
</style>
