# Changelog

All notable changes to **Cactus Launcher** are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project uses
[Semantic Versioning](https://semver.org/).

## [1.3.0] — Unreleased

### Added
- **Achievements & stats dashboard** — a Steam-style page that aggregates your
  advancements and lifetime stats across *every* instance and world, not per-save
  like vanilla. Shows overall completion, per-category progress, locked/hidden
  advancements, launcher-exclusive "Cactus Challenges" (e.g. Pacifist, Marathoner),
  lifetime totals, top mined blocks / mobs, and a per-instance breakdown. Reads
  only local vanilla files — no mods required.
- **Command palette** — press `⌘K` / `Ctrl+K` to fuzzy-search your instances,
  jump between pages, and run actions (create instance, accounts, check for updates).
- **CurseForge modpack install** — install CurseForge modpacks (not just Modrinth),
  with mods resolved through the backend proxy and configs applied.
- **Server list sync** — add a server from the Servers page straight into an
  instance's in-game multiplayer list (`servers.dat`).

## [1.2.0] — 2026-07-19

### Added
- **Configurable concurrent downloads** — a slider in Settings (with a
  recommendation) controls how many files download at once, across the launch
  pipeline, content, and modpacks.
- **Check for updates** button in Settings.

### Fixed
- Installing a modpack now lists its mods in the instance's **Content tab**
  (they can be toggled, updated, and removed).

## [1.1.0] — 2026-07-19

### Added
- **Servers** page — an editable quick-connect list with live player counts and
  each server's own icon; copy an address in a click.
- **Cactus theme** and a refreshed app icon.
- Skeleton loading states; distinct icons for Browse and Servers.

### Fixed
- A modpack's download progress no longer leaks into the next project you open.

## [1.0.0] — 2026-07-19

First public release.

### Added
- **In-app auto-updates** — signed updates with an "Install & restart" prompt.

### Changed
- Relicensed under **AGPL-3.0**.
- The community backend moved to its own repository; the desktop app only ever
  receives public URLs (no secret keys shipped in the client).

## [0.2.0] and earlier

Early builds establishing the foundation:

- Instance management (per-version, per-loader) with groups, custom icons and covers.
- Mod loaders: Vanilla, Fabric, Quilt, Forge, NeoForge (managed Java per version).
- Content from **Modrinth** and **CurseForge**; one-click Modrinth modpack install.
- Dedicated servers (console, worlds, ops/whitelist, sharing).
- Community boards, presence / play-together, instance sharing.
- Microsoft sign-in with skins & capes; offline mode.
- Adaptive **Tune-up** (hardware-aware performance recommendations), themes,
  a customizable dock, and click sounds.

[1.3.0]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.2.0
[1.1.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.1.0
[1.0.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.0.0
