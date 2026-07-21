<script lang="ts">
  import { goto } from "$app/navigation";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Icon from "./Icon.svelte";
  import { boardApi } from "$lib/boardApi";
  import { boardAuth } from "$lib/stores/boardAuth.svelte";
  import { followedBoards } from "$lib/stores/followedBoards.svelte";
  import { recordImport } from "$lib/importedFrom";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { timeAgo } from "$lib/time";
  import { t } from "$lib/i18n";
  import ServerWeather from "./ServerWeather.svelte";
  import type { Board } from "$lib/types";

  let { handle }: { handle: string } = $props();

  let board = $state<Board | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let importingId = $state<string | null>(null);
  let importedIds = $state<string[]>([]);
  let reportOpen = $state(false);
  let reportReason = $state("");

  const following = $derived(board ? followedBoards.isFollowing(board.handle) : false);

  $effect(() => {
    const currentHandle = handle;
    if (!boardApi.configured()) {
      error = t("community.boardsServiceNotConfigured");
      loading = false;
      return;
    }
    loading = true;
    error = null;
    board = null;
    boardApi
      .board(currentHandle)
      .then((fetchedBoard) => (board = fetchedBoard))
      .catch((err) => (error = String(err)))
      .finally(() => (loading = false));
  });


  async function importInstance(id: string) {
    if (importingId) return;
    importingId = id;
    try {
      const result = await boardApi.importSnapshot(id);
      recordImport(result.instance.id, {
        handle: board!.handle,
        snapshotId: id,
        importedAt: Date.now(),
      });
      importedIds = [...importedIds, id];
      await instancesStore.refresh();
      toast.success(t("community.importedToast"));
    } catch (err) {
      toast.error(String(err));
    } finally {
      importingId = null;
    }
  }

  async function submitReport() {
    const token = boardAuth.token;
    if (!token || !reportReason.trim() || !board) return;
    try {
      await boardApi.report(token, board.handle, reportReason.trim());
      toast.success(t("community.reportSubmitted"));
    } catch (err) {
      toast.error(String(err));
    }
    reportOpen = false;
    reportReason = "";
  }
</script>

