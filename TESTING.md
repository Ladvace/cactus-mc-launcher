# Testing

Three layers across both stacks. Everything below is runnable today.

## Frontend (Vitest + Testing Library)

Config: `vitest.config.ts` (jsdom, `$lib`/`$app/*` aliased, Tauri IPC stubbed in
`src/test/setup.ts`). Tests live next to their subject as `*.test.ts`.

```bash
bun run test         # run once
bun run test:watch   # watch mode
```

- **Unit** — pure modules: `src/lib/background.test.ts`, `serverAddress.test.ts`,
  `format.test.ts`, `stores/groupCovers.test.ts`.
- **Integration (component)** — `src/lib/components/Toaster.test.ts` renders a
  component and drives it through the real store.

Add a test by dropping a `<name>.test.ts` beside the module. Mock a Tauri
command per-test with `vi.mock("@tauri-apps/api/core", …)`.

## Rust (cargo test)

In-module `#[cfg(test)]` unit tests (the crate's internals are private, so unit
tests live with the code rather than in `tests/`).

```bash
cd src-tauri && cargo test --lib
```

Covers `offline_uuid`, Forge/NeoForge version matching, the `.cactuspack` path
guard (`safe_rel`), the LWJGL Rosetta threshold, and UUID dashing.

## End-to-end (Playwright)

`playwright.config.ts` builds the SPA and serves it; `e2e/home.spec.ts` boots it
in Chromium with the Tauri IPC mocked (each command returns a sensible default),
then asserts the Home empty state, the create action, and Settings render.

```bash
bunx playwright install chromium   # one-time
bun run e2e                        # build + serve + run
bun run e2e:ui                     # interactive
```

Full desktop e2e (real Rust backend) would use `tauri-driver` + WebDriver; this
suite exercises the frontend, where most UI logic lives. To extend, add specs
under `e2e/` and grow the IPC stub in `mockTauri()` for the commands a flow hits.
