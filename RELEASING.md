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
| `CACTUS_API_BASE` | Backend URL (the deployed Worker) — enables the CurseForge proxy |

Both baked values are public (a client ID and a URL) — safe to ship in the
binary. **The CurseForge API key is never baked into the client:** it lives on
the backend Worker (`server/`) as a secret, and the client reaches CurseForge
through the `/v1/curseforge` proxy. Set it on the Worker:

```bash
cd server
wrangler secret put CURSEFORGE_API_KEY   # get a free key at console.curseforge.com
# local dev: add CURSEFORGE_API_KEY=... to server/.dev.vars
```

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
