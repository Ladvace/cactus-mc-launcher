//! `TuneupModal.svelte`: hardware-aware mods, memory and JVM flags for a client instance.

use std::collections::HashMap;

use rune::{Cursor, El, col, el, row, text};

use super::{content, t_};
use crate::app::{App, Ignore};
use crate::core::i18n::tf;
use crate::theme::Theme;
use crate::tuneup::{SelectedMod, TuneupPlan, TuneupSelection};
use crate::ui::{self, Btn};

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub plan: Option<TuneupPlan>,
    pub loading: bool,
    pub applying: bool,
    pub error: Option<String>,
    pub visuals: bool,
    pub picked: HashMap<String, bool>,
    pub apply_memory: bool,
    pub apply_flags: bool,
    pub max_mem: u32,
    /// Bumped per request so a late answer for an earlier mode is dropped.
    generation: u64,
}

pub fn open(app: &mut App) {
    let tuneup = &mut app.instance_page.tuneup;
    tuneup.open = true;
    tuneup.apply_memory = true;
    tuneup.apply_flags = true;
    recommend(app);
}

fn close(app: &mut App) {
    app.instance_page.tuneup.open = false;
    app.instance_page.tuneup.generation += 1;
}

fn recommend(app: &mut App) {
    let tuneup = &mut app.instance_page.tuneup;
    tuneup.generation += 1;
    let generation = tuneup.generation;
    tuneup.loading = true;
    tuneup.error = None;
    tuneup.plan = None;
    let mode = if tuneup.visuals { "visuals" } else { "performance" }.to_string();
    let id = app.instance_page.id.clone();
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::tuneup_recommend(host, id, Some(mode)).await }, move |app, result| {
        let tuneup = &mut app.instance_page.tuneup;
        if tuneup.generation != generation {
            return;
        }
        tuneup.loading = false;
        match result {
            Ok(plan) => {
                tuneup.picked = plan.mods.iter().map(|suggestion| (suggestion.version_id.clone(), suggestion.recommended)).collect();
                tuneup.max_mem = plan.max_mem_mb;
                tuneup.plan = Some(plan);
            }
            Err(e) => tuneup.error = Some(e.to_string()),
        }
    });
}

fn set_mode(app: &mut App, visuals: bool) {
    if app.instance_page.tuneup.visuals != visuals {
        app.instance_page.tuneup.visuals = visuals;
        recommend(app);
    }
}

fn min_mem(max: u32) -> u32 {
    max.min(1024.max(max / 2 / 512 * 512))
}

fn chosen_count(tuneup: &State) -> usize {
    tuneup.plan.as_ref().map_or(0, |plan| plan.mods.iter().filter(|suggestion| !suggestion.installed && tuneup.picked.get(&suggestion.version_id).copied().unwrap_or(false)).count())
}

fn nothing_to_do(tuneup: &State) -> bool {
    chosen_count(tuneup) == 0 && !tuneup.apply_memory && !tuneup.apply_flags
}

fn format_gb(mb: u64) -> String {
    let gigabytes = mb as f64 / 1024.0;
    if mb % 1024 == 0 { format!("{gigabytes:.0}") } else { format!("{gigabytes:.1}") }
}

