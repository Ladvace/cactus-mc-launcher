<script lang="ts">
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import { t, type MessageKey } from "$lib/i18n";

  let { id, running = false }: { id: string; running?: boolean } = $props();

  type FieldType = "text" | "number" | "bool" | "select";
  interface Field {
    key: string;
    label: MessageKey;
    type: FieldType;
    def: string;
    options?: string[];
    hint?: MessageKey;
  }

  const FIELDS: Field[] = [
    { key: "motd", label: "server.fieldMotd", type: "text", def: "A Minecraft Server" },
    { key: "max-players", label: "server.fieldMaxPlayers", type: "number", def: "20" },
    { key: "difficulty", label: "server.fieldDifficulty", type: "select", def: "easy", options: ["peaceful", "easy", "normal", "hard"] },
    { key: "gamemode", label: "server.fieldGamemode", type: "select", def: "survival", options: ["survival", "creative", "adventure", "spectator"] },
    { key: "hardcore", label: "server.fieldHardcore", type: "bool", def: "false" },
    { key: "pvp", label: "server.fieldPvp", type: "bool", def: "true" },
    { key: "online-mode", label: "server.fieldOnlineMode", type: "bool", def: "true", hint: "server.fieldOnlineModeHint" },
    { key: "white-list", label: "server.fieldWhitelist", type: "bool", def: "false" },
    { key: "level-name", label: "server.fieldWorldName", type: "text", def: "world" },
    { key: "level-seed", label: "server.fieldWorldSeed", type: "text", def: "" },
    { key: "server-port", label: "server.fieldServerPort", type: "number", def: "25565" },
    { key: "view-distance", label: "server.fieldViewDistance", type: "number", def: "10" },
    { key: "simulation-distance", label: "server.fieldSimulationDistance", type: "number", def: "10" },
    { key: "spawn-protection", label: "server.fieldSpawnProtection", type: "number", def: "16" },
    { key: "allow-nether", label: "server.fieldAllowNether", type: "bool", def: "true" },
    { key: "allow-flight", label: "server.fieldAllowFlight", type: "bool", def: "false" },
    { key: "enable-command-block", label: "server.fieldCommandBlocks", type: "bool", def: "false" },
  ];
  const MANAGED = new Set(FIELDS.map((field) => field.key));

  type Line =
    | { kind: "kv"; key: string; value: string; raw: string }
    | { kind: "raw"; raw: string };

  let rawLines = $state<Line[]>([]);
  let values = $state<Record<string, string>>({});
  let loading = $state(false);
  let saving = $state(false);
  let loaded = $state(false);

  function parse(text: string): Line[] {
    return text.split(/\r?\n/).map((raw): Line => {
      const trimmed = raw.trimStart();
      if (!trimmed || trimmed.startsWith("#")) return { kind: "raw", raw };
      const equalsIndex = raw.indexOf("=");
      if (equalsIndex < 0) return { kind: "raw", raw };
      return { kind: "kv", key: raw.slice(0, equalsIndex).trim(), value: raw.slice(equalsIndex + 1), raw };
    });
  }

  async function load() {
    loading = true;
    try {
      const text = await api.readServerProperties(id);
      const lines = parse(text);
      rawLines = lines;
      const found: Record<string, string> = {};
      for (const line of lines) if (line.kind === "kv") found[line.key] = line.value;
      const next: Record<string, string> = {};
      for (const field of FIELDS) next[field.key] = found[field.key] ?? field.def;
      values = next;
      loaded = true;
    } catch (error) {
      toast.error(String(error));
    } finally {
      loading = false;
    }
  }

  let lastId = "";
  $effect(() => {
    if (id && id !== lastId) {
      lastId = id;
      load();
    }
  });

  async function save() {
    saving = true;
    try {
      const seen = new Set<string>();
      const out = rawLines.map((line) => {
        if (line.kind === "kv" && MANAGED.has(line.key)) {
          seen.add(line.key);
          return `${line.key}=${values[line.key] ?? ""}`;
        }
        return line.raw;
      });
      for (const field of FIELDS) {
        if (!seen.has(field.key) && (values[field.key] ?? "") !== "") {
          out.push(`${field.key}=${values[field.key]}`);
        }
      }
      let text = out.join("\n");
      if (!text.endsWith("\n")) text += "\n";
      await api.writeServerProperties(id, text);
      rawLines = parse(text);
      toast.success(t("server.propsSaved"));
    } catch (error) {
      toast.error(String(error));
    } finally {
      saving = false;
    }
  }
</script>

<div class="props">
  <div class="props-head">
    <p class="muted">
      {t("server.propsLeadPrefix")}<code>server.properties</code>{t("server.propsLeadSuffix")}
      {#if running}
        <strong>{t("server.propsRestartHint")}</strong>
      {/if}
    </p>
    <div class="head-actions">
      <button class="btn ghost sm" onclick={load} disabled={loading || saving}>{t("server.reload")}</button>
      <button class="btn primary sm" onclick={save} disabled={loading || saving || !loaded}>
        {saving ? t("server.saving") : t("common.save")}
      </button>
    </div>
  </div>

  {#if loading && !loaded}
    <p class="muted">{t("common.loading")}</p>
  {:else}
    <div class="grid">
      {#each FIELDS as field (field.key)}
        <div class="field" class:wide={field.type === "text"}>
          {#if field.type === "bool"}
            <label class="toggle">
              <input
                type="checkbox"
                checked={values[field.key] === "true"}
                onchange={(event) =>
                  (values[field.key] = (event.currentTarget as HTMLInputElement).checked ? "true" : "false")}
              />
              <span>{t(field.label)}</span>
            </label>
          {:else}
            <label class="field-label" for={`p-${field.key}`}>{t(field.label)}</label>
            {#if field.type === "select"}
              <select id={`p-${field.key}`} class="select" bind:value={values[field.key]}>
                {#each field.options ?? [] as option (option)}
                  <option value={option}>{option}</option>
                {/each}
              </select>
            {:else}
              <input
                id={`p-${field.key}`}
                class="input"
                type={field.type === "number" ? "number" : "text"}
                bind:value={values[field.key]}
              />
            {/if}
          {/if}
          {#if field.hint}<span class="hint">{t(field.hint)}</span>{/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .props-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
  }
  .props-head .muted {
    font-size: 12.5px;
    max-width: 60ch;
  }
  .head-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  code {
    font-family: var(--font-pixel);
    color: var(--accent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px 16px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field.wide {
    grid-column: 1 / -1;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
    padding-top: 4px;
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-muted);
  }
</style>
