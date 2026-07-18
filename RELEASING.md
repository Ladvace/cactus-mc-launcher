# Releasing

Installers are built by `.github/workflows/release.yml` when you push a semver
tag. They land on a **draft** GitHub release you can review and publish.

```bash
# bump the version in src-tauri/tauri.conf.json + package.json first, then:
git tag v0.1.0
git push origin v0.1.0
```

Matrix: macOS (Apple Silicon + Intel), Linux (`ubuntu-22.04`), Windows.

## Feature keys (compile-time)

These are baked into the binary at build time by `build.rs`. Locally they come
from `src-tauri/.env` (gitignored); in CI, `.env` isn't present, so add them as
repo **Settings → Secrets and variables → Actions** secrets. Leave one unset to
ship with that feature disabled.

| Secret | Enables |
| --- | --- |
| `AZURE_CLIENT_ID` | Microsoft sign-in (device-code login) — a public client ID, safe to bake |
| `CURSEFORGE_API_KEY` | The CurseForge content source + FTB modpacks |

> ⚠️ **CurseForge key + public distribution.** A baked `CURSEFORGE_API_KEY` is
> extractable from the shipped binary, which is against CurseForge's API terms
> (keys must stay server-side) and risks revocation. For a **publicly
> distributed** build, do **not** set the `CURSEFORGE_API_KEY` secret — leave
> CurseForge disabled, or proxy CurseForge calls through a server you control
> (e.g. the Cloudflare Worker in `server/`) that holds the key. Baking it is fine
> only for personal/internal builds. `AZURE_CLIENT_ID` is a public client ID and
> is safe to bake.

## macOS signing + notarization

The macOS jobs are Developer-ID signed and notarized when these repo
**Settings → Secrets and variables → Actions** secrets are present. Without
them the build still runs, just unsigned.

| Secret | What it is |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 of your **Developer ID Application** `.p12` export |
| `APPLE_CERTIFICATE_PASSWORD` | Password you set when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | An **app-specific password** (appleid.apple.com → Sign-In & Security) |
| `APPLE_TEAM_ID` | Your 10-char Team ID |

Preparing the certificate:

```bash
# From Keychain Access, export the "Developer ID Application" cert as cert.p12,
# then base64 it for the secret value:
base64 -i cert.p12 | pbcopy   # paste into APPLE_CERTIFICATE
```

`APPLE_SIGNING_IDENTITY` is the full name shown by:

```bash
security find-identity -v -p codesigning
```

> Notarization uploads the build to Apple and can take a few minutes per macOS
> job. Windows/Linux builds are unsigned for now.
