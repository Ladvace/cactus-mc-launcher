//! The routes. `on_enter` runs when one becomes current (and at start for Home).

pub mod achievements;
pub mod browse;
pub mod home;
pub mod instance;
pub mod servers;
pub mod settings;
pub mod share;

use rune::{Cx, El, col, text};

use crate::app::{App, Route};

pub fn on_enter(app: &mut App) {
    match app.route.clone() {
        Route::Home => home::on_enter(app),
        Route::Browse => browse::on_enter(app),
        Route::Servers => servers::on_enter(app),
        Route::Community | Route::ShareCreator | Route::ShareBoard(_) => share::on_enter(app),
        Route::Achievements => achievements::on_enter(app),
        Route::Settings => settings::on_enter(app),
        Route::Instance(_) => instance::on_enter(app),
    }
}

pub fn on_event(app: &mut App, name: &str, payload: &serde_json::Value) {
    home::on_event(app, name, payload);
    instance::on_event(app, name, payload);
    browse::on_event(app, name, payload);
    servers::on_event(app, name, payload);
    achievements::on_event(app, name, payload);
    share::on_event(app, name, payload);
    settings::on_event(app, name, payload);
}

pub fn view(app: &App, cx: &mut Cx<App>, layer: &mut Vec<El>) -> El {
    match &app.route {
        Route::Home => home::view(app, cx, layer),
        Route::Browse => browse::view(app, cx, layer),
        Route::Servers => servers::view(app, cx, layer),
        Route::Community | Route::ShareCreator | Route::ShareBoard(_) => share::view(app, cx, layer),
        Route::Achievements => achievements::view(app, cx, layer),
        Route::Settings => settings::view(app, cx, layer),
        Route::Instance(_) => instance::view(app, cx, layer),
    }
}

/// What a page not yet ported shows.
pub fn placeholder(app: &App, name: &str) -> El {
    let theme = app.theme();
    col().w_full().h_full().center().gap(8.0)
        .child(text(name.to_string()).font(theme.pixel(22.0)).color(theme.text))
        .child(text("Not ported yet").font(theme.sans(13.0, 400.0)).color(theme.text_muted))
}
