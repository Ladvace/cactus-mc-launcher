<script lang="ts">
  import { api } from "$lib/api";
  import { toast } from "$lib/stores/toast.svelte";
  import type { TuneupPlan } from "$lib/types";
  import Modal from "./Modal.svelte";

  interface Props {
    instanceId: string;
    open: boolean;
    onClose: () => void;
    onApplied?: () => void;
  }
  let { instanceId, open, onClose, onApplied }: Props = $props();

  let plan = $state<TuneupPlan | null>(null);
  let loading = $state(false);
  let applying = $state(false);
  let error = $state<string | null>(null);
  let mode = $state<"performance" | "visuals">("performance");

  // Which mods are ticked (by versionId), and whether to apply memory / flags.
  let picked = $state<Record<string, boolean>>({});
  let applyMemory = $state(true);
  let applyFlags = $state(true);
  // User-adjustable max heap (MB); seeded from the recommendation.
  let maxMem = $state(4096);

  // (Re)load the recommendation whenever the modal opens or the mode changes.
  $effect(() => {
    if (!open) return;
    const currentMode = mode;
    let cancelled = false;
    loading = true;
    error = null;
    plan = null;
    api
      .tuneupRecommend(instanceId, currentMode)
      .then((result) => {
        if (cancelled) return;
        plan = result;
        picked = Object.fromEntries(result.mods.map((m) => [m.versionId, m.recommended]));
        maxMem = result.maxMemMb;
      })
      .catch((e) => {
        if (!cancelled) error = String(e);
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  // Keep a sane min-heap relative to the chosen max.
  const minMem = $derived(Math.min(maxMem, Math.max(1024, Math.floor(maxMem / 2 / 512) * 512)));

  const chosenCount = $derived(
    plan ? plan.mods.filter((m) => !m.installed && picked[m.versionId]).length : 0,
  );
  const allModsInstalled = $derived(
    !!plan && plan.mods.length > 0 && plan.mods.every((m) => m.installed),
  );
  const nothingToDo = $derived(chosenCount === 0 && !applyMemory && !applyFlags);

  function gb(mb: number) {
    return (mb / 1024).toFixed(mb % 1024 === 0 ? 0 : 1);
  }

  async function apply() {
    if (!plan || nothingToDo) return;
    applying = true;
    try {
      const chosen = plan.mods.filter((m) => !m.installed && picked[m.versionId]);
      const count = await api.tuneupApply(instanceId, {
        mods: chosen.map((m) => ({ versionId: m.versionId, title: m.title })),
        applyMemory,
        applyFlags,
        maxMemMb: maxMem,
        minMemMb: minMem,
        jvmArgs: plan.jvmArgs,
      });
      const bits: string[] = [];
      if (count > 0) bits.push(`${count} mod${count === 1 ? "" : "s"}`);
      if (applyMemory) bits.push("memory");
      if (applyFlags) bits.push("JVM flags");
      toast.success(`Tuned up: applied ${bits.join(", ")}.`);
      onApplied?.();
      onClose();
    } catch (e) {
      toast.error(String(e));
    } finally {
      applying = false;
    }
  }
</script>

<Modal title="Tune-up" {open} {onClose} width={560}>
  {#if loading}
    <p class="muted">Inspecting your machine…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if plan}
    <p class="lead">
      Recommendations tailored to this machine and instance. Everything here is
      optional — review and untick anything you don't want.
    </p>

    <div class="modes" role="tablist" aria-label="Tune-up mode">
      <button
        role="tab"
        aria-selected={mode === "performance"}
        class:active={mode === "performance"}
        onclick={() => (mode = "performance")}
      >
        Performance
      </button>
      <button
        role="tab"
        aria-selected={mode === "visuals"}
        class:active={mode === "visuals"}
        onclick={() => (mode = "visuals")}
      >
        Visuals
      </button>
    </div>

    <div class="specs">
      <span><strong>{gb(plan.specs.totalRamMb)} GB</strong> RAM</span>
      <span><strong>{plan.specs.cpuCores}</strong> cores</span>
      <span>{plan.specs.os} · {plan.specs.arch}</span>
      <span>{plan.loader} {plan.mcVersion}</span>
    </div>

    {#if plan.loader === "vanilla"}
      <p class="muted">
        This is a vanilla instance, so there are no mods to add. You can still
        apply the memory and JVM tuning below.
      </p>
    {:else if plan.mods.length}
      <h4>Performance mods</h4>
      {#if allModsInstalled}
        <p class="muted small">All recommended mods are already installed. 🎉</p>
      {/if}
      <ul class="mods">
        {#each plan.mods as mod (mod.versionId)}
          <li class:installed={mod.installed}>
            <label>
              <input
                type="checkbox"
                checked={mod.installed ? true : picked[mod.versionId]}
                disabled={mod.installed}
                onchange={(e) => (picked[mod.versionId] = e.currentTarget.checked)}
              />
              <span class="mod-name">
                {mod.title}
                {#if mod.installed}<span class="badge">Installed</span>{/if}
              </span>
              <span class="mod-reason">{mod.reason}</span>
            </label>
          </li>
        {/each}
      </ul>
    {/if}

    {#if plan.unavailable.length}
      <p class="muted small">
        No {plan.loader} {plan.mcVersion} build yet: {plan.unavailable.join(", ")}.
      </p>
    {/if}

    <h4>Java tuning</h4>
    <label class="row">
      <input type="checkbox" bind:checked={applyMemory} />
      <span>Set memory allocation</span>
    </label>
    <div class="heap" class:disabled={!applyMemory}>
      <input
        type="range"
        min="1024"
        max={Math.max(2048, plan.specs.totalRamMb)}
        step="512"
        bind:value={maxMem}
        disabled={!applyMemory}
        aria-label="Maximum memory"
      />
      <span class="heap-val"><strong>{gb(maxMem)} GB</strong> max · {gb(minMem)} GB min</span>
    </div>
    <label class="row">
      <input type="checkbox" bind:checked={applyFlags} />
      <span>Apply tuned JVM flags{plan.jvmArgs.includes("MaxGCPauseMillis") ? " (Aikar's G1GC)" : ""}</span>
    </label>
  {/if}

  {#snippet footer()}
    <button class="btn ghost" onclick={onClose}>Cancel</button>
    <button class="btn primary" onclick={apply} disabled={!plan || applying || nothingToDo}>
      {applying ? "Applying…" : "Apply tune-up"}
    </button>
  {/snippet}
</Modal>

<style>
  .lead {
    margin: 0 0 0.75rem;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .modes {
    display: inline-flex;
    gap: 0;
    margin-bottom: 0.9rem;
    border: 1px solid var(--border);
    border-radius: var(--radius, 8px);
    overflow: hidden;
  }
  .modes button {
    padding: 0.35rem 0.9rem;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  .modes button.active {
    background: var(--accent);
    color: var(--accent-contrast, #1a1a1a);
    font-weight: 600;
  }
  .heap {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 0 0.35rem 1.7rem;
  }
  .heap.disabled {
    opacity: 0.45;
  }
  .heap input[type="range"] {
    flex: 1;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .heap-val {
    font-size: 0.85rem;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .specs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 0.75rem;
    padding: 0.6rem 0.75rem;
    background: var(--bg-elevated, var(--bg-card));
    border: 1px solid var(--border);
    border-radius: var(--radius, 8px);
    font-size: 0.85rem;
    margin-bottom: 1rem;
  }
  .specs span {
    color: var(--text-muted);
  }
  .specs strong {
    color: var(--text);
  }
  h4 {
    margin: 1rem 0 0.4rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  ul.mods {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  ul.mods label {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-areas: "check name" "check reason";
    gap: 0 0.6rem;
    align-items: center;
    cursor: pointer;
  }
  ul.mods input {
    grid-area: check;
  }
  .mod-name {
    grid-area: name;
    font-weight: 600;
  }
  .mod-reason {
    grid-area: reason;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  label.row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.25rem 0;
    cursor: pointer;
  }
  .muted {
    color: var(--text-muted);
  }
  .small {
    font-size: 0.8rem;
  }
  .error {
    color: var(--danger, #e5484d);
  }
</style>
