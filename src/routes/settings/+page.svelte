<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { api } from "$lib/api";
  import { listen } from "@tauri-apps/api/event";
  import type { Settings } from "$lib/types";

  // Local editable copy; committed on "Save".
  let draft = $state<Settings>({ ...settingsStore.settings });
  let saved = $state(false);
  let saving = $state(false);

  // Sync the draft once settings finish loading.
  $effect(() => {
    if (settingsStore.loaded) {
      draft = { ...settingsStore.settings };
    }
  });

  // --- Managed Java auto-setup ---
  let javaBusy = $state(false);
  let javaLabel = $state("");
  let javaCur = $state(0);
  let javaTotal = $state(0);
  let javaInstalled = $state<string[]>([]);
  let javaError = $state<string | null>(null);

  const javaPct = $derived(
    javaTotal > 0 ? Math.round((javaCur / javaTotal) * 100) : null
  );

  $effect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ label: string; current: number; total: number }>(
      "java-setup",
      (e) => {
        javaLabel = e.payload.label;
        javaCur = e.payload.current;
        javaTotal = e.payload.total;
      }
    ).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  async function setupJava() {
    javaBusy = true;
    javaError = null;
    javaInstalled = [];
    javaLabel = "Fetching runtime list…";
    javaCur = 0;
    javaTotal = 0;
    try {
      javaInstalled = await api.setupJava();
    } catch (e) {
      javaError = String(e);
    } finally {
      javaBusy = false;
      javaLabel = "";
      javaCur = 0;
      javaTotal = 0;
    }
  }

  async function save() {
    saving = true;
    try {
      await settingsStore.save({ ...draft });
      saved = true;
      setTimeout(() => (saved = false), 1800);
    } finally {
      saving = false;
    }
  }
</script>

<div class="page">
  <header class="head">
    <h1>Settings</h1>
    <div class="head-actions">
      {#if saved}<span class="saved">Saved ✓</span>{/if}
      <button class="btn primary" onclick={save} disabled={saving}>
        {saving ? "Saving…" : "Save changes"}
      </button>
    </div>
  </header>

  <section class="card-block">
    <h3>Account</h3>
    <div class="setting">
      <div class="label">
        <span>Offline username</span>
        <small>Used for offline launches until Microsoft sign-in is added.</small>
      </div>
      <input
        class="input narrow"
        placeholder="Player"
        bind:value={draft.offlineUsername}
        autocomplete="off"
      />
    </div>
  </section>

  <section class="card-block">
    <h3>Appearance</h3>
    <div class="setting">
      <div class="label">
        <span>Theme</span>
        <small>Only dark is implemented for now.</small>
      </div>
      <select class="select narrow" bind:value={draft.theme}>
        <option value="dark">Dark</option>
        <option value="light">Light</option>
        <option value="system">System</option>
      </select>
    </div>
  </section>

  <section class="card-block">
    <h3>Java & Memory</h3>
    <div class="setting">
      <div class="label">
        <span>Managed Java</span>
        <small>
          Pre-download the runtimes Minecraft needs (Java 8, 17 & 21). Used
          automatically at launch — you don't need to set a path below.
        </small>
      </div>
      <button class="btn primary" onclick={setupJava} disabled={javaBusy}>
        {javaBusy ? "Installing…" : "Auto-install Java"}
      </button>
    </div>
    {#if javaBusy || javaInstalled.length > 0 || javaError}
      <div class="java-status">
        {#if javaBusy}
          <div class="progress-head">
            <span>{javaLabel || "Preparing…"}</span>
            {#if javaPct !== null}<span>{javaPct}%</span>{/if}
          </div>
          <div class="bar">
            <div
              class="bar-fill"
              class:indeterminate={javaPct === null}
              style={javaPct !== null ? `width:${javaPct}%` : ""}
            ></div>
          </div>
        {:else if javaError}
          <p class="java-err">{javaError}</p>
        {:else}
          <p class="java-ok">✓ Ready: {javaInstalled.join(", ")}</p>
        {/if}
      </div>
    {/if}
    <div class="setting">
      <div class="label">
        <span>Java path (optional)</span>
        <small>
          Leave empty to use managed Java. Only set this to force your own Java
          executable.
        </small>
      </div>
      <input
        class="input narrow"
        placeholder="/path/to/java"
        bind:value={draft.javaPath}
      />
    </div>
    <div class="setting">
      <div class="label">
        <span>Maximum memory</span>
        <small>{draft.maxMemoryMb} MB</small>
      </div>
      <input
        type="range"
        min="1024"
        max="16384"
        step="512"
        bind:value={draft.maxMemoryMb}
        class="range"
      />
    </div>
    <div class="setting">
      <div class="label">
        <span>JVM arguments</span>
        <small>Extra flags passed to the JVM at launch.</small>
      </div>
      <input
        class="input narrow"
        placeholder="-XX:+UseG1GC …"
        bind:value={draft.jvmArgs}
      />
    </div>
  </section>

  <section class="card-block">
    <h3>Game window</h3>
    <div class="setting">
      <div class="label"><span>Default resolution</span></div>
      <div class="res">
        <input type="number" class="input tiny" bind:value={draft.gameWidth} />
        <span>×</span>
        <input type="number" class="input tiny" bind:value={draft.gameHeight} />
      </div>
    </div>
  </section>
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 760px;
    margin: 0 auto;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }
  .head h1 {
    font-size: 24px;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .saved {
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
  }
  .card-block {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: 18px 20px;
    margin-bottom: 18px;
  }
  .card-block h3 {
    font-size: 14px;
    margin-bottom: 8px;
  }
  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 14px 0;
    border-top: 1px solid var(--border-subtle);
  }
  .card-block h3 + .setting {
    border-top: none;
  }
  .label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .label span {
    font-size: 13.5px;
    font-weight: 500;
  }
  .label small {
    color: var(--text-muted);
    font-size: 12px;
  }
  .narrow {
    width: 240px;
    flex-shrink: 0;
  }
  .range {
    width: 240px;
    accent-color: var(--accent);
  }
  .res {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }
  .tiny {
    width: 90px;
  }

  /* Managed Java status */
  .java-status {
    padding: 0 0 14px;
    max-width: 100%;
  }
  .progress-head {
    display: flex;
    justify-content: space-between;
    font-size: 12.5px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .bar {
    height: 8px;
    background: var(--bg-input);
    border-radius: 6px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 6px;
    transition: width 0.2s ease;
  }
  .bar-fill.indeterminate {
    width: 35%;
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    0% {
      margin-left: -35%;
    }
    100% {
      margin-left: 100%;
    }
  }
  .java-ok {
    margin: 0;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
  }
  .java-err {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }
</style>
