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

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function renderInline(s: string): string {
  return escapeHtml(s)
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/`([^`]+)`/g, "<code>$1</code>");
}

function parse(md: string): Release[] {
  const releases: Release[] = [];
  let rel: Release | null = null;
  let group: ChangeGroup | null = null;

  for (const line of md.split("\n")) {
    const h2 = line.match(/^##\s+\[([^\]]+)\]\s*(.*)$/);
    if (h2) {
      rel = { version: h2[1], date: h2[2].replace(/^[—-]\s*/, "").trim(), groups: [] };
      releases.push(rel);
      group = null;
      continue;
    }
    if (!rel) continue;

    const h3 = line.match(/^###\s+(.+)$/);
    if (h3) {
      group = { title: h3[1].trim(), items: [] };
      rel.groups.push(group);
      continue;
    }

    const item = line.match(/^-\s+(.+)$/);
    if (item) {
      if (!group) {
        group = { title: "", items: [] };
        rel.groups.push(group);
      }
      group.items.push(item[1].trim());
      continue;
    }

    if (group?.items.length && /^\s+\S/.test(line) && !line.trimStart().startsWith("[")) {
      group.items[group.items.length - 1] += " " + line.trim();
    }
  }

  return releases;
}

export const CHANGELOG: Release[] = parse(raw);
