<script lang="ts">
  import { goto } from "$app/navigation";
  import Icon from "$lib/components/Icon.svelte";
  import Select from "$lib/components/Select.svelte";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { t } from "$lib/i18n";
  import type { OwnedBoard } from "$lib/types";

  const ready = boardApi.configured();

  const account = $derived(accountsStore.active);
  $effect(() => {
    const activeAccount = account;
    if (!ready || !activeAccount) return;
    // Don't auto-retry after a failure (avoids an endless "Connecting…" loop).
    if (
      !boardAuth.loggingIn &&
      !boardAuth.error &&
      (!boardAuth.signedIn || boardAuth.session?.uuid !== activeAccount.uuid)
    ) {
      boardAuth.login();
    }
  });

  let boards = $state<OwnedBoard[]>([]);
  let loadingBoards = $state(false);

  const myBoard = $derived(boards[0] ?? null);

  const kindOptions = $derived([
    { value: "creator", label: t("community.kindCreator") },
    { value: "streamer", label: t("community.kindStreamer") },
    { value: "server", label: t("community.kindServer") },
  ]);
  const publishToOptions = $derived([
    { value: "", label: t("community.standaloneCodeOnly") },
    ...(myBoard ? [{ value: myBoard.handle, label: `@${myBoard.handle}` }] : []),
  ]);
  const instanceOptions = $derived([
    { value: "", label: t("community.chooseInstance"), disabled: true },
    ...instancesStore.instances.map((instance) => ({
      value: instance.id,
      label: `${instance.name} · ${instance.loader} ${instance.mcVersion}`,
    })),
  ]);
  const formatOptions = [
    { value: "cactuspack", label: ".cactuspack" },
    { value: "mrpack", label: ".mrpack" },
  ];

  function timeAgo(iso: string): string {
    const seconds = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
    if (seconds < 90) return t("community.justNow");
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return t("community.minutesAgo", { count: minutes });
    const hours = Math.round(minutes / 60);
    if (hours < 24) return t("community.hoursAgo", { count: hours });
    return t("community.daysAgo", { count: Math.round(hours / 24) });
  }

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
      toast.success(t("community.boardCreated"));
    } catch (err) {
      toast.error(String(err));
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
        publishTo ? t("community.publishedToBoard") : t("community.publishedStandalone")
      );
    } catch (err) {
      toast.error(String(err));
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
      toast.success(t("community.announcementPosted"));
    } catch (err) {
      toast.error(String(err));
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
      toast.success(t("community.announcementDeleted"));
    } catch (err) {
      toast.error(String(err));
    } finally {
      deletingMsg = null;
    }
  }

  let editName = $state("");
  let editDesc = $state("");
  let editStream = $state("");
  let editServer = $state("");
  let editPublic = $state(true);
  let savingEdit = $state(false);
  let confirmDelete = $state(false);

  $effect(() => {
    const board = myBoard;
    confirmDelete = false;
    if (!board) return;
    editName = board.displayName;
    editDesc = board.description ?? "";
    editStream = board.streamUrl ?? "";
    editServer = board.serverAddress ?? "";
    editPublic = board.isPublic;
  });

  async function saveEdit() {
    const token = boardAuth.token;
    const handle = myBoard?.handle;
    if (!token || !handle || savingEdit) return;
    savingEdit = true;
    try {
      await boardApi.updateBoard(token, handle, {
        displayName: editName.trim(),
        description: editDesc.trim(),
        streamUrl: editStream.trim(),
        serverAddress: editServer.trim(),
        isPublic: editPublic,
      });
      await loadBoards();
      toast.success(t("community.boardSaved"));
    } catch (err) {
      toast.error(String(err));
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
      toast.success(t("community.boardDeleted"));
    } catch (err) {
      toast.error(String(err));
    }
  }
</script>

