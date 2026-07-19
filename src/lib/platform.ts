// Platform-aware labels for keyboard-shortcut hints shown in the UI.

const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

/** Primary modifier symbol: "⌘" on macOS, "Ctrl" elsewhere. */
export const MOD_KEY = isMac ? "⌘" : "Ctrl";

/** The command-palette shortcut label (e.g. "⌘K" / "Ctrl K"). */
export const CMD_K = isMac ? "⌘K" : "Ctrl K";
