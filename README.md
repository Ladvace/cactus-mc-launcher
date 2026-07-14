# Minecraft Launcher

A custom Minecraft launcher (Modrinth-style) built with **Tauri v2**, **SvelteKit**, **Svelte 5 (runes)**, and **Rust**.

## Running

```bash
bun install
bun run tauri dev      # launches the desktop app
```

Other useful scripts:

```bash
bun run check          # svelte-check (type-check the frontend)
bun run build          # build the SvelteKit SPA
```

## Architecture

### Frontend (`src/`)
- `lib/types.ts` — TypeScript mirrors of the Rust types (keep in sync).
- `lib/api.ts` — typed wrapper around the Rust command layer (`invoke`).
- `lib/stores/*.svelte.ts` — runes-based reactive stores (`instances`, `settings`, `ui`).
- `lib/components/` — `Sidebar`, `InstanceCard`, `InstanceIcon`, `Modal`, `CreateInstanceModal`, `Icon`.
- `routes/` — Home, `browse/`, `library/`, `instance/[id]/`, `settings/`.

### Backend (`src-tauri/src/`)
- `error.rs` — `AppError` (serializable, returned from commands).
- `paths.rs` — app-data directory helpers (instances, shared `meta/` downloads).
- `instance/` — `Instance`/`ModLoader` model + `InstanceStore` (folder-per-instance, `instance.json`).
- `settings.rs` — `Settings` + `SettingsStore` (persisted to `settings.json`).
- `minecraft/` — Mojang version manifest + per-version detail fetching/caching.
- `launch/` — the launch pipeline:
  - `download.rs` — concurrent downloads with SHA-1 verification.
  - `rules.rs` — OS/arch/feature rule evaluation.
  - `libraries.rs` — classpath resolution + native extraction (old + new schemes).
  - `assets.rs` — asset index + object store (with legacy `virtual` support).
  - `java.rs` — managed Java runtime download (per required component) + path override.
  - `args.rs` — JVM/game argument building, placeholder substitution, offline UUID.
  - `process.rs` — spawn, stream stdout/stderr, kill support, playtime tracking.
- `commands.rs` — Tauri command handlers.

Shared downloads live under `meta/` (`versions/`, `libraries/`, `assets/`, `java/`);
per-instance game files live in `instances/<id>/minecraft/`, natives in `instances/<id>/natives/`.

Launch progress/logs stream to the frontend via `launch-status`, `launch-progress`,
and `launch-log` events (see `src/lib/stores/launch.svelte.ts`).

## Roadmap

- [x] **Foundation** — UI shell, command architecture, instance model, settings, version fetching.
- [x] **Launch pipeline** — download vanilla client/libraries/assets, managed Java, offline launch with live logs.
- [x] **Microsoft auth** — device-code login (MS → Xbox → XSTS → Minecraft), multi-account, offline mode retained. Requires your Azure client ID in `src-tauri/src/auth/mod.rs`.
- [ ] **Mod loaders** — Fabric, Quilt, NeoForge, Forge install logic.
- [ ] **Modrinth integration** — browse & install mods, modpacks, resource packs, shaders.
