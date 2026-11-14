//! `ChangelogModal.svelte` and `src/lib/changelog.ts`: CHANGELOG.md, parsed as Keep a Changelog.

use std::sync::LazyLock;

use rune::tok::TextStyle;
use rune::{Cx, El, Span, col, rich, row, text};

use crate::app::App;
use crate::core::i18n;
use crate::theme::Theme;
use crate::ui;

const RAW: &str = include_str!("../../../CHANGELOG.md");

struct Group {
    title: String,
    items: Vec<String>,
}

struct Release {
    version: String,
    date: String,
    groups: Vec<Group>,
}

static CHANGELOG: LazyLock<Vec<Release>> = LazyLock::new(|| parse(RAW));

/// "## [version] date", "### Group", "- item" and indented continuation lines.
fn parse(md: &str) -> Vec<Release> {
    let mut out: Vec<Release> = Vec::new();
    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("## [") {
            if let Some((version, date)) = rest.split_once(']') {
                let date = date.trim_start().trim_start_matches(['—', '-']).trim().to_string();
                out.push(Release { version: version.to_string(), date, groups: Vec::new() });
            }
            continue;
        }
        let Some(release) = out.last_mut() else { continue };
        if let Some(title) = line.strip_prefix("### ") {
            release.groups.push(Group { title: title.trim().to_string(), items: Vec::new() });
            continue;
        }
        if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("-\t")) {
            if release.groups.is_empty() {
                release.groups.push(Group { title: String::new(), items: Vec::new() });
            }
            release.groups.last_mut().unwrap().items.push(item.trim().to_string());
            continue;
        }
        let continues = line.starts_with(char::is_whitespace) && !line.trim().is_empty() && !line.trim_start().starts_with('[');
        if let Some(last) = release.groups.last_mut().and_then(|group| group.items.last_mut()).filter(|_| continues) {
            last.push(' ');
            last.push_str(line.trim());
        }
    }
    out
}

/// `renderInline`: `**bold**` and `` `code` `` as rich-text spans.
fn inline(theme: &Theme, src: &str, size: f32) -> El {
    let mut plain = String::new();
    let mut spans = Vec::new();
    let mut rest = src;
    while !rest.is_empty() {
        let bold = rest.find("**").filter(|&i| rest[i + 2..].contains("**"));
        let code = rest.find('`').filter(|&i| rest[i + 1..].contains('`'));
        let next = match (bold, code) {
            (Some(bold_at), Some(code_at)) => Some(if bold_at <= code_at { (bold_at, true) } else { (code_at, false) }),
            (Some(bold_at), None) => Some((bold_at, true)),
            (None, Some(code_at)) => Some((code_at, false)),
            (None, None) => None,
        };
        let Some((at, is_bold)) = next else {
            plain.push_str(rest);
            break;
        };
        plain.push_str(&rest[..at]);
        let (open, close) = if is_bold { ("**", "**") } else { ("`", "`") };
        let body = &rest[at + open.len()..];
        let end = body.find(close).unwrap_or(body.len());
        let inner = &body[..end];
        let start = plain.len();
        // A code span inside bold keeps its backticks out of the text.
        if is_bold {
            plain.push_str(&inner.replace('`', ""));
            spans.push(Span::new(start, plain.len()).weight(600.0).color(theme.text));
        } else {
            plain.push_str(inner);
            spans.push(Span::new(start, plain.len()).mono().size(size * 0.82).bg(theme.text.alpha(0.1), 3.0));
        }
        rest = &body[(end + close.len()).min(body.len())..];
    }
    rich(plain, spans).font(TextStyle { line_height: size * 1.5, ..theme.sans(size, 400.0) }).color(theme.text_secondary).wrap()
}

fn release(theme: &Theme, entry: &Release) -> El {
    let mut head = row()
        .key(format!("head-{}", entry.version))
        .center_y()
        .gap(9.6)
        .pb(5.6)
        .border_sides([0.0, 0.0, 1.0, 0.0], theme.border)
        .child(text(entry.version.clone()).font(theme.pixel(18.4)).color(theme.accent));
    if !entry.date.is_empty() {
        head = head.child(text(entry.date.clone()).font(theme.sans(12.5, 400.0)).color(theme.text_muted));
    }
    let mut section = col().key(format!("rel-{}", entry.version)).gap(6.4).child(head);
    for (gi, group) in entry.groups.iter().enumerate() {
        if !group.title.is_empty() {
            let color = match group.title.to_lowercase().as_str() {
                "added" => theme.success,
                "fixed" => theme.warning,
                _ => theme.text_secondary,
            };
            section = section.child(
                text(group.title.to_uppercase())
                    .font(TextStyle { tracking: 11.5 * 0.05, ..theme.pixel(11.5) })
                    .color(color)
                    .mt(if gi == 0 { 1.6 } else { 8.0 })
                    .key(format!("grp-{}-{gi}", entry.version)),
            );
        }
        let mut list = col().gap(4.8).mt(3.2);
        for item in &group.items {
            list = list.child(
                row()
                    .items_start()
                    .child(text("•").font(theme.sans(14.4, 400.0)).color(theme.text_secondary).w(17.6).pl(4.0).h(21.6).center_y())
                    .child(col().grow(1.0).shrink().child(inline(theme, item, 14.4))),
            );
        }
        section = section.child(list);
    }
    section
}

pub fn view(app: &App, _cx: &mut Cx<App>, layer: &mut Vec<El>) {
    if !app.ui.changelog_open {
        return;
    }
    let theme = app.theme();
    let log = col().key("changelog-log").gap(22.4).children(CHANGELOG.iter().map(|entry| release(&theme, entry)));
    layer.push(ui::modal(&theme, "changelog", i18n::t("changelog.title"), 560.0, log, None, |app: &mut App| app.ui.changelog_open = false));
}
