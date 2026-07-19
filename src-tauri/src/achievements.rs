//! Cross-instance achievements & lifetime stats.
//!
//! Minecraft records advancements and stats *per world*, resetting each save.
//! This module aggregates them *per player, across every local instance/world*
//! so users get a lifetime view the base game never offers.
//!
//! Everything here is read from local vanilla JSON — no mods, no server access,
//! so it works uniformly across any instance type. Only singleplayer and
//! player-hosted LAN worlds are covered; progress on remote servers lives
//! server-side and is unreachable.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::auth::AccountStore;
use crate::error::Result;
use crate::instance::store::InstanceStore;
use crate::launch::args::offline_uuid;
use crate::paths;
use crate::settings::SettingsStore;

/// Bundled advancement metadata (real titles + hidden flags), grouped by
/// category. Lets us render locked/greyed tiles and a correct denominator
/// without parsing game assets at runtime.
static ADV_META_JSON: &str = include_str!("data/advancements.json");
/// Stat-derived launcher-only achievements, defined purely as data (stat key +
/// operator + threshold) so new ones ship without touching the engine.
static CUSTOM_JSON: &str = include_str!("data/custom_achievements.json");

const ORDER: [&str; 5] = ["story", "nether", "end", "adventure", "husbandry"];

// ---------------------------------------------------------------------------
// Output payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementsPayload {
    pub player: PlayerInfo,
    /// True once at least one advancement/stats file was found.
    pub has_data: bool,
    pub completion: Completion,
    pub categories: Vec<CategoryProgress>,
    pub advancements: Vec<AdvancementView>,
    pub custom: Vec<CustomAchievement>,
    pub stats: LifetimeStats,
    pub instances: Vec<InstanceBreakdown>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInfo {
    pub name: String,
    /// How many distinct save folders contributed data.
    pub worlds_scanned: usize,
    pub instances_scanned: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    pub earned: usize,
    pub total: usize,
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryProgress {
    pub key: String,
    pub earned: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancementView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub done: bool,
    pub hidden: bool,
    pub earned_at: Option<String>,
    pub earned_in: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAchievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub earned: bool,
    /// 0–100 progress toward the primary (first) condition.
    pub progress: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifetimeStats {
    pub play_time_ticks: i64,
    pub blocks_mined: i64,
    pub mobs_killed: i64,
    pub deaths: i64,
    pub distance_cm: i64,
    pub jumps: i64,
    pub items_picked_up: i64,
    pub damage_dealt: i64,
    pub times_slept: i64,
    pub top_mined: Vec<StatEntry>,
    pub top_killed: Vec<StatEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatEntry {
    pub key: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceBreakdown {
    pub id: String,
    pub name: String,
    pub worlds: usize,
    pub play_time_ticks: i64,
    pub blocks_mined: i64,
    pub mobs_killed: i64,
    pub deaths: i64,
}

// ---------------------------------------------------------------------------
// Bundled metadata parsing
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct AdvMeta {
    id: String,
    name: String,
    #[serde(default)]
    hidden: bool,
}

#[derive(serde::Deserialize)]
struct CustomRule {
    id: String,
    name: String,
    description: String,
    icon: String,
    conditions: Vec<Condition>,
}

#[derive(serde::Deserialize)]
struct Condition {
    stat: String,
    op: String,
    value: i64,
}

// ---------------------------------------------------------------------------
// Aggregation state
// ---------------------------------------------------------------------------

/// Best-known completion of one advancement across all worlds.
struct AdvAgg {
    at: Option<DateTime<FixedOffset>>,
    location: String,
}

#[derive(Default)]
struct Aggregate {
    /// advancement id (without `minecraft:`) -> earliest completion.
    done: HashMap<String, AdvAgg>,
    /// stat category (e.g. `minecraft:mined`) -> subkey -> summed count.
    stats: HashMap<String, HashMap<String, i64>>,
}

impl Aggregate {
    fn stat(&self, category: &str, key: &str) -> i64 {
        self.stats
            .get(category)
            .and_then(|m| m.get(key))
            .copied()
            .unwrap_or(0)
    }

    fn sum_category(&self, category: &str) -> i64 {
        self.stats
            .get(category)
            .map(|m| m.values().sum())
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn compute(app: &AppHandle) -> Result<AchievementsPayload> {
    let (player_name, uuids) = player_identity(app);

    let mut agg = Aggregate::default();
    let mut instances_out: Vec<InstanceBreakdown> = Vec::new();
    let mut worlds_scanned = 0usize;
    let mut instances_scanned = 0usize;

    for instance in app.state::<InstanceStore>().list() {
        // Resolve the game dir without creating it — this scan is read-only, so
        // it must not spawn empty `minecraft/` folders for unlaunched instances.
        let game = match instance.game_dir.as_deref().map(str::trim) {
            Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
            _ => match paths::default_game_dir(app, &instance.id) {
                Ok(dir) => dir,
                Err(_) => continue,
            },
        };
        let saves = game.join("saves");
        if !saves.is_dir() {
            continue;
        }

        let mut per_instance = InstanceBreakdown {
            id: instance.id.clone(),
            name: instance.name.clone(),
            worlds: 0,
            play_time_ticks: 0,
            blocks_mined: 0,
            mobs_killed: 0,
            deaths: 0,
        };
        let mut instance_had_data = false;

        let entries = match std::fs::read_dir(&saves) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let world_dir = entry.path();
            if !world_dir.is_dir() {
                continue;
            }
            let world_name = entry.file_name().to_string_lossy().to_string();
            let location = format!("{} · {}", instance.name, world_name);
            let mut world_had_data = false;

            if let Some(file) = locate_player_file(&world_dir, "advancements", &uuids) {
                if merge_advancements(&file, &location, &mut agg) {
                    world_had_data = true;
                }
            }
            if let Some(file) = locate_player_file(&world_dir, "stats", &uuids) {
                if let Some(delta) = merge_stats(&file, &mut agg) {
                    per_instance.play_time_ticks += delta.play_time;
                    per_instance.blocks_mined += delta.mined;
                    per_instance.mobs_killed += delta.killed;
                    per_instance.deaths += delta.deaths;
                    world_had_data = true;
                }
            }

            if world_had_data {
                per_instance.worlds += 1;
                worlds_scanned += 1;
                instance_had_data = true;
            }
        }

        if instance_had_data {
            instances_scanned += 1;
            instances_out.push(per_instance);
        }
    }

    instances_out.sort_by(|a, b| b.play_time_ticks.cmp(&a.play_time_ticks));

    let advancements = build_advancements(&agg);
    let categories = build_categories(&advancements);
    let earned = advancements.iter().filter(|a| a.done).count();
    let total = advancements.len();
    let completion = Completion {
        earned,
        total,
        percent: pct(earned as i64, total as i64),
    };
    let custom = build_custom(&agg);
    let stats = build_stats(&agg);

    Ok(AchievementsPayload {
        player: PlayerInfo {
            name: player_name,
            worlds_scanned,
            instances_scanned,
        },
        has_data: worlds_scanned > 0,
        completion,
        categories,
        advancements,
        custom,
        stats,
        instances: instances_out,
    })
}

// ---------------------------------------------------------------------------
// Player identity
// ---------------------------------------------------------------------------

/// Returns the display name plus the set of UUIDs (dashless, lowercase) whose
/// save files belong to this player: the active Microsoft account and the
/// offline UUIDs derived from known usernames.
fn player_identity(app: &AppHandle) -> (String, HashSet<String>) {
    let accounts = app.state::<AccountStore>();
    let settings = app.state::<SettingsStore>().get();

    let mut uuids = HashSet::new();
    let name;

    if let Some(acc) = accounts.active_account() {
        uuids.insert(normalize_uuid(&acc.uuid));
        uuids.insert(normalize_uuid(&offline_uuid(&acc.username)));
        name = acc.username;
    } else {
        let offline = if settings.offline_username.trim().is_empty() {
            "Player".to_string()
        } else {
            settings.offline_username.trim().to_string()
        };
        uuids.insert(normalize_uuid(&offline_uuid(&offline)));
        name = offline;
    }

    // Also match offline saves made under any other stored account's name.
    for info in accounts.state().accounts {
        uuids.insert(normalize_uuid(&info.uuid));
        uuids.insert(normalize_uuid(&offline_uuid(&info.username)));
    }

    (name, uuids)
}

fn normalize_uuid(uuid: &str) -> String {
    uuid.replace('-', "").to_lowercase()
}

/// Locates a world's `advancements`/`stats` player file, tolerating both the
/// vanilla layout (`<world>/<sub>/<uuid>.json`) and the layout some worlds use
/// where player data is nested under a `players/` folder
/// (`<world>/players/<sub>/<uuid>.json`).
fn locate_player_file(
    world_dir: &Path,
    sub: &str,
    uuids: &HashSet<String>,
) -> Option<std::path::PathBuf> {
    find_player_file(&world_dir.join(sub), uuids)
        .or_else(|| find_player_file(&world_dir.join("players").join(sub), uuids))
}

/// Finds `<dir>/<uuid>.json` whose stem matches one of the player's UUIDs.
fn find_player_file(dir: &Path, uuids: &HashSet<String>) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if uuids.contains(&normalize_uuid(stem)) {
                return Some(path);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Advancements
// ---------------------------------------------------------------------------

/// Reads one world's advancement file and merges completed advancements into
/// the aggregate, keeping the earliest completion across duplicates. Returns
/// whether the file was a readable advancements file.
fn merge_advancements(file: &Path, location: &str, agg: &mut Aggregate) -> bool {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(_) => return false,
    };
    let root: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let Some(map) = root.as_object() else {
        return false;
    };

    for (id, entry) in map {
        // Only real advancements (recipe unlocks and DataVersion are noise).
        let Some(id) = strip_ns(id) else { continue };
        if !is_real_advancement(&id) {
            continue;
        }
        let Some(obj) = entry.as_object() else { continue };
        if obj.get("done").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let at = latest_criterion(obj.get("criteria"));
        let candidate = AdvAgg {
            at,
            location: location.to_string(),
        };
        agg.done
            .entry(id)
            .and_modify(|existing| {
                if earlier(candidate.at, existing.at) {
                    existing.at = candidate.at;
                    existing.location = location.to_string();
                }
            })
            .or_insert(candidate);
    }
    true
}

/// An advancement completes when its last criterion is met, so we take the
/// latest criterion timestamp as this world's completion time.
fn latest_criterion(criteria: Option<&Value>) -> Option<DateTime<FixedOffset>> {
    let obj = criteria?.as_object()?;
    obj.values()
        .filter_map(|v| v.as_str())
        .filter_map(parse_ts)
        .max()
}

fn parse_ts(raw: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M:%S %z").ok()
}

/// True if `a` should replace `b` as the earliest completion. A known timestamp
/// always beats an unknown one; unknown never overrides a known.
fn earlier(a: Option<DateTime<FixedOffset>>, b: Option<DateTime<FixedOffset>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a < b,
        (Some(_), None) => true,
        _ => false,
    }
}

fn strip_ns(id: &str) -> Option<String> {
    Some(id.strip_prefix("minecraft:").unwrap_or(id).to_string())
}

fn is_real_advancement(id: &str) -> bool {
    ORDER.iter().any(|cat| id.starts_with(&format!("{cat}/")))
}

fn category_of(id: &str) -> String {
    id.split('/').next().unwrap_or("other").to_string()
}

/// Joins bundled metadata with scanned completions into the full grid. Earned
/// advancements missing from the bundle (e.g. from a newer game version) are
/// still included with a name derived from their id, so nothing is lost.
fn build_advancements(agg: &Aggregate) -> Vec<AdvancementView> {
    let meta: HashMap<String, Vec<AdvMeta>> =
        serde_json::from_str(ADV_META_JSON).unwrap_or_default();

    let mut out: Vec<AdvancementView> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for cat in ORDER {
        let Some(list) = meta.get(cat) else { continue };
        for item in list {
            seen.insert(item.id.clone());
            let done = agg.done.get(&item.id);
            out.push(AdvancementView {
                category: category_of(&item.id),
                name: item.name.clone(),
                done: done.is_some(),
                hidden: item.hidden,
                earned_at: done.and_then(|d| d.at).map(|t| t.to_rfc3339()),
                earned_in: done.map(|d| d.location.clone()),
                id: item.id.clone(),
            });
        }
    }

    // Completed advancements we don't have metadata for.
    for (id, done) in &agg.done {
        if seen.contains(id) {
            continue;
        }
        out.push(AdvancementView {
            category: category_of(id),
            name: derive_name(id),
            done: true,
            hidden: false,
            earned_at: done.at.map(|t| t.to_rfc3339()),
            earned_in: Some(done.location.clone()),
            id: id.clone(),
        });
    }

    out
}

fn derive_name(id: &str) -> String {
    let last = id.rsplit('/').next().unwrap_or(id);
    let mut name = String::new();
    for (i, word) in last.split('_').enumerate() {
        if i > 0 {
            name.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    name
}

fn build_categories(advancements: &[AdvancementView]) -> Vec<CategoryProgress> {
    ORDER
        .iter()
        .map(|cat| {
            let in_cat: Vec<&AdvancementView> =
                advancements.iter().filter(|a| a.category == *cat).collect();
            CategoryProgress {
                key: cat.to_string(),
                earned: in_cat.iter().filter(|a| a.done).count(),
                total: in_cat.len(),
            }
        })
        .filter(|c| c.total > 0)
        .collect()
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

struct StatsDelta {
    play_time: i64,
    mined: i64,
    killed: i64,
    deaths: i64,
}

/// Merges one world's stats file into the aggregate, returning this world's
/// contribution to the per-instance rollup. `None` if the file is unreadable.
fn merge_stats(file: &Path, agg: &mut Aggregate) -> Option<StatsDelta> {
    let text = std::fs::read_to_string(file).ok()?;
    let root: Value = serde_json::from_str(&text).ok()?;
    let stats = root.get("stats")?.as_object()?;

    let mut delta = StatsDelta {
        play_time: 0,
        mined: 0,
        killed: 0,
        deaths: 0,
    };

    for (category, entries) in stats {
        let Some(entries) = entries.as_object() else {
            continue;
        };
        let bucket = agg.stats.entry(category.clone()).or_default();
        for (key, value) in entries {
            let count = value.as_i64().unwrap_or(0);
            *bucket.entry(key.clone()).or_insert(0) += count;

            match category.as_str() {
                "minecraft:mined" => delta.mined += count,
                "minecraft:killed" => delta.killed += count,
                "minecraft:custom" if key == "minecraft:play_time" => delta.play_time += count,
                "minecraft:custom" if key == "minecraft:deaths" => delta.deaths += count,
                _ => {}
            }
        }
    }

    Some(delta)
}

fn build_stats(agg: &Aggregate) -> LifetimeStats {
    let custom = "minecraft:custom";
    let distance_cm: i64 = agg
        .stats
        .get(custom)
        .map(|m| {
            m.iter()
                .filter(|(k, _)| k.ends_with("_one_cm"))
                .map(|(_, v)| *v)
                .sum()
        })
        .unwrap_or(0);

    LifetimeStats {
        play_time_ticks: agg.stat(custom, "minecraft:play_time"),
        blocks_mined: agg.sum_category("minecraft:mined"),
        mobs_killed: agg.sum_category("minecraft:killed"),
        deaths: agg.stat(custom, "minecraft:deaths"),
        distance_cm,
        jumps: agg.stat(custom, "minecraft:jump"),
        items_picked_up: agg.sum_category("minecraft:picked_up"),
        damage_dealt: agg.stat(custom, "minecraft:damage_dealt"),
        times_slept: agg.stat(custom, "minecraft:sleep_in_bed"),
        top_mined: top_entries(agg, "minecraft:mined", 5),
        top_killed: top_entries(agg, "minecraft:killed", 5),
    }
}

fn top_entries(agg: &Aggregate, category: &str, n: usize) -> Vec<StatEntry> {
    let Some(map) = agg.stats.get(category) else {
        return Vec::new();
    };
    let mut entries: Vec<StatEntry> = map
        .iter()
        .map(|(k, v)| StatEntry {
            key: strip_ns(k).unwrap_or_else(|| k.clone()),
            count: *v,
        })
        .collect();
    entries.sort_by(|a, b| b.count.cmp(&a.count));
    entries.truncate(n);
    entries
}

// ---------------------------------------------------------------------------
// Custom achievements (data-driven rule engine)
// ---------------------------------------------------------------------------

fn build_custom(agg: &Aggregate) -> Vec<CustomAchievement> {
    let rules: Vec<CustomRule> = serde_json::from_str(CUSTOM_JSON).unwrap_or_default();

    rules
        .into_iter()
        .map(|rule| {
            let earned = rule.conditions.iter().all(|c| eval_condition(agg, c));
            let progress = rule
                .conditions
                .first()
                .map(|c| condition_progress(agg, c))
                .unwrap_or(0);
            CustomAchievement {
                id: rule.id,
                name: rule.name,
                description: rule.description,
                icon: rule.icon,
                earned,
                progress,
            }
        })
        .collect()
}

fn resolve_stat(agg: &Aggregate, key: &str) -> i64 {
    if let Some(category) = key.strip_prefix("sum:") {
        return agg.sum_category(category);
    }
    match key.split_once('/') {
        Some((category, sub)) => agg.stat(category, sub),
        None => 0,
    }
}

fn eval_condition(agg: &Aggregate, cond: &Condition) -> bool {
    let actual = resolve_stat(agg, &cond.stat);
    match cond.op.as_str() {
        "gte" => actual >= cond.value,
        "gt" => actual > cond.value,
        "lte" => actual <= cond.value,
        "lt" => actual < cond.value,
        "eq" => actual == cond.value,
        _ => false,
    }
}

/// Progress toward a "reach a threshold" condition. Zero-target conditions
/// (e.g. "never slept") don't have meaningful progress, so report 0.
fn condition_progress(agg: &Aggregate, cond: &Condition) -> u32 {
    if !matches!(cond.op.as_str(), "gte" | "gt") || cond.value <= 0 {
        return 0;
    }
    let actual = resolve_stat(agg, &cond.stat);
    pct(actual, cond.value)
}

fn pct(num: i64, den: i64) -> u32 {
    if den <= 0 {
        return 0;
    }
    ((num.max(0) as f64 / den as f64) * 100.0).round().min(100.0) as u32
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_metadata_parses() {
        let meta: HashMap<String, Vec<AdvMeta>> = serde_json::from_str(ADV_META_JSON).unwrap();
        assert!(meta.contains_key("story"));
        assert!(meta["end"].iter().any(|m| m.id == "end/kill_dragon"));
        let rules: Vec<CustomRule> = serde_json::from_str(CUSTOM_JSON).unwrap();
        assert!(rules.iter().any(|r| r.id == "pacifist"));
    }

    #[test]
    fn derives_readable_names() {
        assert_eq!(derive_name("nether/obtain_ancient_debris"), "Obtain Ancient Debris");
        assert_eq!(derive_name("story/root"), "Root");
    }

    #[test]
    fn sum_and_specific_stats_resolve() {
        let mut agg = Aggregate::default();
        let killed = agg.stats.entry("minecraft:killed".into()).or_default();
        killed.insert("minecraft:zombie".into(), 30);
        killed.insert("minecraft:creeper".into(), 12);
        let custom = agg.stats.entry("minecraft:custom".into()).or_default();
        custom.insert("minecraft:deaths".into(), 5);

        assert_eq!(resolve_stat(&agg, "sum:minecraft:killed"), 42);
        assert_eq!(resolve_stat(&agg, "minecraft:custom/minecraft:deaths"), 5);
        assert_eq!(resolve_stat(&agg, "minecraft:custom/minecraft:missing"), 0);
    }

    #[test]
    fn pacifist_requires_zero_kills() {
        let mut agg = Aggregate::default();
        agg.stats
            .entry("minecraft:custom".into())
            .or_default()
            .insert("minecraft:play_time".into(), 800_000);
        // No kills yet -> pacifist earned.
        let earned: Vec<_> = build_custom(&agg).into_iter().filter(|c| c.earned).collect();
        assert!(earned.iter().any(|c| c.id == "pacifist"));

        // One kill -> pacifist lost.
        agg.stats
            .entry("minecraft:killed".into())
            .or_default()
            .insert("minecraft:zombie".into(), 1);
        let earned: Vec<_> = build_custom(&agg).into_iter().filter(|c| c.earned).collect();
        assert!(!earned.iter().any(|c| c.id == "pacifist"));
    }

    #[test]
    fn locates_files_in_both_layouts() {
        use std::fs;
        let uuid = "04a84533-680d-4739-b4f1-1c33393c5074";
        let mut uuids = HashSet::new();
        uuids.insert(normalize_uuid(uuid));

        let base = std::env::temp_dir().join(format!("cactus-adv-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);

        // Vanilla layout: <world>/stats/<uuid>.json
        let vanilla = base.join("vanilla");
        fs::create_dir_all(vanilla.join("stats")).unwrap();
        fs::write(vanilla.join("stats").join(format!("{uuid}.json")), "{}").unwrap();
        assert!(locate_player_file(&vanilla, "stats", &uuids).is_some());

        // Nested layout: <world>/players/advancements/<uuid>.json
        let nested = base.join("nested");
        fs::create_dir_all(nested.join("players").join("advancements")).unwrap();
        fs::write(
            nested.join("players").join("advancements").join(format!("{uuid}.json")),
            "{}",
        )
        .unwrap();
        assert!(locate_player_file(&nested, "advancements", &uuids).is_some());
        // A non-matching uuid isn't picked up.
        let mut other = HashSet::new();
        other.insert(normalize_uuid("ffffffff-0000-0000-0000-000000000000"));
        assert!(locate_player_file(&nested, "advancements", &other).is_none());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn keeps_earliest_completion() {
        let mut agg = Aggregate::default();
        let early = parse_ts("2023-01-01 10:00:00 +0000");
        let late = parse_ts("2024-01-01 10:00:00 +0000");
        agg.done.insert(
            "story/mine_stone".into(),
            AdvAgg { at: late, location: "B".into() },
        );
        // Simulate merge picking the earlier one.
        agg.done.entry("story/mine_stone".into()).and_modify(|e| {
            if earlier(early, e.at) {
                e.at = early;
                e.location = "A".into();
            }
        });
        assert_eq!(agg.done["story/mine_stone"].location, "A");
    }
}
