<script lang="ts">
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { THEME_PRESETS, type ThemePreset } from "$lib/themes";
  import { backgroundCss } from "$lib/background";
  import { ui } from "$lib/stores/ui.svelte";
  import { writeJson } from "$lib/storage";
  import Icon from "./Icon.svelte";

  let { onDone }: { onDone: () => void } = $props();

  const STEPS = 4;
  let step = $state(0);
  let dir = $state(1);
  let username = $state(settingsStore.settings.offlineUsername || "Player");

  const currentBg = $derived(settingsStore.settings.background ?? "");

  function next() {
    if (step < STEPS - 1) {
      dir = 1;
      step++;
    }
  }
  function back() {
    if (step > 0) {
      dir = -1;
      step--;
    }
  }

  function applyTheme(preset: ThemePreset) {
    settingsStore.save({
      ...settingsStore.settings,
      background: preset.bg,
      decorTheme: preset.decor ?? "",
    });
  }

  async function finish(createInstance: boolean) {
    await settingsStore.save({
      ...settingsStore.settings,
      offlineUsername: username.trim() || "Player",
    });
    writeJson("cactus:onboarded", true);
    onDone();
    if (createInstance) ui.openCreateInstance();
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Enter" && (step === 0 || step === 1)) next();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="onboard" transition:fly={{ y: 12, duration: 260 }}>
  <div class="glow"></div>

  <button class="skip" onclick={() => finish(false)}>Skip</button>

  <div class="card">
    {#key step}
      <div
        class="step"
        in:fly={{ x: dir * 32, duration: 260, easing: quintOut, opacity: 0 }}
      >
        {#if step === 0}
          <img class="mascot" src="/empty-cactus.png" alt="" />
          <h1>Welcome to <span class="brand">Cactus&nbsp;Launcher</span></h1>
          <p class="lead">
            A cozy home for your Minecraft worlds — arrange instances, install
            mods, and play together. Let's get you set up.
          </p>
          <button class="btn primary lg" onclick={next}>
            Get started <Icon name="play" size={15} />
          </button>
        {:else if step === 1}
          <span class="eyebrow">Your player</span>
          <h1>What should we call you?</h1>
          <p class="lead">Your in-game name for offline play. You can sign in with Microsoft anytime from the account menu.</p>
          <input
            class="name-input"
            placeholder="Player"
            maxlength="24"
            bind:value={username}
            onkeydown={(event) => event.key === "Enter" && next()}
          />
          <div class="nav">
            <button class="btn ghost" onclick={back}>Back</button>
            <button class="btn primary" onclick={next}>Continue</button>
          </div>
        {:else if step === 2}
          <span class="eyebrow">Make it yours</span>
          <h1>Pick a look</h1>
          <p class="lead">Choose a theme — you can change it later in Settings.</p>
          <div class="themes">
            {#each THEME_PRESETS as preset (preset.name)}
              <button
                class="theme"
                class:on={currentBg === preset.bg}
                title={preset.name}
                onclick={() => applyTheme(preset)}
              >
                <span class="swatch" style="background: {backgroundCss(preset.bg)};"></span>
                <span class="theme-name">{preset.name}</span>
              </button>
            {/each}
          </div>
          <div class="nav">
            <button class="btn ghost" onclick={back}>Back</button>
            <button class="btn primary" onclick={next}>Continue</button>
          </div>
        {:else}
          <img class="mascot" src="/empty-cactus.png" alt="" />
          <h1>You're all set{username.trim() ? `, ${username.trim()}` : ""}!</h1>
          <p class="lead">
            Create your first instance to start playing — Java is set up
            automatically the first time you launch.
          </p>
          <div class="finish">
            <button class="btn primary lg" onclick={() => finish(true)}>
              <Icon name="plus" size={15} /> Create my first instance
            </button>
            <button class="btn ghost" onclick={() => finish(false)}>
              I'll explore on my own
            </button>
          </div>
        {/if}
      </div>
    {/key}

    <div class="dots">
      {#each Array(STEPS) as _, index (index)}
        <span class="dot" class:on={index === step} class:done={index < step}></span>
      {/each}
    </div>
  </div>
</div>

<style>
  .onboard {
    position: fixed;
    inset: 0;
    z-index: 950;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background:
      radial-gradient(120% 120% at 50% 0%, color-mix(in srgb, var(--accent) 22%, transparent), transparent 60%),
      var(--bg-app);
    overflow: hidden;
  }
  /* Soft moving glow behind the card. */
  .glow {
    position: absolute;
    width: 640px;
    height: 640px;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--accent) 30%, transparent), transparent 70%);
    filter: blur(40px);
    opacity: 0.6;
    animation: drift 12s ease-in-out infinite;
    pointer-events: none;
  }
  @keyframes drift {
    0%, 100% { transform: translate(-10%, -8%) scale(1); }
    50% { transform: translate(10%, 6%) scale(1.1); }
  }
  .skip {
    position: absolute;
    top: 22px;
    right: 24px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 13px;
    z-index: 2;
  }
  .skip:hover {
    color: var(--text);
  }
  .card {
    position: relative;
    z-index: 1;
    width: min(560px, 100%);
    min-height: 420px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 4px;
    padding: 40px 40px 28px;
    background: var(--bg-card);
    border: 2px solid var(--border);
    box-shadow:
      inset 2px 2px 0 rgba(255, 255, 255, 0.04),
      inset -2px -2px 0 rgba(0, 0, 0, 0.3),
      0 24px 60px rgba(0, 0, 0, 0.45);
  }
  .step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    width: 100%;
  }
  .mascot {
    width: 180px;
    max-width: 60%;
    height: auto;
    image-rendering: pixelated;
    filter: drop-shadow(0 10px 18px rgba(0, 0, 0, 0.4));
    -webkit-user-drag: none;
  }
  h1 {
    font-size: 24px;
    line-height: 1.2;
    margin: 0;
  }
  .brand {
    color: var(--accent);
    font-family: var(--font-pixel);
  }
  .eyebrow {
    font-family: var(--font-pixel);
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent);
  }
  .lead {
    margin: 0;
    max-width: 42ch;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.6;
  }
  .btn.lg {
    padding: 12px 22px;
    font-size: 15px;
    margin-top: 8px;
  }
  .name-input {
    width: min(320px, 100%);
    margin-top: 4px;
    padding: 12px 14px;
    text-align: center;
    font-size: 18px;
    font-family: var(--font-pixel);
    color: var(--text);
    background: var(--bg-input);
    border: 2px solid var(--border);
    box-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.3);
  }
  .name-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
    width: 100%;
    margin: 4px 0;
  }
  .theme {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 6px;
    background: none;
    border: 2px solid transparent;
    cursor: pointer;
  }
  .theme.on {
    border-color: var(--accent);
  }
  .swatch {
    width: 100%;
    aspect-ratio: 1;
    border: 2px solid var(--border);
  }
  .theme-name {
    font-size: 10.5px;
    color: var(--text-secondary);
  }
  .theme.on .theme-name {
    color: var(--accent);
  }
  .nav,
  .finish {
    display: flex;
    gap: 10px;
    margin-top: 10px;
  }
  .finish {
    flex-direction: column;
    align-items: center;
    width: 100%;
  }
  .dots {
    display: flex;
    gap: 8px;
    margin-top: 24px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
    transition: background 0.2s, width 0.2s;
  }
  .dot.done {
    background: color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .dot.on {
    width: 22px;
    border-radius: 4px;
    background: var(--accent);
  }
</style>
