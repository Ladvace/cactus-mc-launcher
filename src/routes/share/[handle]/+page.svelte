<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "$lib/components/Icon.svelte";
  import { streamerApi } from "$lib/streamerApi";
  import { streamerAuth } from "$lib/stores/streamerAuth.svelte";
  import { recordImport, importForHandle } from "$lib/importedFrom";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import type {
    ImportResult,
    SnapshotManifest,
    StreamerProfile,
  } from "$lib/types";

  const handle = $derived($page.params.handle);

  let profile = $state<StreamerProfile | null>(null);
  let manifest = $state<SnapshotManifest | null>(null);
  let prior = $state<{ snapshotId: string; importedAt: number } | null>(null);
  let loading = $state(true);
  let loadError = $state<string | null>(null);

  const alreadyHave = $derived(
    !!prior && prior.snapshotId === profile?.currentSnapshotId
  );
  const updateAvailable = $derived(
    !!prior &&
      !!profile?.currentSnapshotId &&
      prior.snapshotId !== profile.currentSnapshotId
  );

  function timeAgo(iso: string | null): string {
    if (!iso) return "";
    const s = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
    if (s < 90) return "just now";
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    return `${Math.round(h / 24)}d ago`;
  }

  let importing = $state(false);
  let progress = $state<{ phase: string; current: number; total: number } | null>(
    null
  );
  let result = $state<ImportResult | null>(null);
  let importError = $state<string | null>(null);

  $effect(() => {
    const h = handle;
    if (!h) return;
    if (!streamerApi.configured()) {
      loadError = "The streamer service isn't configured.";
      loading = false;
      return;
    }
    loading = true;
    loadError = null;
    manifest = null;
    prior = importForHandle(h)?.rec ?? null;
    streamerApi
      .profile(h)
      .then(async (p) => {
        profile = p;
        if (p.currentSnapshotId) {
          manifest = await streamerApi.snapshot(p.currentSnapshotId).catch(() => null);
        }
      })
      .catch((e) => (loadError = String(e)))
      .finally(() => (loading = false));
  });

  $effect(() => {
    let un: (() => void) | undefined;
    listen<{ phase: string; current: number; total: number }>(
      "snapshot-progress",
      (e) => (progress = e.payload)
    ).then((u) => (un = u));
    return () => un?.();
  });

  const pct = $derived(
    progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : null
  );

  // Reporting
  let reportOpen = $state(false);
  let reportReason = $state("");
  let reportBusy = $state(false);
  let reportDone = $state(false);

  async function submitReport() {
    const token = streamerAuth.token;
    if (!token || !reportReason.trim() || reportBusy) return;
    reportBusy = true;
    try {
      await streamerApi.report(token, handle!, reportReason.trim());
      reportDone = true;
      reportOpen = false;
      reportReason = "";
    } catch {
      /* swallow — reporting is best-effort */
    } finally {
      reportBusy = false;
    }
  }

  async function importSetup() {
    if (!profile?.currentSnapshotId || importing) return;
    importing = true;
    importError = null;
    result = null;
    try {
      const snapshotId = profile.currentSnapshotId;
      result = await streamerApi.importSnapshot(snapshotId);
      recordImport(result.instance.id, {
        handle: profile.handle,
        snapshotId,
        importedAt: Date.now(),
      });
      prior = { snapshotId, importedAt: Date.now() };
      await instancesStore.refresh();
    } catch (e) {
      importError = String(e);
    } finally {
      importing = false;
      progress = null;
    }
  }
</script>

