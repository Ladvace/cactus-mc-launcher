<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import Icon from "./Icon.svelte";
  import InstanceIcon from "./InstanceIcon.svelte";
  import { instancesStore } from "$lib/stores/instances.svelte";
  import { accountsStore } from "$lib/stores/accounts.svelte";
  import { ui } from "$lib/stores/ui.svelte";

  interface Props {
    onCreate: () => void;
  }
  let { onCreate }: Props = $props();

  const nav = [
    { href: "/", label: "Home", icon: "home" },
    { href: "/browse", label: "Browse", icon: "compass" },
    { href: "/library", label: "Library", icon: "library" },
  ];

  const path = $derived($page.url.pathname);
  // Pinned = the 6 most-recently-played instances.
  const pinned = $derived(instancesStore.instances.slice(0, 6));

  function isActive(href: string) {
    return href === "/" ? path === "/" : path.startsWith(href);
  }
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="brand-mark"><Icon name="cube" size={22} /></div>
    <span class="brand-name">Launcher</span>
  </div>

  <nav class="nav">
    {#each nav as item}
      <a href={item.href} class="nav-item" class:active={isActive(item.href)}>
        <Icon name={item.icon} size={18} />
        <span>{item.label}</span>
      </a>
    {/each}
  </nav>

  <div class="section">
    <div class="section-head">
      <span>Instances</span>
      <button class="icon-btn" title="Create instance" onclick={onCreate}>
        <Icon name="plus" size={16} />
      </button>
    </div>

    <div class="pinned">
      {#if pinned.length === 0}
        <p class="empty">No instances yet</p>
      {:else}
        {#each pinned as inst (inst.id)}
          <button
            class="pinned-item"
            class:active={path === `/instance/${inst.id}`}
            onclick={() => goto(`/instance/${inst.id}`)}
            title={inst.name}
          >
            <InstanceIcon instance={inst} size={26} />
            <span class="pinned-name">{inst.name}</span>
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <div class="footer">
    <a href="/settings" class="nav-item" class:active={isActive("/settings")}>
      <Icon name="settings" size={18} />
      <span>Settings</span>
    </a>
    <button class="account" onclick={() => ui.openAccounts()}>
      {#if accountsStore.active}
        <img
          class="avatar-img"
          src={`https://crafatar.com/avatars/${accountsStore.active.uuid}?size=30&overlay`}
          alt={accountsStore.active.username}
        />
      {:else}
        <span class="avatar"><Icon name="user" size={16} /></span>
      {/if}
      <span class="account-text">
        <span class="account-name">{accountsStore.activeName}</span>
        <span class="account-sub">
          {accountsStore.active ? "Microsoft" : "Offline"}
        </span>
      </span>
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    height: 100%;
    background: var(--bg-raised);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    padding: 12px 10px;
    gap: 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px 10px;
  }
  .brand-mark {
    color: var(--accent);
    display: flex;
  }
  .brand-name {
    font-weight: 700;
    font-size: 15px;
    letter-spacing: 0.01em;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-weight: 500;
    font-size: 13.5px;
    transition: background 0.12s, color 0.12s;
  }
  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--bg-hover);
    color: var(--text);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .section {
    margin-top: 6px;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    transition: background 0.12s, color 0.12s;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--accent);
  }

  .pinned {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: 2px;
  }
  .pinned-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    text-align: left;
    transition: background 0.12s, color 0.12s;
  }
  .pinned-item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .pinned-item.active {
    background: var(--bg-hover);
    color: var(--text);
  }
  .pinned-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }
  .empty {
    padding: 4px 10px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .footer {
    border-top: 1px solid var(--border-subtle);
    padding-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--text);
    text-align: left;
    transition: background 0.12s;
  }
  .account:hover {
    background: var(--bg-hover);
  }
  .avatar {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--bg-card);
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .avatar-img {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    flex-shrink: 0;
    object-fit: cover;
    image-rendering: pixelated;
    background: var(--bg-card);
  }
  .account-text {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .account-name {
    font-size: 13px;
    font-weight: 600;
  }
  .account-sub {
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
