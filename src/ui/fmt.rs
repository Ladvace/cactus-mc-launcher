//! `src/lib/format.ts` and `src/lib/time.ts`.

use chrono::{DateTime, Local, Utc};

pub fn count(n: u64) -> String {
    match n {
        0..1000 => n.to_string(),
        1000..10_000 => format!("{:.1}K", n as f64 / 1000.0),
        10_000..1_000_000 => format!("{:.0}K", n as f64 / 1000.0),
        _ => format!("{:.1}M", n as f64 / 1_000_000.0),
    }
}

pub fn bytes(size: u64) -> String {
    if size < 1024 {
        return format!("{size} B");
    }
    let units = ["KB", "MB", "GB", "TB"];
    let mut value = size as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 { format!("{value:.1} {}", units[unit]) } else { format!("{value:.0} {}", units[unit]) }
}

/// `formatDate` with the `dateFormat` setting: system | iso | us | eu.
pub fn date(at: DateTime<Utc>, format: &str) -> String {
    let local = at.with_timezone(&Local);
    match format {
        "iso" => local.format("%Y-%m-%d").to_string(),
        "us" => local.format("%m/%d/%Y").to_string(),
        "eu" => local.format("%d/%m/%Y").to_string(),
        _ => local.format("%b %-d, %Y").to_string(),
    }
}

/// `formatDate` for an ISO string; "" when it doesn't parse.
pub fn date_str(iso: &str, format: &str) -> String {
    DateTime::parse_from_rfc3339(iso).map(|parsed| date(parsed.with_timezone(&Utc), format)).unwrap_or_default()
}

/// `timeAgo`: "just now", "5m ago", "3h ago", "2d ago"; `when_none` for no date.
pub fn time_ago(at: Option<DateTime<Utc>>, when_none: &str) -> String {
    let Some(at) = at else { return when_none.to_string() };
    let secs = (Utc::now() - at).num_seconds().max(0) as f64;
    if secs < 90.0 {
        return "just now".into();
    }
    let minutes = (secs / 60.0).round();
    if minutes < 60.0 {
        return format!("{minutes}m ago");
    }
    let hours = (minutes / 60.0).round();
    if hours < 24.0 {
        return format!("{hours}h ago");
    }
    format!("{}d ago", (hours / 24.0).round())
}

/// Playtime as the instance cards show it: "12h 5m", "45m", "never played" for 0.
pub fn playtime(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    match (hours, minutes) {
        (0, 0) if secs == 0 => String::new(),
        (0, 0) => "<1m".into(),
        (0, minutes) => format!("{minutes}m"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h {minutes}m"),
    }
}
