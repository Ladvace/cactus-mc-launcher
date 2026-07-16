<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "$lib/components/Icon.svelte";
  import { streamerApi, supabaseConfigured } from "$lib/streamerApi";
  import { streamerAuth } from "$lib/stores/streamerAuth.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import type { OwnedProfile } from "$lib/types";

  const ready = streamerApi.configured() && supabaseConfigured();

  let profile = $state<OwnedProfile | null>(null);
  let loadingMe = $state(false);

  // Claim form
  let handle = $state("");
  let displayName = $state("");
  let claiming = $state(false);
  let claimError = $state<string | null>(null);

  // Publish
  let selectedId = $state("");
  let format = $state<"drakepack" | "mrpack">("drakepack");
  let changelog = $state("");
  let publishing = $state(false);
  let publishError = $state<string | null>(null);
  let publishedSnapshot = $state<string | null>(null);

  // Share code
  let code = $state<string | null>(null);
  let codeBusy = $state(false);

  $effect(() => {
    if (ready && streamerAuth.signedIn) loadMe();
  });

  async function loadMe() {
    const token = streamerAuth.token;
    if (!token) return;
    loadingMe = true;
    try {
      profile = (await streamerApi.me(token)).profile;
    } catch {
      profile = null;
    } finally {
      loadingMe = false;
    }
  }

  async function claim() {
    const token = streamerAuth.token;
    if (!token || claiming) return;
    claiming = true;
    claimError = null;
    try {
      profile = await streamerApi.claimHandle(
        token,
        handle.trim().toLowerCase(),
        displayName.trim() || handle.trim()
      );
    } catch (e) {
      claimError = String(e);
    } finally {
      claiming = false;
    }
  }

  async function publish() {
    const token = streamerAuth.token;
    if (!token || !selectedId || publishing) return;
    publishing = true;
    publishError = null;
    publishedSnapshot = null;
    try {
      publishedSnapshot = await streamerApi.publish(
        selectedId,
        format,
        token,
        changelog
      );
      await loadMe();
    } catch (e) {
      publishError = String(e);
    } finally {
      publishing = false;
    }
  }

  async function makeCode() {
    const token = streamerAuth.token;
    if (!token || codeBusy) return;
    codeBusy = true;
    try {
      code = (await streamerApi.mintCode(token)).code;
    } catch (e) {
      code = null;
      publishError = String(e);
    } finally {
      codeBusy = false;
    }
  }
</script>

