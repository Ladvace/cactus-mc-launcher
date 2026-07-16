<script lang="ts">
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";

  let { id, running = false }: { id: string; running?: boolean } = $props();

  type FieldType = "text" | "number" | "bool" | "select";
  interface Field {
    key: string;
    label: string;
    type: FieldType;
    def: string;
    options?: string[];
    hint?: string;
  }

  // The curated set of common properties. Any other keys already in the file
  // are preserved untouched on save.
  const FIELDS: Field[] = [
    { key: "motd", label: "MOTD (server list message)", type: "text", def: "A Minecraft Server" },
    { key: "max-players", label: "Max players", type: "number", def: "20" },
    { key: "difficulty", label: "Difficulty", type: "select", def: "easy", options: ["peaceful", "easy", "normal", "hard"] },
    { key: "gamemode", label: "Game mode", type: "select", def: "survival", options: ["survival", "creative", "adventure", "spectator"] },
    { key: "hardcore", label: "Hardcore", type: "bool", def: "false" },
    { key: "pvp", label: "PvP", type: "bool", def: "true" },
    { key: "online-mode", label: "Online mode", type: "bool", def: "true", hint: "Off allows cracked/offline accounts to join." },
    { key: "white-list", label: "Whitelist", type: "bool", def: "false" },
    { key: "level-name", label: "World name", type: "text", def: "world" },
    { key: "level-seed", label: "World seed", type: "text", def: "" },
    { key: "server-port", label: "Server port", type: "number", def: "25565" },
    { key: "view-distance", label: "View distance", type: "number", def: "10" },
    { key: "simulation-distance", label: "Simulation distance", type: "number", def: "10" },
    { key: "spawn-protection", label: "Spawn protection", type: "number", def: "16" },
    { key: "allow-nether", label: "Allow Nether", type: "bool", def: "true" },
    { key: "allow-flight", label: "Allow flight", type: "bool", def: "false" },
    { key: "enable-command-block", label: "Command blocks", type: "bool", def: "false" },
  ];
  const MANAGED = new Set(FIELDS.map((f) => f.key));

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
      const t = raw.trimStart();
      if (!t || t.startsWith("#")) return { kind: "raw", raw };
      const eq = raw.indexOf("=");
      if (eq < 0) return { kind: "raw", raw };
      return { kind: "kv", key: raw.slice(0, eq).trim(), value: raw.slice(eq + 1), raw };
    });
  }

  async function load() {
    loading = true;
    try {
      const text = await api.readServerProperties(id);
      const lines = parse(text);
      rawLines = lines;
      const found: Record<string, string> = {};
      for (const l of lines) if (l.kind === "kv") found[l.key] = l.value;
      // Seed the form from the file, falling back to sensible defaults.
      const next: Record<string, string> = {};
      for (const f of FIELDS) next[f.key] = found[f.key] ?? f.def;
      values = next;
      loaded = true;
    } catch (e) {
      toast.error(String(e));
    } finally {
      loading = false;
    }
  }

  // (Re)load whenever the instance changes.
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
      const out = rawLines.map((l) => {
        if (l.kind === "kv" && MANAGED.has(l.key)) {
          seen.add(l.key);
          return `${l.key}=${values[l.key] ?? ""}`;
        }
        return l.raw;
      });
      // Append managed keys not already present (skip empty optional values).
      for (const f of FIELDS) {
        if (!seen.has(f.key) && (values[f.key] ?? "") !== "") {
          out.push(`${f.key}=${values[f.key]}`);
        }
      }
      let text = out.join("\n");
      if (!text.endsWith("\n")) text += "\n";
      await api.writeServerProperties(id, text);
      // Reflect the saved state so re-saving is stable.
      rawLines = parse(text);
      toast.success("Server properties saved.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      saving = false;
    }
  }
</script>

<div class="props">
  <div class="props-head">
    <p class="muted">
      Edit common <code>server.properties</code> settings.
      {#if running}
        <strong>Restart the server to apply changes.</strong>
      {/if}
    </p>
    <div class="head-actions">
      <button class="btn ghost sm" onclick={load} disabled={loading || saving}>Reload</button>
      <button class="btn primary sm" onclick={save} disabled={loading || saving || !loaded}>
        {saving ? "Saving…" : "Save"}
      </button>
    </div>
  </div>

  {#if loading && !loaded}
    <p class="muted">Loading…</p>
  {:else}
    <div class="grid">
      {#each FIELDS as f (f.key)}
        <div class="field" class:wide={f.type === "text"}>
          {#if f.type === "bool"}
            <label class="toggle">
              <input
                type="checkbox"
                checked={values[f.key] === "true"}
                onchange={(e) =>
                  (values[f.key] = (e.currentTarget as HTMLInputElement).checked ? "true" : "false")}
              />
              <span>{f.label}</span>
            </label>
          {:else}
            <label class="field-label" for={`p-${f.key}`}>{f.label}</label>
            {#if f.type === "select"}
              <select id={`p-${f.key}`} class="select" bind:value={values[f.key]}>
                {#each f.options ?? [] as o (o)}
                  <option value={o}>{o}</option>
                {/each}
              </select>
            {:else}
              <input
                id={`p-${f.key}`}
                class="input"
                type={f.type === "number" ? "number" : "text"}
                bind:value={values[f.key]}
              />
            {/if}
          {/if}
          {#if f.hint}<span class="hint">{f.hint}</span>{/if}
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
