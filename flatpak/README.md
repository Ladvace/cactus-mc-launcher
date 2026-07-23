# Flatpak / Flathub packaging

This directory holds everything needed to ship Cactus Launcher on
[Flathub](https://flathub.org/) — the store the **Steam Deck** (and most Linux
desktops) install desktop apps from.

- `com.ladvace.cactuslauncher.yml` — the Flatpak manifest (builds from the
  Tauri-produced `.deb`).
- `com.ladvace.cactuslauncher.metainfo.xml` — the AppStream metadata Flathub
  requires (name, description, screenshots, releases).

The built-in Tauri self-updater is automatically disabled inside Flatpak (the
app detects `FLATPAK_ID` and defers to the store for updates), so there's
nothing to strip from the build.

## Build & test locally

```sh
# One-time: install the tooling and the runtime
sudo apt install flatpak flatpak-builder      # (or pacman -S / dnf install)
flatpak install flathub org.gnome.Platform//46 org.gnome.Sdk//46

# From this directory, after building the amd64 .deb (bun tauri build):
#   1. Either edit the manifest's remote source to a local one:
#        - type: file
#          path: ../src-tauri/target/release/bundle/deb/<name>_amd64.deb
#      …or fill in the real release URL + sha256 (see below).
flatpak-builder --force-clean --user --install --repo repo build-dir \
  com.ladvace.cactuslauncher.yml

# Run it
flatpak run com.ladvace.cactuslauncher
```

Validate the metadata before submitting:

```sh
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest com.ladvace.cactuslauncher.yml
flatpak run --command=appstreamcli org.flatpak.Builder validate com.ladvace.cactuslauncher.metainfo.xml
```

## Before submitting to Flathub — checklist

1. **Fill in the release source** in the manifest: set the `url` to the amd64
   `.deb` of the release you're packaging and its `sha256`
   (`sha256sum "Cactus Launcher_<ver>_amd64.deb"`).
2. **Real screenshots**: replace the placeholder `<screenshot>` URLs in the
   metainfo with reachable image URLs (Flathub downloads and validates them).
3. **Keep `<releases>` current** — add an entry per tagged release.
4. **Confirm the binary name**: the manifest copies whatever single file is in
   the `.deb`'s `usr/bin`, so it's robust, but double-check `flatpak run` starts
   the app.

## Submit to Flathub

1. Fork <https://github.com/flathub/flathub> and create a new branch named
   `com.ladvace.cactuslauncher`.
2. Add `com.ladvace.cactuslauncher.yml` and
   `com.ladvace.cactuslauncher.metainfo.xml` to the repo root of your branch.
3. Open a PR against `flathub/flathub`'s `new-pr` branch and follow the review
   bot's feedback. Once merged, updates are published by bumping the manifest's
   release source in the app's dedicated Flathub repo.

### Optional: automatic update checks

Flathub can auto-open PRs when you tag a new release if you add an
`x-checker-data` block to the `.deb` source, e.g.:

```yaml
    x-checker-data:
      type: anitya
      # or a "type: json"/"type: html" checker pointed at the GitHub releases API
```

See <https://github.com/flathub/flatpak-external-data-checker> for details.
