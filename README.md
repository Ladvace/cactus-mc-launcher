<div align="center">

<img src="static/cactus-logo.png" alt="Cactus Launcher" width="120" />

# Cactus Launcher

**A cozy, fast, ad-free Minecraft launcher.**
Arrange your instances, install mods, run servers, and play together — spiky but not spooky.

[![CI](https://github.com/Ladvace/cactus-mc-launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/Ladvace/cactus-mc-launcher/actions/workflows/ci.yml)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-24C8DB)

Built with **Tauri v2** · **SvelteKit** · **Svelte 5 (runes)** · **Rust**

</div>

---

## Screenshots

> Drop images into `docs/screenshots/` with these names and they'll appear here.

|  |  |
| --- | --- |
| ![Home — instances](docs/screenshots/home.png) | ![Instance detail](docs/screenshots/instance.png) |
| **Home** — drag-to-group instances, custom covers | **Instance** — content, worlds, servers, tune-up |
| ![Browse mods](docs/screenshots/browse.png) | ![Adaptive tune-up](docs/screenshots/tuneup.png) |
| **Browse** — Modrinth & CurseForge | **Tune-up** — hardware-aware performance |

## Why Cactus

- 🚫 **No ads, ever.** Free and open-source — no tracking, no upsells, no premium tier.
- 🪶 **Tiny & light.** A **~10 MB** release binary (see [Lightweight](#lightweight)). It uses your OS's native WebView instead of bundling a whole Chromium runtime like Electron apps do.
- 🎨 **Make it yours.** Themeable backgrounds, gradient/decor presets, a customizable dock, and drag-to-group instances with cover images.
- 🔒 **Your keys stay yours.** No secrets are baked into the client; online features are optional and off by default.

## Features

- **Instances** — create per-version, per-loader instances; drag to group; custom icons & covers; playtime tracking.
- **Mod loaders** — Vanilla, **Fabric**, **Quilt**, **Forge**, and **NeoForge** (Forge/NeoForge run the official installer headlessly on first launch).
- **Content** — browse and install mods, resource packs, shaders, and datapacks from **Modrinth** and **CurseForge**; per-instance enable/disable/remove; one-click `.mrpack` / `.cactuspack` install.
- **Adaptive Tune-up** — inspects your RAM/CPU and the instance's loader/version, then recommends a tailored performance mod set (Sodium, Lithium, …), heap size, and JVM flags. Transparent and editable — not a black box. Optional **Visuals** mode adds shaders.
- **Servers** — create dedicated-server instances, an interactive console, `server.properties` editor, worlds, ops/whitelist management, and one-command sharing.
- **Play together** — a presence panel showing who's online, filterable by version/loader.
- **Community boards** — shareable creator/server/streamer pages with published instance snapshots (opt-in, powered by the [backend](#backend)).
- **Accounts** — Microsoft sign-in (device code) with multi-account support, plus offline mode.
- **Managed Java** — the right Java runtime is downloaded automatically per version; Apple Silicon Rosetta handling for old LWJGL.

## Lightweight

Cactus is built on **Tauri**, so the UI runs in the operating system's built-in WebView (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux) and the core logic is native **Rust**. There's no bundled browser engine — the single biggest reason Electron apps are hundreds of megabytes and heavy on memory.

| Metric | Cactus |
| --- | --- |
| Release binary (macOS, arm64, stripped + LTO) | **10.2 MB** |
| Bundled browser runtime | **None** (uses the OS WebView) |

> Measured from `cargo build --release` on Apple Silicon. Memory footprint is dominated by the shared system WebView; contributions of reproducible cross-platform benchmarks are welcome.

## Getting started

Requires [Bun](https://bun.sh) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) (Rust toolchain + platform build deps).

```bash
bun install
bun run tauri dev        # launch the desktop app (dev)
```

Other scripts:

```bash
bun run check            # svelte-check (type-check the frontend)
bun run test             # unit tests (Vitest)
bun run e2e              # end-to-end tests (Playwright)
bun run storybook        # component explorer
bun run tauri build      # produce a release .app / .dmg
```

### Online features (all optional)

| Feature | How to enable |
| --- | --- |
| Microsoft login | Set `AZURE_CLIENT_ID` in `src-tauri/.env` (a personal-accounts Azure app with public client flows; Mojang app approval required). Offline mode needs nothing. |
| Community / CurseForge / presence | Deploy the [backend](#backend) and set `VITE_STREAMER_API_URL` + `CACTUS_API_BASE`. |
| Stickers (Giphy) | Add a Giphy key in Settings. |

Until configured, these stay inert and the launcher runs fully local.

## Backend

The community boards, presence, snapshot sharing, and the CurseForge proxy are served by a small **Cloudflare Worker + Supabase + R2** backend that lives in its own repository. The desktop app only ever receives public URLs — no secret keys are shipped in the client. See that repo's `HOSTING.md` to deploy your own on free tiers.

## Project structure

```
src/                     SvelteKit frontend (Svelte 5 runes)
  lib/api.ts             typed wrapper over the Rust command layer
  lib/types.ts           TS mirrors of the Rust types
  lib/stores/            runes-based reactive stores
  lib/components/        UI components
  routes/                Home, browse, instance/[id], settings, …
src-tauri/src/           Rust core
  instance/ settings.rs  models + persisted stores
  minecraft/ modrinth/   Mojang manifest + Modrinth client
  sources/               content-provider abstraction (Modrinth, CurseForge)
  content/               install content into instances
  loader/                Fabric/Quilt/Forge/NeoForge profile handling
  launch/                the launch pipeline (download, java, args, process)
  tuneup.rs              adaptive performance recommendations
  http.rs                shared HTTP client (identifying User-Agent)
  commands.rs            Tauri command handlers
```

Shared downloads live under `meta/` (versions, libraries, assets, java); per-instance
game files live in `instances/<id>/minecraft/`. Launch progress/logs stream to the
frontend via `launch-status` / `launch-progress` / `launch-log` events.

## Testing

Three layers, all wired into CI:

- **Unit** — Vitest (frontend) + `cargo test` (Rust).
- **E2E** — Playwright against a mocked Tauri IPC.
- **Storybook** — component explorer.

## Contributing

Issues and PRs welcome. Please run `bun run check` and `bun run test` before opening a PR (a Husky pre-commit hook runs them automatically).

## License

Licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-only).
