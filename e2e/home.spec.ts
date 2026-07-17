import { test, expect, type Page } from "@playwright/test";

// Stub the Tauri IPC so the SPA boots in a plain browser. Every command the
// startup path calls returns a sensible default; unknown commands return null.
async function mockTauri(page: Page) {
  await page.addInitScript(() => {
    const settings = {
      theme: "dark",
      javaPath: null,
      maxMemoryMb: 4096,
      minMemoryMb: 1024,
      jvmArgs: "",
      gameWidth: 854,
      gameHeight: 480,
      offlineUsername: "Player",
      background: "",
      uiSounds: true,
      giphyApiKey: "",
      dockPosition: "bottom",
      decorTheme: "",
      dockMagnify: true,
    };
    const responses: Record<string, unknown> = {
      list_instances: [],
      get_settings: settings,
      get_accounts: { accounts: [], activeId: null, microsoftConfigured: false },
      content_cache_stats: { files: 0, bytes: 0, linkedBytes: 0, savedBytes: 0 },
      is_instance_running: false,
    };
    // @ts-expect-error test-only global
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string) =>
        Promise.resolve(cmd in responses ? responses[cmd] : null),
      transformCallback: (cb: unknown) => cb,
    };
  });
}

test.beforeEach(async ({ page }) => {
  await mockTauri(page);
});

test("boots to the Home empty state", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Welcome back" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "No instances yet" })).toBeVisible();
});

test("shows the create-instance action", async ({ page }) => {
  await page.goto("/");
  // Present both in the hero and the dock.
  await expect(page.getByRole("button", { name: /New instance/i }).first()).toBeVisible();
});

test("navigates to Settings and shows theme presets", async ({ page }) => {
  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByText("Theme presets")).toBeVisible();
});
