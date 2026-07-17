import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";
import path from "node:path";

// Standalone Vitest config (kept separate from the SvelteKit vite.config so the
// kit runtime doesn't need to boot). SvelteKit's `$app/*` virtual modules are
// aliased to small test stubs.
import { fileURLToPath } from 'node:url';
import { storybookTest } from '@storybook/addon-vitest/vitest-plugin';
const dirname = typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath(import.meta.url));

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig({
  // Cast: Vite 8 (rolldown) and Vitest's bundled Vite disagree on plugin types.
  plugins: [svelte(), svelteTesting()] as never,
  resolve: {
    alias: {
      $lib: path.resolve("src/lib"),
      "$app/environment": path.resolve("src/test/mocks/app-environment.ts"),
      "$app/navigation": path.resolve("src/test/mocks/app-navigation.ts"),
      "$app/stores": path.resolve("src/test/mocks/app-stores.ts")
    }
  },
  test: {
    projects: [{
      extends: true,
      test: {
        name: "unit",
        environment: "jsdom",
        globals: true,
        setupFiles: ["./src/test/setup.ts"],
        include: ["src/**/*.{test,spec}.{ts,js}"]
      }
    }, {
      extends: true,
      plugins: [
      // The plugin will run tests for the stories defined in your Storybook config
      // See options at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon#storybooktest
      storybookTest({
        configDir: path.join(dirname, '.storybook')
      })],
      test: {
        name: 'storybook',
        browser: {
          enabled: true,
          headless: true,
          provider: 'playwright',
          instances: [{
            browser: 'chromium'
          }]
        }
      }
    }]
  }
});