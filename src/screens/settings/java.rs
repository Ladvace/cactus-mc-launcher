//! The Java & memory card: managed Java setup, a runtime per major version, memory, JVM
//! arguments and download concurrency.

use rune::{El, col, el, text};

use super::general::input;
use super::widgets::{card, label, linked_small, range, setting, switch, ticks};
use super::{JAVA_MAJORS, State, edit, field};
use crate::app::{App, to_percent};
use crate::core::i18n::{t, tf};
use crate::settings::Settings;
use crate::theme::Theme;
use crate::ui::{self, Btn};

const MEM_MIN: u32 = 1024;
const MEM_MAX: u32 = 16384;

pub fn load_managed(app: &mut App) {
    app.fx.blocking(|host| crate::commands::resolved_java_paths(host.clone()), |app, paths| app.settings_page.managed_paths = paths);
}

/// `javaManaged`: no own path for this major (or switched back to managed this visit).
pub fn managed(page: &State, major: u32) -> bool {
    let own = page.java_editing.get(&major).copied().unwrap_or_else(|| page.draft.java_paths.get(&major).is_some_and(|path| !path.trim().is_empty()));
    !own
}

pub fn set_path(settings: &mut Settings, major: u32, value: &str) {
    if value.is_empty() {
        settings.java_paths.remove(&major);
    } else {
        settings.java_paths.insert(major, value.to_string());
    }
}

/// `toggleJavaManaged`.
fn toggle(app: &mut App, idx: usize, major: u32) {
    let auto = !managed(&app.settings_page, major);
    app.settings_page.java_editing.insert(major, !auto);
    let page = &mut app.settings_page;
    if auto {
        set_path(&mut page.draft, major, "");
        page.java_paths[idx].replace_text("");
    } else if !page.draft.java_paths.contains_key(&major) {
        let path = page.managed_paths.get(&major).cloned().unwrap_or_default();
        set_path(&mut page.draft, major, &path);
        page.java_paths[idx].replace_text(path);
    }
    edit(app, |_| {});
}

fn setup_java(app: &mut App) {
    let java = &mut app.settings_page.java;
    java.busy = true;
    java.error = None;
    java.installed.clear();
    java.label = t("settings.fetchingRuntimeList");
    java.current = 0;
    java.total = 0;
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::setup_java(host).await }, |app, result| {
        let java = &mut app.settings_page.java;
        match result {
            Ok(list) => java.installed = list,
            Err(e) => java.error = Some(e.to_string()),
        }
        java.busy = false;
        java.label.clear();
        java.current = 0;
        java.total = 0;
        load_managed(app);
    });
}