fn apply(app: &mut App) {
    let tuneup = &app.instance_page.tuneup;
    let Some(plan) = &tuneup.plan else { return };
    if nothing_to_do(tuneup) || tuneup.applying {
        return;
    }
    let mods: Vec<SelectedMod> = plan
        .mods
        .iter()
        .filter(|suggestion| !suggestion.installed && tuneup.picked.get(&suggestion.version_id).copied().unwrap_or(false))
        .map(|suggestion| SelectedMod { version_id: suggestion.version_id.clone(), title: suggestion.title.clone() })
        .collect();
    let (apply_memory, apply_flags) = (tuneup.apply_memory, tuneup.apply_flags);
    let selection = TuneupSelection {
        mods,
        apply_memory,
        apply_flags,
        max_mem_mb: tuneup.max_mem,
        min_mem_mb: min_mem(tuneup.max_mem),
        jvm_args: plan.jvm_args.clone(),
    };
    app.instance_page.tuneup.applying = true;
    let id = app.instance_page.id.clone();
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::tuneup_apply(host, id, selection).await }, move |app, result| {
        app.instance_page.tuneup.applying = false;
        match result {
            Ok(count) => {
                let mut bits = Vec::new();
                if count > 0 {
                    bits.push(tf(if count == 1 { "tuneup.oneMod" } else { "tuneup.manyMods" }, &[("count", &count.to_string())]));
                }
                if apply_memory {
                    bits.push(t_("tuneup.memory"));
                }
                if apply_flags {
                    bits.push(t_("tuneup.jvmFlags"));
                }
                app.toasts.success(tf("tuneup.applied", &[("items", &bits.join(", "))]));
                content::load(app);
                app.refresh_instances();
                close(app);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn h4(theme: &Theme, label: String) -> El {
    text(label.to_uppercase()).font(rune::tok::TextStyle { tracking: 12.0 * 0.04, ..theme.pixel(12.0) }).color(theme.text_muted).mt(16.0).mb(6.0)
}

fn small(theme: &Theme, content: String) -> El {
    ui::muted(theme, content, 12.5).wrap()
}

pub fn view(app: &App, theme: &Theme) -> El {
    let tuneup = &app.instance_page.tuneup;
    let body = if tuneup.loading {
        col().w_full().items_center().gap(10.0).px(16.0).py(40.0)
            .child(ui::spinner(theme, 28.0))
            .child(text(t_("tuneup.loadingText")).font(theme.sans(14.0, 400.0)).color(theme.text).wrap().align(rune::Align::Center))
            .child(ui::muted(theme, t_("tuneup.loadingSub"), 12.5))
    } else if let Some(error) = &tuneup.error {
        col().child(text(error.clone()).font(theme.sans(14.0, 400.0)).color(theme.danger).wrap())
    } else if let Some(plan) = &tuneup.plan {
        plan_view(theme, tuneup, plan)
    } else {
        col()
    };
    let footer = ui::buttons([
        Btn::new(t_("common.cancel")).ghost().key("tuneup-cancel").on_press(close).build(theme),
        Btn::new(if tuneup.applying { t_("tuneup.applying") } else { t_("tuneup.apply") })
            .primary()
            .key("tuneup-apply")
            .disabled(tuneup.plan.is_none() || tuneup.applying || nothing_to_do(tuneup))
            .on_press(apply)
            .build(theme),
    ]);
    ui::modal(theme, "tuneup-modal", t_("tuneup.title"), 560.0, body, Some(footer), close)
}

fn plan_view(theme: &Theme, tuneup: &State, plan: &TuneupPlan) -> El {
    let mode = |key: &str, label: String, active: bool, visuals: bool| {
        el().key(key.to_string())
            .px(14.0)
            .py(6.0)
            .fill(if active { theme.accent } else { rune::Color::TRANSPARENT })
            .role("tab")
            .label(label.clone())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| set_mode(app, visuals))
            .child(text(label).font(theme.sans(13.5, if active { 600.0 } else { 400.0 })).color(if active { theme.accent_contrast } else { theme.text_muted }))
    };
    let modes = row()
        .key("tuneup-modes")
        .self_start()
        .border(1.0, theme.border)
        .mb(14.0)
        .child(mode("mode-performance", t_("tuneup.performance"), !tuneup.visuals, false))
        .child(mode("mode-visuals", t_("tuneup.visuals"), tuneup.visuals, true));

    let spec = |strong: Option<String>, rest: String| {
        let mut spec_row = row().gap(4.0);
        if let Some(bold) = strong {
            spec_row = spec_row.child(text(bold).font(theme.sans(13.5, 700.0)).color(theme.text));
        }
        spec_row.child(ui::muted(theme, rest, 13.5))
    };
    let specs = row()
        .key("tuneup-specs")
        .w_full()
        .flex_wrap()
        .gaps(6.0, 12.0)
        .px(12.0)
        .py(10.0)
        .fill(theme.bg_card)
        .border(1.0, theme.border)
        .mb(16.0)
        .child(spec(Some(format!("{} GB", format_gb(plan.specs.total_ram_mb))), t_("tuneup.ram")))
        .child(spec(Some(plan.specs.cpu_cores.to_string()), t_("tuneup.cores")))
        .child(spec(None, format!("{} · {}", plan.specs.os, plan.specs.arch)))
        .child(spec(None, format!("{} {}", plan.loader, plan.mc_version)));

    let mut body = col()
        .w_full()
        .child(ui::muted(theme, t_("tuneup.lead"), 14.0).wrap().mb(12.0))
        .child(modes)
        .child(specs);

    if plan.loader == "vanilla" {
        body = body.child(ui::muted(theme, t_("tuneup.vanillaNote"), 14.0).wrap());
    } else if !plan.mods.is_empty() {
        body = body.child(h4(theme, t_("tuneup.performanceMods")));
        if plan.mods.iter().all(|suggestion| suggestion.installed) {
            body = body.child(small(theme, t_("tuneup.allInstalled")));
        }
        let mut mods = col().gap(6.0);
        for suggestion in &plan.mods {
            let checked = suggestion.installed || tuneup.picked.get(&suggestion.version_id).copied().unwrap_or(false);
            let mut name = row().gap(8.0).center_y().child(text(suggestion.title.clone()).font(theme.sans(14.0, 600.0)).color(theme.text));
            if suggestion.installed {
                name = name.child(
                    el().px(6.0).py(1.0).border(1.0, theme.border).child(ui::muted(theme, t_("tuneup.installed"), 11.0)),
                );
            }
            let mut mod_row = row()
                .key(format!("mod-{}", suggestion.version_id))
                .gap(10.0)
                .center_y()
                .role("checkbox")
                .checked(checked)
                .label(suggestion.title.clone())
                .child(ui::checkbox(theme, checked).a11y_hidden())
                .child(col().grow(1.0).shrink().child(name).child(small(theme, suggestion.reason.clone())));
            if suggestion.installed {
                mod_row = mod_row.opacity(0.7).disabled(true);
            } else {
                let version_id = suggestion.version_id.clone();
                mod_row = mod_row.focusable(true).cursor(Cursor::Pointer).on_press(move |app: &mut App| {
                    let picked = app.instance_page.tuneup.picked.entry(version_id.clone()).or_default();
                    *picked = !*picked;
                });
            }
            mods = mods.child(mod_row);
        }
        body = body.child(mods);
    }
    if !plan.unavailable.is_empty() {
        body = body.child(small(theme, tf("tuneup.noBuild", &[("loader", &plan.loader), ("mcVersion", &plan.mc_version), ("list", &plan.unavailable.join(", "))])).mt(8.0));
    }

    body = body.child(h4(theme, t_("tuneup.javaTuning")));
    body = body.child(ui::check_row(theme, "tuneup-memory", t_("tuneup.setMemory"), tuneup.apply_memory, |app| {
        let tuneup = &mut app.instance_page.tuneup;
        tuneup.apply_memory = !tuneup.apply_memory;
    }).py(4.0));

    let ram_cap = plan.specs.total_ram_mb.max(2048) as u32;
    let steps = ((ram_cap - 1024) / 512).max(1);
    let fraction = (tuneup.max_mem.clamp(1024, ram_cap) - 1024) as f32 / (ram_cap - 1024).max(1) as f32;
    let slider = ui::slider(theme, "tuneup-heap", 280.0, fraction, steps, move |app, value| {
        let tuneup = &mut app.instance_page.tuneup;
        if tuneup.apply_memory {
            tuneup.max_mem = 1024 + (value * steps as f32).round() as u32 * 512;
        }
    });
    let heap_label = row().gap(4.0)
        .child(text(format!("{} GB", format_gb(tuneup.max_mem as u64))).font(theme.sans(13.5, 700.0)).color(theme.text))
        .child(ui::muted(theme, format!("{} · {} GB {}", t_("tuneup.max"), format_gb(min_mem(tuneup.max_mem) as u64), t_("tuneup.min")), 13.5));
    let heap = row().key("tuneup-heap-row").gap(12.0).center_y().pl(28.0).pb(6.0).child(slider).child(heap_label);
    body = body.child(if tuneup.apply_memory { heap } else { heap.opacity(0.45) });
    if plan.low_ram {
        body = body.child(text(t_("tuneup.lowRamWarning")).font(rune::tok::TextStyle { line_height: 18.0, ..theme.sans(12.8, 400.0) }).color(theme.warning).wrap().mt(6.0));
    }
    let aikar = if plan.jvm_args.contains("MaxGCPauseMillis") { " (Aikar's G1GC)" } else { "" };
    body.child(ui::check_row(theme, "tuneup-flags", format!("{}{aikar}", t_("tuneup.applyJvmFlags")), tuneup.apply_flags, |app| {
        let tuneup = &mut app.instance_page.tuneup;
        tuneup.apply_flags = !tuneup.apply_flags;
    }).py(4.0))
}
