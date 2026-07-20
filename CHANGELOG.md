# Changelog

All notable changes to **Cactus Launcher** are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project uses
[Semantic Versioning](https://semver.org/).

## [1.4.0] — 2026-07-20

### Added
- **Friends** — a Minecraft friends list in Play Together: view your friends
  (with a live "online in Cactus" dot), add by username, and accept, decline,
  or remove requests. Includes a toggle to turn the friends feature and invite
  acceptance on/off. Uses Mojang's friends API directly from the launcher.
- **Date format** setting (Settings → Interface) — System, ISO, US, or EU,
  applied across the app.
- A **beta** badge on the Community tab.

### Changed
- Custom pixel-styled range sliders, with per-GB tick marks on the memory
  sliders.
- Default concurrent downloads lowered to 8 (from 16), with a gentler
  recommendation — high counts give diminishing returns and risk rate-limits.

### Fixed
- **Play Together / Community sign-in now works in released builds.** The old
  sign-in verified your account through Mojang's session server *from the
  backend*, which Mojang's CDN blocks from Cloudflare's IPs. Sign-in now uses a
  Mojang-signed player certificate the backend verifies **offline** — no server
  call to Mojang — so it works everywhere. Your Minecraft token still never
  leaves the launcher.

## [1.3.1] — 2026-07-19

### Fixed
- **Play Together / Community now works in installed builds** — the boards
  backend URL wasn't baked into release builds; it's now derived from the single
  backend URL config, so the feature is available out of the box.
- **"Online now" no longer flickers** in Play Together, and board sign-in shows
  the real error (with a Retry) instead of an endless "Connecting…".
- In offline mode, Play Together now guides you to switch to your Microsoft
  account instead of wrongly saying "add a Microsoft account".
- The Servers "add to instance" chooser opens as a floating menu instead of
  stretching the whole card row.
- The image picker's Cactus tab scrolls vertically instead of overflowing
  sideways.

### Changed
- Faster networking — one shared, pooled HTTP client with a connect timeout, so
  a cold/unreachable backend fails fast instead of hanging.

## [1.3.0] — 2026-07-19

### Added
- **News on Home** — a "Latest news" section on the Home screen pulling the
  official Minecraft news feed (images + read-more links), cached for ~30 min.
  Browse it as an endless carousel (lead + two-up, or one story per page) and
  hide it any time (inline or in Settings). Built on a source-agnostic model so
  more feeds can be added later.
- **Achievements & stats dashboard** — a Steam-style page (opened from the
  accounts modal) that aggregates your advancements and lifetime stats across
  *every* instance and world, not per-save like vanilla. Shows overall
  completion, per-category progress, locked/hidden advancements,
  launcher-exclusive "Cactus Challenges" (e.g. Pacifist, Marathoner), lifetime
  totals, top mined blocks / mobs, and a per-instance breakdown. Reads only
  local vanilla files — no mods required.
- **Command palette** — press `⌘K` / `Ctrl+K` (or the Home search bar) to
  fuzzy-search your instances, jump between pages, and run actions (create
  instance, accounts, check for updates).
- **What's new** — an in-app changelog modal (Settings → About, or the command
  palette) so you can see what changed without leaving the launcher.
- **CurseForge modpack install** — install CurseForge modpacks (not just
  Modrinth), with mods resolved through the backend proxy and configs applied.
- **Server list sync** — add a server from the Servers page straight into an
  instance's in-game multiplayer list (`servers.dat`).

### Changed
- Custom pixel-styled checkboxes across the app, matching the theme.

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

[1.4.0]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.3.1...v1.4.0
[1.3.1]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.2.0
[1.1.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.1.0
[1.0.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.0.0
