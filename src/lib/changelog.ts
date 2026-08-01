import raw from "../../CHANGELOG.md?raw";

export interface ChangeGroup {
  title: string;
  items: string[];
}
export interface Release {
  version: string;
  date: string;
  groups: ChangeGroup[];
}

// Inline markdown → HTML (applied after escaping).
const MD_BOLD_RE = /\*\*(.+?)\*\*/g;
const MD_INLINE_CODE_RE = /`([^`]+)`/g;
// Keep-a-Changelog structure: "## [version] date", "### Group", "- item".
const RELEASE_HEADING_RE = /^##\s+\[([^\]]+)\]\s*(.*)$/;
const GROUP_HEADING_RE = /^###\s+(.+)$/;
const LIST_ITEM_RE = /^-\s+(.+)$/;
const LEADING_DATE_DASH_RE = /^[—-]\s*/;
// An indented, non-empty line that continues the previous list item.
const INDENTED_CONTINUATION_RE = /^\s+\S/;

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function renderInline(text: string): string {
  return escapeHtml(text)
    .replace(MD_BOLD_RE, "<strong>$1</strong>")
    .replace(MD_INLINE_CODE_RE, "<code>$1</code>");
}

function parse(markdown: string): Release[] {
  const releases: Release[] = [];
  let release: Release | null = null;
  let group: ChangeGroup | null = null;

  for (const line of markdown.split("\n")) {
    const releaseMatch = line.match(RELEASE_HEADING_RE);
    if (releaseMatch) {
      const date = releaseMatch[2].replace(LEADING_DATE_DASH_RE, "").trim();
      release = { version: releaseMatch[1], date, groups: [] };
      releases.push(release);
      group = null;
      continue;
    }
    if (!release) continue;

    const groupMatch = line.match(GROUP_HEADING_RE);
    if (groupMatch) {
      group = { title: groupMatch[1].trim(), items: [] };
      release.groups.push(group);
      continue;
    }

    const itemMatch = line.match(LIST_ITEM_RE);
    if (itemMatch) {
      if (!group) {
        group = { title: "", items: [] };
        release.groups.push(group);
      }
      group.items.push(itemMatch[1].trim());
      continue;
    }

    if (group?.items.length && INDENTED_CONTINUATION_RE.test(line) && !line.trimStart().startsWith("[")) {
      group.items[group.items.length - 1] += " " + line.trim();
    }
  }

  return releases;
}

export const CHANGELOG: Release[] = parse(raw);
