<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "$lib/components/Icon.svelte";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import type { OwnedBoard } from "$lib/types";

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

  let boards = $state<OwnedBoard[]>([]);
  let loadingBoards = $state(false);

  // One board per account.
  const myBoard = $derived(boards[0] ?? null);

  function timeAgo(iso: string): string {
    const s = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
    if (s < 90) return "just now";
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    return `${Math.round(h / 24)}d ago`;
  }

  // New board form
  let handle = $state("");
  let displayName = $state("");
  let kind = $state<"creator" | "streamer" | "server">("creator");
  let description = $state("");
  let streamUrl = $state("");
  let serverAddress = $state("");
  let creating = $state(false);

  // Publish target: "" = standalone (code only), or the board handle.
  let publishTo = $state<string>("");
  let instanceId = $state("");
  let format = $state<"cactuspack" | "mrpack">("cactuspack");
  let changelog = $state("");
  let publishing = $state(false);

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
      if (!publishTo && boards.length) publishTo = boards[0].handle;
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
      toast.success("Board created.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      creating = false;
    }
  }

  async function publish() {
    const token = boardAuth.token;
    if (!token || !instanceId || publishing) return;
    publishing = true;
    try {
      await boardApi.publish(instanceId, format, token, {
        boardHandle: publishTo || undefined,
        changelog,
      });
      changelog = "";
      toast.success(
        publishTo ? "Published to your board." : "Published as a standalone code."
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      publishing = false;
    }
  }

  async function postMessage() {
    const token = boardAuth.token;
    const handle = myBoard?.handle;
    if (!token || !handle || !messageBody.trim() || postingMsg) return;
    postingMsg = true;
    try {
      await boardApi.postMessage(token, handle, messageBody.trim());
      messageBody = "";
      await loadBoards();
      toast.success("Announcement posted.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      postingMsg = false;
    }
  }

  let deletingMsg = $state<string | null>(null);
  async function deleteMessage(id: string) {
    const token = boardAuth.token;
    const handle = myBoard?.handle;
    if (!token || !handle || deletingMsg) return;
    deletingMsg = id;
    try {
      await boardApi.deleteMessage(token, handle, id);
      await loadBoards();
      toast.success("Announcement deleted.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      deletingMsg = null;
    }
  }

  // --- Edit / delete the selected board ---
  let eName = $state("");
  let eDesc = $state("");
  let eStream = $state("");
  let eServer = $state("");
  let ePublic = $state(true);
  let savingEdit = $state(false);
  let confirmDelete = $state(false);

  $effect(() => {
    const b = myBoard;
    confirmDelete = false;
    if (!b) return;
    eName = b.displayName;
    eDesc = b.description ?? "";
    eStream = b.streamUrl ?? "";
    eServer = b.serverAddress ?? "";
    ePublic = b.isPublic;
  });

  async function saveEdit() {
    const token = boardAuth.token;
    const handle = myBoard?.handle;
    if (!token || !handle || savingEdit) return;
    savingEdit = true;
    try {
      await boardApi.updateBoard(token, handle, {
        displayName: eName.trim(),
        description: eDesc.trim(),
        streamUrl: eStream.trim(),
        serverAddress: eServer.trim(),
        isPublic: ePublic,
      });
      await loadBoards();
      toast.success("Board saved.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      savingEdit = false;
    }
  }

  async function deleteBoard() {
    const token = boardAuth.token;
    const handle = myBoard?.handle;
    if (!token || !handle) return;
    try {
      await boardApi.deleteBoard(token, handle);
      publishTo = "";
      confirmDelete = false;
      await loadBoards();
      toast.success("Board deleted.");
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

<div class="page">
  <button class="back" onclick={() => goto("/share")}>← Community</button>
  <h1>{myBoard ? "Your board" : "Create a board"}</h1>

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
      <span>Signed in as <strong>{boardAuth.session?.name}</strong></span>
      <button class="link" onclick={() => ui.openAccounts()}>Switch account</button>
    </div>

    {#if loadingBoards}
      <div class="panel"><p class="muted">Loading…</p></div>
    {/if}

    <!-- Edit / delete your board -->
    {#if myBoard}
      <div class="panel">
        <div class="edit-head">
          <h3>Your board — @{myBoard.handle}</h3>
          <button class="link view" onclick={() => goto(`/share/${myBoard.handle}`)}>View public page ↗</button>
        </div>
        <input class="input" placeholder="Display name" bind:value={eName} />
        <input class="input mt" placeholder="Description (optional)" bind:value={eDesc} />
        {#if myBoard.kind === "streamer"}
          <input class="input mt" placeholder="Stream URL (twitch.tv/… or youtube.com/…)" bind:value={eStream} />
        {/if}
        {#if myBoard.kind === "server"}
          <input class="input mt" placeholder="Server address (play.example.net)" bind:value={eServer} />
        {/if}
        <label class="chk mt">
          <input type="checkbox" bind:checked={ePublic} />
          Public — searchable in Discover
        </label>
        <div class="edit-actions mt">
          <button class="btn primary" disabled={savingEdit} onclick={saveEdit}>
            {savingEdit ? "Saving…" : "Save changes"}
          </button>
          {#if confirmDelete}
            <button class="btn danger" onclick={deleteBoard}>Confirm delete</button>
            <button class="btn ghost" onclick={() => (confirmDelete = false)}>Cancel</button>
          {:else}
            <button class="btn danger" onclick={() => (confirmDelete = true)}>Delete board</button>
          {/if}
        </div>
      </div>
    {/if}

    {#if !boards.length && !loadingBoards}
      <div class="panel">
        <h3>Create your board</h3>
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
      </div>
    {/if}

    <!-- Publish an instance -->
    <div class="panel">
      <h3>Publish an instance</h3>
      <p class="muted">Snapshot one of your instances{publishTo ? ` to @${publishTo}` : " as a standalone shareable code"}.</p>
      <div class="form">
        <select class="select" bind:value={publishTo}>
          <option value="">Standalone (code only)</option>
          {#if myBoard}
            <option value={myBoard.handle}>@{myBoard.handle}</option>
          {/if}
        </select>
        <select class="select" bind:value={instanceId}>
          <option value="" disabled>Choose an instance…</option>
          {#each instancesStore.instances as i (i.id)}
            <option value={i.id}>{i.name} · {i.loader} {i.mcVersion}</option>
          {/each}
        </select>
        <select class="select" bind:value={format}>
          <option value="cactuspack">.cactuspack</option>
          <option value="mrpack">.mrpack</option>
        </select>
      </div>
      <input class="input mt" placeholder="What changed? (optional)" bind:value={changelog} />
      <button class="btn primary mt" disabled={publishing || !instanceId} onclick={publish}>
        {publishing ? "Publishing…" : "Publish"}
      </button>
    </div>

    <!-- Announcements -->
    {#if myBoard}
      <div class="panel">
        <h3>Post an announcement to @{myBoard.handle}</h3>
        <textarea class="input" rows="2" placeholder="Say something to your followers…" bind:value={messageBody}></textarea>
        <button class="btn ghost mt" disabled={postingMsg || !messageBody.trim()} onclick={postMessage}>
          {postingMsg ? "Posting…" : "Post"}
        </button>

        {#if myBoard.messages.length}
          <ul class="msgs">
            {#each myBoard.messages as m (m.id)}
              <li class="msg">
                <div class="msg-body">
                  <p>{m.body}</p>
                  <span class="when">{timeAgo(m.createdAt)}</span>
                </div>
                <button
                  class="msg-del"
                  title="Delete announcement"
                  aria-label="Delete announcement"
                  disabled={deletingMsg === m.id}
                  onclick={() => deleteMessage(m.id)}
                >
                  {#if deletingMsg === m.id}…{:else}<Icon name="trash" size={14} />{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
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
  .link.view:hover {
    color: var(--accent);
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
  .msgs {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .msg {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    background: var(--bg-input);
    border: 2px solid var(--border-subtle);
  }
  .msg-body {
    flex: 1;
    min-width: 0;
  }
  .msg-body p {
    margin: 0;
    font-size: 13px;
    word-break: break-word;
  }
  .msg-body .when {
    font-size: 11px;
    color: var(--text-muted);
  }
  .msg-del {
    flex-shrink: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 2px 4px;
    line-height: 1;
  }
  .msg-del:hover:not(:disabled) {
    color: var(--danger);
  }
  .msg-del:disabled {
    opacity: 0.5;
  }
  .edit-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .edit-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .err {
    color: var(--danger);
    font-size: 13px;
    margin: 10px 0 0;
  }
</style>
