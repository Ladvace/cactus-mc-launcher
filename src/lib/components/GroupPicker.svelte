<script lang="ts">
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { t } from "$lib/i18n";

  const inst = $derived(ui.groupFor);

  const groups = $derived(
    [
      ...new Set(
        instancesStore.instances
          .map((instance) => instance.group)
          .filter((group): group is string => !!group)
      ),
    ].sort((first, second) => first.localeCompare(second))
  );

  let newGroup = $state("");

  async function assign(group: string) {
    const target = inst;
    if (!target) return;
    ui.closeGroupPicker();
    await instancesStore.update(target.id, { group });
  }

  async function createAndAssign() {
    const groupName = newGroup.trim();
    if (!groupName) return;
    newGroup = "";
    await assign(groupName);
  }
</script>

<Modal
  title={t("group.moveTitle")}
  open={!!inst}
  onClose={() => ui.closeGroupPicker()}
  width={380}
>
  {#if inst}
    <p class="lead">{t("group.movePrefix")} <strong>{inst.name}</strong> {t("group.moveSuffix")}</p>

    <div class="options">
      <button class="opt" class:on={!inst.group} onclick={() => assign("")}>
        <Icon name="home" size={14} /> {t("group.noGroup")}
      </button>
      {#each groups as group (group)}
        <button class="opt" class:on={inst.group === group} onclick={() => assign(group)}>
          <Icon name="folder" size={14} />
          {group}
        </button>
      {/each}
    </div>

    <div class="new">
      <input
        class="input"
        placeholder={t("group.newGroupPlaceholder")}
        bind:value={newGroup}
        onkeydown={(event) => event.key === "Enter" && createAndAssign()}
      />
      <button class="btn primary" disabled={!newGroup.trim()} onclick={createAndAssign}>
        {t("common.create")}
      </button>
    </div>
  {/if}
</Modal>

<style>
  .lead {
    margin: 0 0 14px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .lead strong {
    color: var(--text);
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
    max-height: 220px;
    overflow-y: auto;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 9px 11px;
    background: var(--bg-input);
    border: 2px solid var(--border);
    color: var(--text-secondary);
    font-size: 13px;
    text-align: left;
  }
  .opt:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .opt.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .opt :global(.hn) {
    color: var(--text-muted);
  }
  .new {
    display: flex;
    gap: 8px;
  }
  .new .input {
    flex: 1;
  }
</style>