<div class="page">
  <button class="back" onclick={() => goto("/share")}>← Streamers</button>

  {#if loading}
    <p class="muted">Loading…</p>
  {:else if loadError}
    <div class="missing">
      <p>{loadError}</p>
    </div>
  {:else if profile}
    <header class="prof">
      <span class="avatar"><Icon name="user" size={30} /></span>
      <div class="prof-body">
        <div class="name-row">
          <h1>{profile.displayName}</h1>
          {#if profile.isLive}
            <span class="live">● LIVE</span>
          {/if}
        </div>
        <p class="handle">@{profile.handle} · {profile.platform}</p>
        {#if profile.isLive && profile.currentActivity}
          <p class="activity">Playing {profile.currentActivity}</p>
        {/if}
      </div>
    </header>

    <section class="panel">
      <div class="setup-head">
        <h3>Their setup</h3>
        {#if manifest}
          <span class="ago">updated {timeAgo(manifest.createdAt)}</span>
        {/if}
      </div>
      {#if manifest?.changelog}
        <p class="changelog">“{manifest.changelog}”</p>
      {/if}
      {#if updateAvailable && !result}
        <p class="update-note">
          <Icon name="refresh" size={13} /> You imported an older version — the streamer
          has updated their setup.
        </p>
      {/if}
      {#if result}
        <div class="ok">
          <Icon name="check" size={16} />
          Imported “{result.instance.name}” — {result.installed} items{result
            .skipped.length
            ? `, ${result.skipped.length} skipped`
            : ""}.
          <button class="btn primary sm" onclick={() => goto(`/instance/${result?.instance.id}`)}>
            Open
          </button>
        </div>
      {:else if importing}
        <div class="importing">
          <span class="spinner"></span>
          <span>Importing… {progress?.phase === "installing" ? `${progress.current}/${progress.total}` : ""}</span>
          {#if pct !== null}
            <div class="bar"><div class="fill" style="width:{pct}%"></div></div>
          {/if}
        </div>
      {:else if profile.currentSnapshotId}
        {#if !alreadyHave}
          <p class="muted">
            One click installs their exact modpack, shaders, resource packs and
            keybinds as a new instance.
          </p>
        {:else}
          <p class="muted">You've already imported this version.</p>
        {/if}
        <button class="btn primary" onclick={importSetup}>
          <Icon name="download" size={16} />
          {updateAvailable
            ? "Update to latest"
            : alreadyHave
              ? "Re-import"
              : "Import this setup"}
        </button>
      {:else}
        <p class="muted">This streamer hasn't published a setup yet.</p>
      {/if}
      {#if importError}<p class="error">{importError}</p>{/if}
    </section>

    {#if profile.serverInfo}
      <section class="panel">
        <h3>Server</h3>
        <p class="server">{profile.serverInfo.address}</p>
        {#if profile.serverInfo.notes}
          <p class="muted">{profile.serverInfo.notes}</p>
        {/if}
        {#if profile.serverInfo.gated}
          <p class="muted">Access may be restricted (whitelist / subscribers).</p>
        {/if}
      </section>
    {/if}

    <div class="report">
      {#if reportDone}
        <span class="muted">Thanks — report submitted.</span>
      {:else if reportOpen}
        <textarea
          class="input"
          rows="2"
          placeholder="What's wrong with this profile? (impersonation, malicious pack…)"
          bind:value={reportReason}
        ></textarea>
        <div class="report-actions">
          <button class="btn ghost" onclick={() => (reportOpen = false)}>Cancel</button>
          <button class="btn danger" disabled={reportBusy || !reportReason.trim()} onclick={submitReport}>
            {reportBusy ? "…" : "Submit report"}
          </button>
        </div>
      {:else if streamerAuth.signedIn}
        <button class="flag" onclick={() => (reportOpen = true)}>Report this profile</button>
      {:else}
        <button class="flag" onclick={() => goto("/share/creator")}>Sign in to report</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page {
    padding: 24px 32px;
    max-width: 760px;
    margin: 0 auto;
  }
  .back {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    padding: 4px 0;
    margin-bottom: 16px;
  }
  .back:hover {
    color: var(--accent);
  }
  .muted {
    color: var(--text-muted);
  }
  .missing {
    padding: 48px;
    text-align: center;
    color: var(--text-muted);
  }
  .prof {
    display: flex;
    gap: 16px;
    align-items: center;
    margin-bottom: 22px;
  }
  .avatar {
    width: 68px;
    height: 68px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-card);
    border: 2px solid var(--border);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .name-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .prof h1 {
    font-size: 24px;
  }
  .live {
    font-family: var(--font-pixel);
    font-size: 12px;
    color: #fff;
    background: var(--danger);
    padding: 2px 8px;
  }
  .handle {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 13px;
  }
  .activity {
    margin: 6px 0 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .panel {
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.28);
    padding: 18px 20px;
    margin-bottom: 16px;
  }
  .panel h3 {
    font-size: 14px;
    margin-bottom: 10px;
  }
  .setup-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
  }
  .setup-head h3 {
    margin-bottom: 0;
  }
  .ago {
    font-size: 12px;
    color: var(--text-muted);
  }
  .changelog {
    margin: 8px 0 4px;
    font-size: 13px;
    color: var(--text-secondary);
    font-style: italic;
  }
  .update-note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 0;
    padding: 8px 10px;
    background: var(--accent-soft);
    border-left: 3px solid var(--accent);
    color: var(--accent);
    font-size: 12.5px;
  }
  .panel .muted {
    font-size: 13px;
    margin: 0 0 12px;
  }
  .server {
    font-family: var(--font-pixel);
    font-size: 16px;
    color: var(--accent);
    margin: 0 0 6px;
  }
  .btn.sm {
    padding: 5px 12px;
    font-size: 12px;
    margin-left: 8px;
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent);
    font-size: 13px;
    flex-wrap: wrap;
  }
  .importing {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .bar {
    flex-basis: 100%;
    height: 10px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s steps(16);
  }
  .spinner {
    width: 18px;
    height: 18px;
    border: 3px solid rgba(255, 255, 255, 0.25);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .error {
    color: var(--danger);
    font-size: 13px;
    margin: 10px 0 0;
  }
  .report {
    margin-top: 8px;
    text-align: center;
  }
  .flag {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
  }
  .flag:hover {
    color: var(--danger);
  }
  .report-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
</style>
