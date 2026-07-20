import { settingsStore } from "$lib/stores/settings.svelte";

/// Format a date per the user's `dateFormat` setting. Reads the setting so it's
/// reactive when called from a component's template/derived. Returns "" for
/// missing/invalid input.
export function formatDate(iso: string | null | undefined, format?: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "";
  switch (format ?? settingsStore.settings.dateFormat) {
    case "iso": {
      const y = d.getFullYear();
      const m = String(d.getMonth() + 1).padStart(2, "0");
      const day = String(d.getDate()).padStart(2, "0");
      return `${y}-${m}-${day}`;
    }
    case "us":
      return d.toLocaleDateString("en-US", { year: "numeric", month: "2-digit", day: "2-digit" });
    case "eu":
      return d.toLocaleDateString("en-GB", { year: "numeric", month: "2-digit", day: "2-digit" });
    default:
      return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
  }
}

export function timeAgo(iso: string | null | undefined, whenNull = "never"): string {
  if (!iso) return whenNull;
  const seconds = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
  if (seconds < 90) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}
