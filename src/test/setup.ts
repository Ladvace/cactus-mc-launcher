import "@testing-library/jest-dom/vitest";

// The Tauri IPC isn't present in jsdom; stub invoke so importing $lib/api is safe
// and tests can override per-call with vi.mock.
import { vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => undefined),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(async () => {}),
  revealItemInDir: vi.fn(async () => {}),
}));