<div class="page">
  <button class="back" onclick={() => goto("/share")}>← {t("nav.community")}</button>
  <h1>{myBoard ? t("community.yourBoard") : t("community.createBoard")}</h1>

  {#if !ready}
    <div class="panel">
      <p class="muted">{t("community.boardsNotConfigured")}</p>
    </div>
  {:else if !account}
    <div class="panel">
      <h3>{t("community.addMicrosoftAccount")}</h3>
      <p class="muted">{t("community.boardsTiedToAccount")}</p>
      <button class="btn primary" onclick={() => ui.openAccounts()}>
        <Icon name="user" size={15} /> {t("community.addAccount")}
      </button>
    </div>
  {:else if !boardAuth.signedIn || boardAuth.session?.uuid !== account.uuid}
    {#if boardAuth.error}
      <div class="panel">
        <p class="muted">{t("community.creatorSessionFailedPrefix")} <strong>{account.username}</strong>.</p>
        <p class="err">{boardAuth.error}</p>
        <button class="btn ghost" onclick={() => boardAuth.login()}>{t("common.retry")}</button>
      </div>
    {:else}
      <div class="panel board-skeleton">
        <span class="skeleton" style="width:44px;height:44px;border-radius:10px"></span>
        <div class="board-skel-lines">
          <span class="skeleton" style="width:40%;height:15px"></span>
          <span class="skeleton" style="width:70%;height:11px"></span>
        </div>
      </div>
    {/if}
  {:else}
    <div class="who">
      <span>{t("community.signedInAs")} <strong>{boardAuth.session?.name}</strong></span>
      <button class="link" onclick={() => ui.openAccounts()}>{t("community.switchAccount")}</button>
    </div>

    {#if loadingBoards}
      <div class="panel board-skeleton">
        <span class="skeleton" style="width:44px;height:44px;border-radius:10px"></span>
        <div class="board-skel-lines">
          <span class="skeleton" style="width:40%;height:15px"></span>
          <span class="skeleton" style="width:70%;height:11px"></span>
        </div>
      </div>
    {/if}

    {#if myBoard}
      <div class="panel">
        <div class="edit-head">
          <h3>{t("community.yourBoardHandle", { handle: myBoard.handle })}</h3>
          <button class="link view" onclick={() => goto(`/share/${myBoard.handle}`)}>{t("community.viewPublicPage")}</button>
        </div>
        <input class="input" placeholder={t("community.displayNamePlaceholder")} bind:value={editName} />
        <input class="input mt" placeholder={t("community.descriptionOptional")} bind:value={editDesc} />
        {#if myBoard.kind === "streamer"}
          <input class="input mt" placeholder={t("community.streamUrlPlaceholder")} bind:value={editStream} />
        {/if}
        {#if myBoard.kind === "server"}
          <input class="input mt" placeholder={t("community.serverAddressPlaceholder")} bind:value={editServer} />
        {/if}
        <label class="chk mt">
          <input type="checkbox" bind:checked={editPublic} />
          {t("community.publicSearchable")}
        </label>
        <div class="edit-actions mt">
          <button class="btn primary" disabled={savingEdit} onclick={saveEdit}>
            {savingEdit ? t("community.saving") : t("community.saveChanges")}
          </button>
          {#if confirmDelete}
            <button class="btn danger" onclick={deleteBoard}>{t("community.confirmDelete")}</button>
            <button class="btn ghost" onclick={() => (confirmDelete = false)}>{t("common.cancel")}</button>
          {:else}
            <button class="btn danger" onclick={() => (confirmDelete = true)}>{t("community.deleteBoard")}</button>
          {/if}
        </div>
      </div>
    {/if}

    {#if !boards.length && !loadingBoards}
      <div class="panel">
        <h3>{t("community.createYourBoard")}</h3>
        <div class="form">
          <input class="input" placeholder={t("community.handlePlaceholder")} bind:value={handle} />
          <input class="input" placeholder={t("community.displayNamePlaceholder")} bind:value={displayName} />
          <Select bind:value={kind} options={kindOptions} width="160px" />
        </div>
        <input class="input mt" placeholder={t("community.shortDescriptionOptional")} bind:value={description} />
        {#if kind === "streamer"}
          <input class="input mt" placeholder={t("community.streamUrlPlaceholder")} bind:value={streamUrl} />
        {/if}
        {#if kind === "server"}
          <input class="input mt" placeholder={t("community.serverAddressPlaceholder")} bind:value={serverAddress} />
        {/if}
        <button class="btn primary mt" disabled={creating || !handle.trim()} onclick={createBoard}>
          {creating ? t("common.creating") : t("community.createBoardButton")}
        </button>
      </div>
    {/if}

    <div class="panel">
      <h3>{t("community.publishInstance")}</h3>
      <p class="muted">{publishTo ? t("community.snapshotToBoard", { handle: publishTo }) : t("community.snapshotStandalone")}</p>
      <div class="form">
        <Select bind:value={publishTo} options={publishToOptions} width="160px" />
        <Select bind:value={instanceId} options={instanceOptions} width="160px" />
        <Select bind:value={format} options={formatOptions} width="160px" />
      </div>
      <input class="input mt" placeholder={t("community.changelogPlaceholder")} bind:value={changelog} />
      <button class="btn primary mt" disabled={publishing || !instanceId} onclick={publish}>
        {publishing ? t("community.publishing") : t("community.publish")}
      </button>
    </div>

    {#if myBoard}
      <div class="panel">
        <h3>{t("community.postAnnouncementTo", { handle: myBoard.handle })}</h3>
        <textarea class="input" rows="2" placeholder={t("community.announcementPlaceholder")} bind:value={messageBody}></textarea>
        <button class="btn ghost mt" disabled={postingMsg || !messageBody.trim()} onclick={postMessage}>
          {postingMsg ? t("community.posting") : t("community.post")}
        </button>

        {#if myBoard.messages.length}
          <ul class="msgs">
            {#each myBoard.messages as message (message.id)}
              <li class="msg">
                <div class="msg-body">
                  <p>{message.body}</p>
                  <span class="when">{timeAgo(message.createdAt)}</span>
                </div>
                <button
                  class="msg-del"
                  title={t("community.deleteAnnouncement")}
                  aria-label={t("community.deleteAnnouncement")}
                  disabled={deletingMsg === message.id}
                  onclick={() => deleteMessage(message.id)}
                >
                  {#if deletingMsg === message.id}…{:else}<Icon name="trash" size={14} />{/if}
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
  .board-skeleton {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .board-skel-lines {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
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
  .form .input {
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
