//! `WorldsList.svelte`: the instance's saves (client) or root-level worlds (server).

use rune::{El, col, row, rich, text};

use super::{reveal, reveal_or_toast, t_, tag_label};
use crate::app::{App, Ignore};
use crate::core::i18n::tf;
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};
use crate::worlds::WorldInfo;

#[derive(Default)]
pub struct State {
    pub list: Vec<WorldInfo>,
    pub loading: bool,
    pub busy: Option<String>,
    pub confirm: Option<String>,
}

pub fn load(app: &mut App) {
    app.instance_page.worlds.loading = true;
    let id = app.instance_page.id.clone();
    app.fx.blocking(move |host| (crate::commands::list_worlds(host.clone(), id.clone()), id), |app, (result, id)| {
        if app.instance_page.id != id {
            return;
        }
        app.instance_page.worlds.loading = false;
        match result {
            Ok(list) => app.instance_page.worlds.list = list,
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn backup(app: &mut App, world: WorldInfo) {
    app.instance_page.worlds.busy = Some(world.folder.clone());
    let id = app.instance_page.id.clone();
    let folder = world.folder.clone();
    app.fx.blocking(move |host| crate::commands::backup_world(host.clone(), id, folder), move |app, result| {
        app.instance_page.worlds.busy = None;
        match result {
            Ok(path) => {
                app.toasts.success(tf("server.backedUp", &[("name", &world.name)]));
                let _ = reveal(&path);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn remove(app: &mut App, world: WorldInfo) {
    app.instance_page.worlds.busy = Some(world.folder.clone());
    let id = app.instance_page.id.clone();
    let folder = world.folder.clone();
    app.fx.blocking(move |host| crate::commands::delete_world(host.clone(), id, folder), move |app, result| {
        app.instance_page.worlds.busy = None;
        match result {
            Ok(()) => {
                app.instance_page.worlds.confirm = None;
                app.toasts.success(tf("server.deleted", &[("name", &world.name)]));
                load(app);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

pub fn view(app: &App, theme: &Theme) -> El {
    let worlds = &app.instance_page.worlds;
    let running = app.is_running(&app.instance_page.id);
    let n = worlds.list.len();
    let count = tf(if n == 1 { "server.worldCountOne" } else { "server.worldCountMany" }, &[("count", &n.to_string())]);
    let head_text = if running {
        let warn = t_("server.stopBeforeDelete");
        let combined = format!("{count} · {warn}");
        rich(combined, vec![rune::Span::new(count.len() + " · ".len(), count.len() + " · ".len() + warn.len()).weight(700.0)])
    } else {
        text(count)
    };
    let head = row()
        .w_full()
        .justify_between()
        .center_y()
        .gap(12.0)
        .mb(14.0)
        .child(head_text.font(theme.sans(12.5, 400.0)).color(theme.text_muted).wrap().grow(1.0).shrink())
        .child(Btn::new(t_("server.reload")).ghost().sm().key("worlds-reload").disabled(worlds.loading).on_press(load).build(theme));
    let mut panel = col().w_full().child(head);

    if worlds.loading && worlds.list.is_empty() {
        return panel.child(ui::muted(theme, t_("common.loading"), 12.5));
    }
    if worlds.list.is_empty() {
        return panel.child(
            col().key("worlds-empty").w_full().items_center().gap(10.0).px(20.0).py(40.0)
                .child(ui::colored_icon(&icons::GLOBE, 30.0, theme.text_muted).opacity(0.6))
                .child(ui::muted(theme, t_("server.noWorlds"), 14.0).wrap().align(rune::Align::Center)),
        );
    }

    let mut list = col().key("worlds-list").gap(8.0);
    for world in &worlds.list {
        let busy = worlds.busy.as_deref() == Some(world.folder.as_str());
        let confirming = worlds.confirm.as_deref() == Some(world.folder.as_str());
        let location = if world.location == "server" { t_("server.locServer") } else { t_("server.locSave") };
        let meta = tf("server.worldMeta", &[
            ("size", &ui::fmt::bytes(world.size_bytes)),
            ("ago", &ui::fmt::time_ago(world.last_modified.as_deref().and_then(|stamp| chrono::DateTime::parse_from_rfc3339(stamp).ok()).map(|modified| modified.to_utc()), "never")),
        ]);
        let info = col()
            .grow(1.0)
            .shrink()
            .child(row().gap(8.0).center_y()
                .child(text(world.name.clone()).font(theme.sans(14.0, 600.0)).color(theme.text).truncate().shrink().help(world.folder.clone()))
                .child(tag_label(theme, location)))
            .child(ui::muted(theme, meta, 12.0));

        let key = &world.folder;
        let mut actions = row().gap(6.0).center_y();
        let path = world.path.clone();
        actions = actions.child(Btn::icon_only(&icons::FOLDER).ghost().sm().key(format!("reveal-{key}")).help(t_("server.showInFileManager")).on_press(move |app| reveal_or_toast(app, &path)).build(theme).label(t_("server.showInFileManager")));
        let target = world.clone();
        actions = actions.child(
            Btn::new(if busy && !confirming { t_("server.zipping") } else { t_("server.backup") })
                .ghost().sm().key(format!("backup-{key}")).disabled(busy).on_press(move |app| backup(app, target.clone())).build(theme),
        );
        if confirming {
            let target = world.clone();
            actions = actions
                .child(Btn::new(if busy { t_("server.deleting") } else { t_("server.confirm") }).danger().sm().key(format!("confirm-{key}")).disabled(busy).on_press(move |app| remove(app, target.clone())).build(theme))
                .child(Btn::new(t_("common.cancel")).ghost().sm().key(format!("cancel-{key}")).on_press(|app| app.instance_page.worlds.confirm = None).build(theme));
        } else {
            let folder = world.folder.clone();
            actions = actions.child(Btn::icon_only(&icons::TRASH).ghost().sm().key(format!("delete-{key}")).help(t_("server.deleteWorld")).on_press(move |app| app.instance_page.worlds.confirm = Some(folder.clone())).build(theme).label(t_("server.deleteWorld")));
        }

        list = list.child(
            row().key(format!("world-{key}")).gap(12.0).center_y().justify_between().px(14.0).py(12.0).fill(theme.bg_card).border(2.0, theme.border).child(info).child(actions),
        );
    }
    panel = panel.child(list);
    panel
}