pub fn view(app: &App, theme: &Theme) -> El {
    let page = &app.settings_page;
    let draft = &page.draft;
    let java = &page.java;
    let mut rows = vec![setting(
        theme,
        "managed-java",
        true,
        label(theme, t("settings.managedJava"), Some(t("settings.managedJavaDesc"))),
        Btn::new(if java.busy { t("settings.installing") } else { t("settings.autoInstallJava") })
            .key("setup-java")
            .primary()
            .disabled(java.busy)
            .on_press(setup_java)
            .build(theme),
    )];
    if java.busy || !java.installed.is_empty() || java.error.is_some() {
        let body = if java.busy {
            ui::progress(theme, if java.label.is_empty() { t("settings.preparing") } else { java.label.clone() }, to_percent(java.current, java.total))
        } else if let Some(error) = &java.error {
            text(error.clone()).font(theme.sans(13.0, 400.0)).color(theme.danger).wrap()
        } else {
            text(tf("settings.javaReady", &[("list", &java.installed.join(", "))])).font(theme.sans(13.0, 500.0)).color(theme.accent).wrap()
        };
        rows.push(col().key("java-status").w_full().pb(14.0).child(body));
    }
    rows.push(setting(theme, "java-runtimes", false, label(theme, t("settings.javaRuntimes"), Some(t("settings.javaRuntimesDesc"))), el()));
    for (i, major) in JAVA_MAJORS.iter().copied().enumerate() {
        let auto = managed(page, major);
        let managed_path = page.managed_paths.get(&major);
        let desc = if !auto {
            t("settings.usingOwnJava")
        } else if managed_path.is_some() {
            t("settings.managedByCactus")
        } else {
            t("settings.downloadedWhenNeeded")
        };
        rows.push(
            setting(
                theme,
                &format!("java-{major}"),
                false,
                label(theme, tf("settings.javaVersion", &[("major", &major.to_string())]), Some(desc)),
                switch(theme, &format!("java-{major}-switch"), &t("settings.manageAutomatically"), auto, move |app| toggle(app, i, major))
                    .help(t("settings.manageAutomatically")),
            )
            .pb(2.0),
        );
        let field_el = if auto {
            let shown = managed_path.cloned().unwrap_or_default();
            let (label_text, color) = if shown.is_empty() { (t("settings.javaPathPlaceholder"), theme.text_muted) } else { (shown, theme.text) };
            ui::input_frame(theme, false)
                .key(format!("java-{major}-input"))
                .w_full()
                .opacity(0.55)
                .disabled(true)
                .child(text(label_text).font(theme.sans(12.5, 400.0)).color(color).truncate().grow(1.0).shrink())
        } else {
            input(theme, &page.java_paths[i], &format!("java-{major}-input"), tf("settings.javaVersion", &[("major", &major.to_string())]), t("settings.javaPathPlaceholder"), None, false, move |app| {
                &mut field(app).settings_page.java_paths[i]
            })
        };
        rows.push(el().w_full().mb(10.0).child(field_el));
    }

    let mem_span = (MEM_MAX - MEM_MIN) as f32;
    let memory = col()
        .w(240.0)
        .child(range(theme, "memory-range", 240.0, (draft.max_memory_mb.clamp(MEM_MIN, MEM_MAX) - MEM_MIN) as f32 / mem_span, 30, 15, false, move |app, fraction| {
            let mb = MEM_MIN + ((fraction * mem_span) / 512.0).round() as u32 * 512;
            if mb != app.settings_page.draft.max_memory_mb {
                edit(app, |draft| draft.max_memory_mb = mb);
            }
        }))
        .child(ticks(
            theme,
            240.0,
            [1u32, 4, 8, 12, 16].iter().map(|gb| ((gb * 1024 - MEM_MIN) as f32 / mem_span, gb.to_string(), Some("GB".to_string()))).collect(),
        ));
    rows.push(setting(theme, "max-memory", false, label(theme, t("settings.maxMemory"), Some(format!("{} MB", draft.max_memory_mb))), memory));
    rows.push(setting(
        theme,
        "jvm-args",
        false,
        label(theme, t("settings.jvmArguments"), Some(t("settings.jvmArgumentsDesc"))),
        input(theme, &page.jvm_args, "jvm-input", t("settings.jvmArguments"), "-XX:+UseG1GC …", Some(240.0), false, |app| &mut field(app).settings_page.jvm_args),
    ));
    let n = draft.max_concurrent_downloads.clamp(1, 64);
    let concurrency_label = col().gap(2.0).grow(1.0).shrink().child(text(t("settings.concurrentDownloads")).font(theme.sans(13.5, 500.0)).color(theme.text)).child(
        linked_small(theme, "concurrency-desc", &[
            (&tf("settings.concurrentDownloadsDesc", &[("count", &n.to_string())]), None),
            (&t("settings.slower"), None),
            (".", None),
        ]),
    );
    rows.push(setting(
        theme,
        "concurrency",
        false,
        concurrency_label,
        range(theme, "concurrency-range", 240.0, (n - 1) as f32 / 63.0, 63, 0, false, |app, fraction| {
            let n = 1 + (fraction * 63.0).round() as u32;
            if n != app.settings_page.draft.max_concurrent_downloads {
                edit(app, |draft| draft.max_concurrent_downloads = n);
            }
        }),
    ));
    card(theme, "java", t("settings.javaMemory"), rows)
}
