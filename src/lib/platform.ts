const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

export const MOD_KEY = isMac ? "⌘" : "Ctrl";

export const CMD_K = isMac ? "⌘K" : "Ctrl K";
