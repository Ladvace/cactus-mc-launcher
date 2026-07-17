/** Compact relative-time label: "just now", "5m ago", "3h ago", "2d ago".
 *  Returns `whenNull` for a null/empty timestamp. */
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
