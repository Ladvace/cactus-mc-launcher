# Changelog

All notable changes to **Cactus Launcher** are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project uses
[Semantic Versioning](https://semver.org/).

## [1.11.0] — 2026-08-01

### Added
- **Capes in the skin modal.** Your owned capes now appear as cropped previews so
  you can see each one before selecting it, and the cape section is always shown
  with clear "no capes" and error states instead of silently disappearing.
- **Sign in again.** Each Microsoft account has a one-click re-sign-in that
  re-mints a stale token in place, so you no longer have to delete and re-add an
  account to recover it.
- **Animated themes.** A new **Animated** tab adds gradient and pattern presets
  plus two interactive backgrounds — a cursor-trail reactive grid and a
  Perlin-noise lattice — both of which honour the *Reduce motion* setting.
- **DevTools.** An opt-in build feature and a keyboard shortcut open the webview
  inspector, including in release builds when enabled.

### Changed
- **Consistent settings saving.** Settings now apply on **Save** (UI scale no
  longer takes effect instantly), and a single "unsaved changes" toast signals a
  pending edit and offers to save it in one click.
- **Clearer settings layout.** Options are grouped into more categories.
- **Smarter sign-in refresh.** Automatic Microsoft token renewal now reports the
  precise reason on failure and distinguishes a genuinely expired session from a
  transient or malformed error, so you're only asked to sign in again when it is
  actually needed.
- **Centralized theming and cleanup.** Theme data lives in one place and the
  codebase received a broad naming and de-duplication pass.

### Fixed
- **Back-to-back installs in the dock.** Starting two modpacks one after another
  now shows a placeholder for both immediately, instead of the second appearing
  only once its first progress event arrived.

### Security
- **Hardened untrusted inputs.** Additional guards against SSRF, path traversal,
  and zip-bomb archives, plus tighter loader JVM-argument handling and pack
  override extraction.

## [1.10.0] — 2026-07-27

### Added
- **Service status in Settings.** A new panel shows the live reachability of the
  services Cactus depends on — Modrinth, CurseForge, Minecraft services,
  Microsoft sign-in, Mojang game files, and mod-loader metadata — with a
  red/green dot, latency, and a refresh button, so you can tell at a glance
  whether a failed browse, sign-in, or download is your connection or an
  upstream outage.
- **Low-RAM heap warning in Tune-up.** On memory-constrained machines the
  recommended startup heap is now kept small so the game can still launch, and
  Tune-up warns that raising max memory much higher may prevent it from starting.

### Changed
- **Resumable downloads.** Mod, resource-pack, and shader downloads now stream to
  disk and resume from where they left off after a dropped connection (HTTP
  `Range`) instead of starting over, with checksum verification and automatic
  retries.
- **Matched heap for smoother play.** Tune-up now recommends an equal min/max
  heap (`-Xms` = `-Xmx`) so the JVM never resizes mid-session — removing a common
  source of in-game stutter — while automatically backing off on machines without
  the RAM headroom to commit it up front.
- **Crash hints on launch failures.** When a game fails to launch, Cactus now
  reads the log tail and shows a plain-language hint (out of memory, heap set too
  high, outdated Java, incompatible mods, graphics drivers) instead of only a raw
  error.

### Fixed
- **Crash-safe writes.** Accounts, settings, instance metadata, content lists, and
  server op/whitelist files are now written atomically (write-then-rename), so a
  crash or power loss mid-write can never leave a truncated or corrupt file.
- **Concurrent content operations no longer clobber each other.** Installing,
  toggling, or removing content on the same instance is now serialized, closing
  a race where a file could be recorded as installed but deleted from disk.
- **Expired sessions refresh on launch** instead of failing, falling back to a
  clean re-login prompt when a token can't be renewed.
- **Hardened against malformed remote filenames** — a downloaded file's name is
  now validated so it can only ever be written inside its instance folder.

## [1.9.1] — 2026-07-26

### Fixed
- **CurseForge browse was capped at 20 results.** The CurseForge total-count
  field wasn't parsed (a camelCase mismatch), so it read as 0 and the "Load more"
  button never appeared. Browse now pages through all CurseForge results.

## [1.9.0] — 2026-07-26

### Added
- **Imported mods become fully managed** — after importing from another launcher,
  Cactus matches each loose mod against Modrinth by file hash and upgrades matches
  into tracked content with a real name, icon, and one-click updates (unmatched
  files stay as local content).
- **"Shared" badge** in the Content tab marks mods that are hard-linked across
  instances (stored once on disk), with a hover tooltip explaining it.
- **Open folder** buttons — on the instance header (opens its game folder) and in
  Settings → Storage (opens the app-data folder).
- **Export cleanup** — old exported `.cactuspack`/`.mrpack` files are auto-pruned
  to the most recent 20, plus a "Clear exports" button in Settings → Storage.

### Changed
- The instance detail view is a little wider, and its buttons use real tooltips
  (native ones don't render in the app's webview).

### Fixed
- **"Saved by sharing"** in Settings now reports the real figure instead of 0 —
  it was comparing against the whole cache (including orphaned downloads) rather
  than the deduplicated on-disk size of instance content.

### Security
- Launcher imports read the launcher's icon strictly from within its bundle, and
  the Modrinth profile path is validated against traversal before use.

## [1.8.0] — 2026-07-26

### Added
- **Import from other launchers** — Cactus now detects **Prism Launcher,
  MultiMC, PolyMC, CurseForge, the Modrinth App, and the official Minecraft
  launcher**, lists their instances, and imports any of them — mods, config,
  worlds, resource packs and all — into a new Cactus instance. There's an
  optional "delete from the other launcher after importing", and each launcher
  shows its own logo (read from your installed copy) with a coloured badge as a
  fallback. Copied mods/packs show up in the Content tab as local items.

### Changed
- The home **Import** menu now spells out the file formats it accepts
  (`.mrpack`, `.zip`, …); file import was removed from the Community page, which
  now keeps only share-code import.
- Button hover reworked to a clean fill brighten, replacing the pixel-edge
  highlight that could leave a stray line at non-fullscreen window sizes.
- Fabric/Quilt dedicated-server installers are now verified against their Maven
  SHA-1 before running.

### Fixed
- The **"Read" link** in news cards now sits in the bottom-right corner
  regardless of the headline length.
- The instance right-click **"Export setup"** entry now uses an export (upload)
  icon instead of a download one.
- Detection of the Modrinth App (its data format changed) and imported instances
  keeping their original icon.

### Security
- Launcher imports validate every source path against a fresh on-disk detection,
  refuse symlinked / absolute / path-escaping sources, only delete real launcher
  instances under your home directory, and size-cap icon reads — so a forged
  request can't copy from or delete an arbitrary folder.

## [1.7.0] — 2026-07-23

### Added
- **Content updates & restore points** — update installed mods/content per-item
  or all at once (checked concurrently). Every update auto-creates a restore
  point with one-click Undo from a toast, and a Restore Points panel snapshots
  content + config so you can restore in place or delete points. Restores
  pre-download everything into the cache before touching the instance, so a
  failed rollback can never half-wipe it.
- Searchable **version dropdowns** with a full-viewport blur backdrop and
  keyboard navigation (arrows, Home/End, PageUp/Down), plus an in-dropdown
  **"Snapshots" toggle** with snapshot versions shown indented and dimmed.

### Changed
- The Content tab is split into sub-tabs by type (Mods / Resource Packs / …).
- **"Find mods"** from an instance now targets that instance and pre-filters
  Browse to its type + Minecraft version + loader, so you can't grab an
  incompatible mod. Vanilla instances guide you to resource packs / datapacks
  and block mod installs.
- Generic **"Browse content"** labels covering Modrinth + CurseForge and all
  content types.

### Fixed
- Duplicate instance names are prevented (auto-suffix on every creation path).
- Full translations across all 12 locales for the new update/restore, sub-tab,
  view, and branding strings.

## [1.6.0] — 2026-07-23

### Added
- **Crafting-table easter egg** using real Minecraft textures (streamed, none
  bundled).
- **In-game menu branding** — a Cactus wordmark and splashes on the title screen
  for every instance.
- **Flatpak / Flathub packaging** for Steam Deck and Linux desktops.
- **Folder tiles in the dock**, plus a sleeker, reduced-motion-aware tile
  rearrange on the home screen.
- **Browse grid/list toggle** and advanced browse filters.
- **Cancellable Microsoft sign-in.**
- Website: per-arch macOS links and all Linux formats always point at the latest
  release.

### Fixed
- LWJGL 2 (Minecraft ≤ 1.12.2) now launches on Apple Silicon via Rosetta
  detection.
- Board sessions auto-relogin on expiry.
- Settings dropdown sizing and consistent "Export setup" wording.

### Security
- Modpack import now enforces an HTTPS host allow-list, mandatory file hashes,
  and a component-based path-escape check (closing SSRF / arbitrary-write / RCE
  vectors).
- The Minecraft access token is redacted from launch logs; `accounts.json` and
  `settings.json` are stored owner-only (0600).
- SSRF guard on image downloads; version-JSON and Maven-library integrity
  checks; an HTTP read timeout with all fetches routed through the shared
  client; and a Content-Security-Policy is now enabled.

## [1.5.1] — 2026-07-22

### Added
- Notifications now **stack** — collapsed cards that fan out into a readable
  list on hover, with their auto-dismiss paused while you're reading them.

### Fixed
- **Right-to-left (Arabic)** — toggles and sliders no longer break; their thumb
  and fill render correctly while the surrounding labels flow right-to-left.
- **Interface size** now works. It previously used CSS zoom, which broke
  fixed-position elements like the dock; it now applies through the real webview
  zoom. The slider also gained min / 100% / max tick marks.

## [1.5.0] — 2026-07-22

### Added
- **12 languages** — Cactus now speaks **English, French, Spanish, German,
  Portuguese, Polish, Russian, Turkish, Arabic, Japanese, Korean, and Chinese**.
  Pick your language from Settings → Interface or straight from the onboarding
  screen, through a custom dropdown with pixel-art flags. The whole interface
  switches live — no restart — and **Arabic renders right-to-left**. Dynamic
  content (mod, world, and achievement names, and raw error text) falls back to
  English where a translation isn't available.
- **Accessibility & customization settings** — a new Accessibility section
  (interface zoom, reduce motion, readable font, high contrast, reduce
  transparency, always-show focus outlines), an **accent colour** picker
  (Gold / Emerald / Diamond / Redstone / Lapis / Amethyst), a **sound-volume**
  slider, and a **default mod loader** for new instances.
- **Discover servers** — a new "Discover" tab in Servers to browse a public
  server directory: search, sort by players/votes/rating, live player counts,
  and one-click add.
- **Advanced content filters** — filter Browse by category, environment
  (client/server), and open-source, on top of version and loader.
- **Skin editor** — draw your own skin on an unwrapped 2D canvas (opened from the
  account modal). Pencil, eraser, fill, and eyedropper tools, a colour palette,
  undo/redo, part guides, and a **live front + back preview**. Applies straight
  to your Minecraft account.
- **Reset skin** — a one-click button to revert your skin to the default
  Steve/Alex.

### Changed
- Custom dropdowns everywhere — every menu now uses one consistent, themed,
  keyboard-navigable picker instead of the OS-native one.
- The account modal now shows skeletons while your skin, capes, and accounts
  load, so the layout no longer jumps.

### Fixed
- Uploading or changing a skin now updates the face and preview **instantly**
  everywhere, instead of lagging behind the avatar service's cache; the choice
  also survives closing the modal.
- Changing a skin no longer fails with a 401 when your session has been open a
  while — the Minecraft token is refreshed automatically.
- Buttons no longer show a stray gold line under them on hover.

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

[1.5.1]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.5.0...v1.5.1
[1.5.0]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.3.1...v1.4.0
[1.3.1]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/Ladvace/cactus-mc-launcher/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.2.0
[1.1.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.1.0
[1.0.0]: https://github.com/Ladvace/cactus-mc-launcher/releases/tag/v1.0.0
