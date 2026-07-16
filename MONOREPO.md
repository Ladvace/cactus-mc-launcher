# Monorepo layout

This repo holds the launcher and its backend service.

```
/                 Drake Launcher — Tauri v2 + SvelteKit + Rust (the desktop app)
  src/            launcher frontend (SvelteKit)
  src-tauri/      launcher backend (Rust)
server/           Drake Streamer API — Cloudflare Worker + Supabase + R2
```

The two are independent projects with their own toolchains — the launcher
builds with `bun` + `cargo tauri`, the server with `wrangler`. They are kept in
one repo so the shared feature (streamer profiles / snapshot sharing) evolves
together.

## How they connect

The launcher's **Streamers** tab works offline today via `.drakepack` /
`.mrpack` file export/import. The `server/` service adds the hosted layer:
searchable streamer profiles, short share codes, live status, and snapshot
hosting. When it's live, the launcher will call `server`'s `/v1/*` API; a
snapshot is the same file format the launcher already produces, stored in R2.

See [`server/README.md`](server/README.md) for the backend, and the launcher
README for the desktop app.

> Note: the launcher intentionally stays at the repo root (rather than moving to
> `apps/launcher`) to avoid churning the Tauri build's many path assumptions. A
> full `apps/*` restructure can happen later if it earns its keep.
