<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "$lib/components/Icon.svelte";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import type { BoardCard } from "$lib/types";

  const ready = boardApi.configured();

  // The board session is tied to the active Minecraft account — acquire it
  // automatically (no separate login) and re-acquire if the account changes.
  const account = $derived(accountsStore.active);
  $effect(() => {
    const a = account;
    if (!ready || !a) return;
    if (
      !boardAuth.loggingIn &&
      (!boardAuth.signedIn || boardAuth.session?.uuid !== a.uuid)
    ) {
      boardAuth.login();
    }
  });

  let boards = $state<BoardCard[]>([]);
  let loadingBoards = $state(false);

  // New board form
  let handle = $state("");
  let displayName = $state("");
  let kind = $state<"creator" | "streamer" | "server">("creator");
  let description = $state("");
  let streamUrl = $state("");
  let serverAddress = $state("");
  let creating = $state(false);
  let createError = $state<string | null>(null);

  // Selected board + publish
  let selected = $state<string>("");
  let instanceId = $state("");
  let format = $state<"drakepack" | "mrpack">("drakepack");
  let changelog = $state("");
  let publishing = $state(false);
  let publishMsg = $state<string | null>(null);

  // Message
  let messageBody = $state("");
  let postingMsg = $state(false);

  $effect(() => {
    if (ready && boardAuth.signedIn) loadBoards();
  });

  async function loadBoards() {
    const token = boardAuth.token;
    if (!token) return;
    loadingBoards = true;
    try {
      boards = await boardApi.myBoards(token);
      if (!selected && boards.length) selected = boards[0].handle;
    } catch {
      boards = [];
    } finally {
      loadingBoards = false;
    }
  }

  async function createBoard() {
    const token = boardAuth.token;
    if (!token || creating) return;
    creating = true;
    createError = null;
    try {
      await boardApi.createBoard(token, {
        handle: handle.trim().toLowerCase(),
        displayName: displayName.trim() || handle.trim(),
        kind,
        description: description.trim() || undefined,
        streamUrl: streamUrl.trim() || undefined,
        serverAddress: serverAddress.trim() || undefined,
      });
      handle = "";
      displayName = "";
      description = "";
      streamUrl = "";
      serverAddress = "";
      await loadBoards();
    } catch (e) {
      createError = String(e);
    } finally {
      creating = false;
    }
  }

  async function publish() {
    const token = boardAuth.token;
    if (!token || !instanceId || publishing) return;
    publishing = true;
    publishMsg = null;
    try {
      await boardApi.publish(instanceId, format, token, {
        boardHandle: selected || undefined,
        changelog,
      });
      publishMsg = selected
        ? "Published to your board."
        : "Published (standalone).";
      changelog = "";
    } catch (e) {
      publishMsg = String(e);
    } finally {
      publishing = false;
    }
  }

  async function postMessage() {
    const token = boardAuth.token;
    if (!token || !selected || !messageBody.trim() || postingMsg) return;
    postingMsg = true;
    try {
      await boardApi.postMessage(token, selected, messageBody.trim());
      messageBody = "";
    } catch {
      /* ignore */
    } finally {
      postingMsg = false;
    }
  }
</script>

