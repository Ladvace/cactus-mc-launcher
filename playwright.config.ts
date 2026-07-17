import { defineConfig, devices } from "@playwright/test";

// E2E runs the built SPA in a real browser with the Tauri IPC mocked (see
// e2e/home.spec.ts). Full desktop e2e would use tauri-driver; this covers the
// frontend behaviour, which is where most UI logic lives.
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: "list",
  use: {
    baseURL: "http://localhost:4173",
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "bun run build && bun run preview --port 4173 --strictPort",
    port: 4173,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