{#if loading}
  <p class="muted">{t("common.loading")}</p>
{:else if error}
  <p class="err">{error}</p>
{:else if board}
  <header class="board-head kind-{board.kind}">
    <div class="head-main">
      <span class="kind">{board.kind}</span>
      <h2>{board.displayName}</h2>
      <p class="by">@{board.handle} · by {board.ownerName}</p>
      {#if board.description}<p class="desc">{board.description}</p>{/if}
    </div>
    <div class="head-actions">
      <button
        class="btn ghost"
        onclick={() =>
          following
            ? followedBoards.unfollow(board!.handle)
            : followedBoards.follow(board!.handle)}
      >
        {following ? t("community.following") : t("community.follow")}
      </button>
    </div>
  </header>

  {#if board.kind === "streamer" && board.streamUrl}
    <button class="banner stream" onclick={() => openUrl(board!.streamUrl!)}>
      <Icon name="video" size={16} /> {t("community.watchStream")}
      <span class="ext">{t("community.opensInBrowser")}</span>
    </button>
  {/if}
  {#if board.kind === "server" && board.serverAddress}
    <div class="banner server">
      <Icon name="globe" size={16} />
      <code>{board.serverAddress}</code>
    </div>
    <div class="weather-strip">
      <ServerWeather address={board.serverAddress} />
    </div>
  {/if}

  {#if board.messages.length}
    <section class="messages">
      <h3>{t("community.announcements")}</h3>
      {#each board.messages as message (message.id)}
        <div class="msg">
          <p>{message.body}</p>
          <span class="when">{timeAgo(message.createdAt)}</span>
        </div>
      {/each}
    </section>
  {/if}

  <section class="insts">
    <h3>{t("community.instances")}</h3>
    {#if board.instances.length === 0}
      <p class="muted">{t("community.noInstancesPublished")}</p>
    {:else}
      <div class="grid">
        {#each board.instances as inst (inst.id)}
          <div class="inst">
            <div class="inst-body">
              <span class="name">{inst.name}</span>
              <span class="sub">{inst.modLoader ?? "vanilla"} · {inst.mcVersion ?? "?"}</span>
              {#if inst.changelog}<p class="cl">“{inst.changelog}”</p>{/if}
              <span class="ago">{t("community.updated")} {timeAgo(inst.createdAt)}</span>
            </div>
            <button
              class="btn primary sm"
              disabled={importingId === inst.id}
              onclick={() => importInstance(inst.id)}
            >
              {#if importingId === inst.id}
                {t("common.importing")}
              {:else if importedIds.includes(inst.id)}
                <Icon name="check" size={13} /> {t("community.importedLabel")}
              {:else}
                <Icon name="download" size={13} /> {t("common.import")}
              {/if}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <div class="report">
    {#if reportOpen}
      <textarea class="input" rows="2" placeholder={t("community.reportPlaceholder")} bind:value={reportReason}></textarea>
      <div class="report-actions">
        <button class="btn ghost" onclick={() => (reportOpen = false)}>{t("common.cancel")}</button>
        <button class="btn danger" disabled={!reportReason.trim()} onclick={submitReport}>{t("community.submitReport")}</button>
      </div>
    {:else if boardAuth.signedIn}
      <button class="flag" onclick={() => (reportOpen = true)}>{t("community.reportBoard")}</button>
    {/if}
  </div>
{/if}

<style>
  .muted {
    color: var(--text-muted);
  }
  .err {
    color: var(--danger);
  }
  .board-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    padding: 18px 20px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    border-left: 4px solid var(--accent);
    margin-bottom: 14px;
  }
  .kind-server {
    border-left-color: #5b8a3a;
  }
  .kind-creator {
    border-left-color: #9b59d0;
  }
  .kind {
    font-family: var(--font-pixel);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .board-head h2 {
    font-size: 22px;
    margin: 4px 0;
  }
  .by {
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .desc {
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--text-secondary);
    max-width: 60ch;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 12px 16px;
    margin-bottom: 14px;
    border: 2px solid var(--border);
    background: var(--bg-input);
    color: var(--text);
    font-size: 14px;
  }
  .banner.stream {
    border-color: #9147ff;
    color: #fff;
    background: linear-gradient(90deg, rgba(145, 71, 255, 0.25), transparent);
  }
  .banner .ext {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-muted);
  }
  .banner.server code {
    font-family: var(--font-pixel);
    color: var(--accent);
    user-select: all;
  }
  .weather-strip {
    margin: -8px 0 14px;
    padding: 8px 16px;
    border: 2px solid var(--border-subtle);
    border-top: none;
    background: var(--bg-card);
  }
  h3 {
    font-size: 14px;
    margin: 0 0 10px;
  }
  .messages {
    margin-bottom: 18px;
  }
  .msg {
    padding: 10px 12px;
    background: var(--bg-card);
    border: 2px solid var(--border-subtle);
    margin-bottom: 6px;
  }
  .msg p {
    margin: 0;
    font-size: 13px;
  }
  .msg .when {
    font-size: 11px;
    color: var(--text-muted);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
  }
  .inst {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--bg-card);
    border: 2px solid var(--border);
  }
  .inst-body {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .inst .name {
    font-weight: 600;
    font-size: 14px;
  }
  .inst .sub {
    font-size: 12px;
    color: var(--text-muted);
  }
  .inst .cl {
    margin: 4px 0 0;
    font-size: 12.5px;
    font-style: italic;
    color: var(--text-secondary);
  }
  .inst .ago {
    font-size: 11px;
    color: var(--text-muted);
  }
  .btn.sm {
    align-self: flex-start;
  }
  .report {
    margin-top: 18px;
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