<div class="page">
  <button class="back" onclick={() => goto("/share")}>← Community</button>
  <h1>Create a board</h1>

  {#if !ready}
    <div class="panel">
      <p class="muted">The boards service isn't configured in this build (set <code>VITE_STREAMER_API_URL</code>).</p>
    </div>
  {:else if !account}
    <div class="panel">
      <h3>Add a Microsoft account</h3>
      <p class="muted">Boards are tied to your Minecraft account. Add one to get started.</p>
      <button class="btn primary" onclick={() => ui.openAccounts()}>
        <Icon name="user" size={15} /> Add account
      </button>
    </div>
  {:else if !boardAuth.signedIn || boardAuth.session?.uuid !== account.uuid}
    <div class="panel">
      <p class="muted">Preparing your creator session as <strong>{account.username}</strong>…</p>
      {#if boardAuth.error}
        <p class="err">{boardAuth.error}</p>
        <button class="btn ghost" onclick={() => boardAuth.login()}>Retry</button>
      {/if}
    </div>
  {:else}
    <div class="who">
      <span>Creating as <strong>{boardAuth.session?.name}</strong></span>
      <button class="link" onclick={() => ui.openAccounts()}>Switch account</button>
    </div>

    <!-- Existing boards + create -->
    <div class="panel">
      <h3>Your boards</h3>
      {#if loadingBoards}
        <p class="muted">Loading…</p>
      {:else if boards.length}
        <div class="chips">
          {#each boards as b (b.handle)}
            <button class="chip" class:on={selected === b.handle} onclick={() => (selected = b.handle)}>
              @{b.handle}
            </button>
          {/each}
        </div>
      {:else}
        <p class="muted">No boards yet — create one below.</p>
      {/if}
    </div>

    <div class="panel">
      <h3>New board</h3>
      <div class="form">
        <input class="input" placeholder="handle (a–z, 0–9, _)" bind:value={handle} />
        <input class="input" placeholder="Display name" bind:value={displayName} />
        <select class="select" bind:value={kind}>
          <option value="creator">Creator</option>
          <option value="streamer">Streamer</option>
          <option value="server">Server</option>
        </select>
      </div>
      <input class="input mt" placeholder="Short description (optional)" bind:value={description} />
      {#if kind === "streamer"}
        <input class="input mt" placeholder="Stream URL (twitch.tv/… or youtube.com/…)" bind:value={streamUrl} />
      {/if}
      {#if kind === "server"}
        <input class="input mt" placeholder="Server address (play.example.net)" bind:value={serverAddress} />
      {/if}
      <button class="btn primary mt" disabled={creating || !handle.trim()} onclick={createBoard}>
        {creating ? "Creating…" : "Create board"}
      </button>
      {#if createError}<p class="err">{createError}</p>{/if}
    </div>

    <!-- Publish an instance -->
    <div class="panel">
      <h3>Publish an instance</h3>
      <p class="muted">Snapshot one of your instances{selected ? ` to @${selected}` : " as a standalone shareable code"}.</p>
      <div class="form">
        <select class="select" bind:value={selected}>
          <option value="">Standalone (code only)</option>
          {#each boards as b (b.handle)}
            <option value={b.handle}>@{b.handle}</option>
          {/each}
        </select>
        <select class="select" bind:value={instanceId}>
          <option value="" disabled>Choose an instance…</option>
          {#each instancesStore.instances as i (i.id)}
            <option value={i.id}>{i.name} · {i.loader} {i.mcVersion}</option>
          {/each}
        </select>
        <select class="select" bind:value={format}>
          <option value="drakepack">.drakepack</option>
          <option value="mrpack">.mrpack</option>
        </select>
      </div>
      <input class="input mt" placeholder="What changed? (optional)" bind:value={changelog} />
      <button class="btn primary mt" disabled={publishing || !instanceId} onclick={publish}>
        {publishing ? "Publishing…" : "Publish"}
      </button>
      {#if publishMsg}<p class="ok">{publishMsg}</p>{/if}
    </div>

    <!-- Announcements -->
    {#if selected}
      <div class="panel">
        <h3>Post an announcement to @{selected}</h3>
        <textarea class="input" rows="2" placeholder="Say something to your followers…" bind:value={messageBody}></textarea>
        <button class="btn ghost mt" disabled={postingMsg || !messageBody.trim()} onclick={postMessage}>
          {postingMsg ? "Posting…" : "Post"}
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
  .who {
    display: flex;
    justify-content: space-between;
    align-items: center;
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
  }
  .form .input,
  .form .select {
    flex: 1;
    min-width: 150px;
  }
  .mt {
    margin-top: 10px;
  }
  .chips {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .chip {
    padding: 6px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 12.5px;
  }
  .chip.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .ok {
    color: var(--accent);
    font-size: 13px;
    margin: 10px 0 0;
  }
  .err {
    color: var(--danger);
    font-size: 13px;
    margin: 10px 0 0;
  }
</style>