<div class="page">
  <button class="back" onclick={() => goto("/share")}>← Streamers</button>
  <h1>Creator</h1>

  {#if !ready}
    <div class="panel">
      <p class="muted">
        The streamer service isn't configured in this build (set
        <code>VITE_STREAMER_API_URL</code>, <code>VITE_SUPABASE_URL</code> and
        <code>VITE_SUPABASE_ANON_KEY</code>).
      </p>
    </div>
  {:else if !streamerAuth.signedIn}
    <div class="panel">
      <h3>Sign in to publish your setup</h3>
      <p class="muted">Verify with the platform you stream on. This proves the profile is yours.</p>
      <div class="signin">
        <button class="btn primary" disabled={streamerAuth.loggingIn} onclick={() => streamerAuth.login("twitch")}>
          <Icon name="video" size={15} /> Sign in with Twitch
        </button>
        <button class="btn ghost" disabled={streamerAuth.loggingIn} onclick={() => streamerAuth.login("google")}>
          Sign in with YouTube
        </button>
      </div>
      {#if streamerAuth.loggingIn}<p class="muted">Waiting for the browser…</p>{/if}
      {#if streamerAuth.error}<p class="error">{streamerAuth.error}</p>{/if}
    </div>
  {:else}
    <div class="who">
      <span>Signed in as <strong>{streamerAuth.session?.displayName || "streamer"}</strong></span>
      <button class="link" onclick={() => streamerAuth.logout()}>Sign out</button>
    </div>

    {#if loadingMe}
      <p class="muted">Loading…</p>
    {:else if !profile}
      <div class="panel">
        <h3>Claim your handle</h3>
        <p class="muted">This is your public URL, e.g. drake://ninja.</p>
        <div class="form">
          <input class="input" placeholder="handle (a–z, 0–9, _)" bind:value={handle} />
          <input class="input" placeholder="Display name" bind:value={displayName} />
          <button class="btn primary" disabled={claiming || !handle.trim()} onclick={claim}>
            {claiming ? "Claiming…" : "Claim handle"}
          </button>
        </div>
        {#if claimError}<p class="error">{claimError}</p>{/if}
      </div>
    {:else}
      <div class="panel">
        <h3>Your profile</h3>
        <p class="handle">@{profile.handle} · {profile.platform}{profile.isPublic ? "" : " · code-only"}</p>
        {#if profile.currentSnapshotId}
          <p class="muted">Current published setup: {profile.currentSnapshotId.slice(0, 8)}…</p>
        {:else}
          <p class="muted">No setup published yet.</p>
        {/if}
      </div>

      <div class="panel">
        <h3>Publish a setup</h3>
        <p class="muted">Snapshot one of your instances as your current setup for viewers to import.</p>
        <div class="form">
          <select class="select" bind:value={selectedId}>
            <option value="" disabled>Choose an instance…</option>
            {#each instancesStore.instances as i (i.id)}
              <option value={i.id}>{i.name} · {i.loader} {i.mcVersion}</option>
            {/each}
          </select>
          <select class="select" bind:value={format}>
            <option value="drakepack">.drakepack (full)</option>
            <option value="mrpack">.mrpack (Modrinth)</option>
          </select>
          <button class="btn primary" disabled={publishing || !selectedId} onclick={publish}>
            {publishing ? "Publishing…" : "Publish"}
          </button>
        </div>
        <input
          class="input changelog"
          placeholder="What changed? (shown to viewers, optional)"
          bind:value={changelog}
        />
        {#if publishedSnapshot}
          <p class="ok"><Icon name="check" size={14} /> Published as your current setup.</p>
        {/if}
        {#if publishError}<p class="error">{publishError}</p>{/if}
      </div>

      <div class="panel">
        <h3>Share code</h3>
        <p class="muted">A code viewers can paste — works even if your profile is private.</p>
        {#if code}
          <p class="code">{code}</p>
        {/if}
        <button class="btn ghost" disabled={codeBusy} onclick={makeCode}>
          {codeBusy ? "…" : code ? "New code" : "Create share code"}
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page {
    padding: 24px 32px;
    max-width: 720px;
    margin: 0 auto;
  }
  .back {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    padding: 4px 0;
    margin-bottom: 8px;
  }
  .back:hover {
    color: var(--accent);
  }
  h1 {
    font-size: 24px;
    margin-bottom: 18px;
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
    margin-bottom: 8px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 13px;
    margin: 0 0 12px;
  }
  .muted code {
    background: var(--bg-app);
    padding: 1px 5px;
    color: var(--accent);
  }
  .signin {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .who {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .link {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
  }
  .link:hover {
    color: var(--danger);
  }
  .form {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }
  .form .input,
  .form .select {
    flex: 1;
    min-width: 160px;
  }
  .changelog {
    margin-top: 10px;
  }
  .handle {
    font-family: var(--font-pixel);
    color: var(--accent);
    margin: 0 0 8px;
  }
  .code {
    font-family: var(--font-pixel);
    font-size: 20px;
    color: var(--accent);
    background: var(--bg-input);
    border: 2px solid var(--border);
    padding: 10px 14px;
    display: inline-block;
    margin: 0 0 12px;
    user-select: all;
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--accent);
    font-size: 13px;
  }
  .error {
    color: var(--danger);
    font-size: 13px;
    margin: 10px 0 0;
  }
</style>
