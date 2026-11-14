//! SkinPanel.svelte: the active account's skin in 3D (or Minotar's 2D render), its model and
//! cape, and changing, drawing or resetting the skin.

use std::collections::BTreeMap;
use std::f32::consts::PI;

use rune::{Color, Cursor, Drag, El, FileDialogOptions, Phase, SurfaceFit, Tx, col, el, row, text};
use serde::Deserialize;

use super::viewer::{Model, Texture, Viewer};
use super::{editor, skins};
use crate::app::{App, Ignore};
use crate::auth::AccountInfo;
use crate::core::i18n::{t as tr, tf};
use crate::icons;
use crate::theme::Theme;
use crate::ui::{self, Btn};

/// The stage is 170×250 with a 2 pt border; the canvas fills its inside.
const VIEW_W: f32 = 166.0;
const VIEW_H: f32 = 246.0;

#[derive(Clone, Debug, Deserialize)]
pub struct Cape {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub active: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    ThreeD,
    TwoD,
}

#[derive(Default)]
pub struct State {
    /// The account this panel was mounted for (`{#key active.id}`).
    pub uuid: Option<String>,
    pub mode: Mode,
    pub variant: Model,
    pub variant_open: bool,
    pub skin_data: String,
    pub capes: Vec<Cape>,
    pub cape_thumbs: BTreeMap<String, String>,
    pub capes_loading: bool,
    pub capes_error: Option<String>,
    pub changing: bool,
    pub drag_over: bool,
    pub skin_msg: Option<(u64, String)>,
    msg_seq: u64,
    pub viewer: Viewer,
    drag_base: (f32, f32),
}

/// Mount for `acc`: reset, then load its skin and capes, as the component's `$effect`s do.
pub fn mount(app: &mut App, acc: &AccountInfo) {
    app.account_modal.panel.viewer.stop();
    let viewer = app.account_modal.panel.viewer.clone();
    viewer.update(|scene| *scene = super::viewer::Scene { size: scene.size, ..Default::default() }, Some(0.0));
    app.account_modal.panel = State {
        uuid: Some(acc.uuid.clone()),
        capes_loading: true,
        viewer,
        ..State::default()
    };
    let uuid = acc.uuid.clone();
    match app.account_modal.skins.full(&uuid).map(str::to_string) {
        Some(full) => set_skin_data(app, full),
        None => {
            let url = format!("https://minotar.net/skin/{uuid}");
            let for_uuid = uuid.clone();
            app.fx.spawn(async move { crate::commands::download_image(url).await }, move |app, result| {
                if app.account_modal.panel.uuid.as_deref() == Some(&for_uuid) {
                    set_skin_data(app, result.unwrap_or_default());
                }
            });
        }
    }
    let host = app.fx.host.clone();
    app.fx.spawn(async move { crate::commands::get_capes(host).await }, move |app, result| {
        let panel = &mut app.account_modal.panel;
        if panel.uuid.as_deref() != Some(&uuid) {
            return;
        }
        panel.capes_loading = false;
        match result {
            Ok(list) => {
                let list: Vec<Cape> = list.iter().filter_map(|raw| serde_json::to_value(raw).ok().and_then(|value| serde_json::from_value(value).ok())).collect();
                panel.capes = list;
                load_active_cape(app);
                load_cape_thumbs(app, uuid);
            }
            Err(e) => {
                panel.capes = Vec::new();
                panel.capes_error = Some(e.to_string());
            }
        }
    });
}

pub fn unmount(app: &mut App) {
    let panel = &mut app.account_modal.panel;
    panel.viewer.stop();
    panel.uuid = None;
    panel.variant_open = false;
}

/// The skin as a data URI: shown in 3D once decoded, empty while loading or when it failed.
pub fn set_skin_data(app: &mut App, uri: String) {
    let texture = skins::texture_from_data_uri(&uri).map(std::sync::Arc::new);
    let panel = &mut app.account_modal.panel;
    panel.skin_data = uri;
    let animate = texture.is_some();
    panel.viewer.update(|scene| {
        scene.skin = texture;
        scene.animate = animate;
    }, Some(0.0));
    if animate {
        panel.viewer.run();
    }
}

fn load_active_cape(app: &mut App) {
    let panel = &app.account_modal.panel;
    let Some(url) = panel.capes.iter().find(|cape| cape.active).map(|cape| cape.url.clone()) else {
        panel.viewer.update(|scene| scene.cape = None, None);
        return;
    };
    let uuid = panel.uuid.clone();
    app.fx.spawn(async move { crate::commands::download_image(url).await }, move |app, result| {
        let panel = &app.account_modal.panel;
        if panel.uuid != uuid {
            return;
        }
        let cape = result
            .ok()
            .and_then(|uri| skins::data_uri_bytes(&uri))
            .and_then(|bytes| skins::decode(&bytes))
            .and_then(|(w, h, rgba)| Texture::cape(w, h, rgba))
            .map(std::sync::Arc::new);
        panel.viewer.update(|scene| scene.cape = cape, None);
    });
}

fn load_cape_thumbs(app: &mut App, uuid: String) {
    let capes: Vec<(String, String)> = app.account_modal.panel.capes.iter().map(|cape| (cape.id.clone(), cape.url.clone())).collect();
    app.fx.spawn(
        async move {
            let mut thumbs = BTreeMap::new();
            for (id, url) in capes {
                let Ok(uri) = crate::commands::download_image(url).await else { continue };
                if let Some(img) = skins::data_uri_bytes(&uri).and_then(|bytes| skins::cape_front(&bytes)) {
                    thumbs.insert(id, skins::png_data_uri(&img));
                }
            }
            thumbs
        },
        move |app, thumbs| {
            if app.account_modal.panel.uuid.as_deref() == Some(&uuid) {
                app.account_modal.panel.cape_thumbs = thumbs;
            }
        },
    );
}

fn choose_cape(app: &mut App, id: Option<String>) {
    let host = app.fx.host.clone();
    let request_id = id.clone();
    app.fx.spawn(async move { crate::commands::set_cape(host, request_id).await }, move |app, result| match result {
        Ok(()) => {
            for cape in &mut app.account_modal.panel.capes {
                cape.active = Some(&cape.id) == id.as_ref();
            }
            load_active_cape(app);
        }
        Err(e) => app.toasts.error(e.to_string()).ignore(),
    });
}

fn flash(app: &mut App, msg: String) {
    let panel = &mut app.account_modal.panel;
    panel.msg_seq += 1;
    let seq = panel.msg_seq;
    panel.skin_msg = Some((seq, msg));
    app.fx.after(std::time::Duration::from_secs(4), move |app| {
        let panel = &mut app.account_modal.panel;
        if panel.skin_msg.as_ref().is_some_and(|(shown_seq, _)| *shown_seq == seq) {
            panel.skin_msg = None;
        }
    });
}

fn variant_str(model: Model) -> String {
    if model == Model::Slim { "slim".into() } else { "classic".into() }
}

/// `applySkinFile`: upload, then show and cache what was uploaded.
pub fn apply_skin_bytes(app: &mut App, bytes: Vec<u8>) {
    let Some(uuid) = app.account_modal.panel.uuid.clone() else { return };
    let panel = &mut app.account_modal.panel;
    panel.changing = true;
    panel.skin_msg = None;
    let host = app.fx.host.clone();
    let variant = variant_str(panel.variant);
    let uri = skins::bytes_data_uri(&bytes);
    app.fx.spawn(async move { crate::commands::set_skin(host, bytes, variant).await }, move |app, result| {
        app.account_modal.panel.changing = false;
        match result {
            Ok(()) => {
                if app.account_modal.panel.uuid.as_deref() == Some(&uuid) {
                    set_skin_data(app, uri.clone());
                    flash(app, tr("account.skinUpdated"));
                }
                skins::set_skin(app, &uuid, uri);
            }
            Err(e) => app.toasts.error(e.to_string()).ignore(),
        }
    });
}

fn apply_skin_file(app: &mut App, path: std::path::PathBuf) {
    app.fx.blocking(move |_| std::fs::read(path), |app, result| match result {
        Ok(bytes) => apply_skin_bytes(app, bytes),
        Err(e) => app.toasts.error(e.to_string()).ignore(),
    });
}

fn reset_skin(app: &mut App) {
    let Some(uuid) = app.account_modal.panel.uuid.clone() else { return };
    let panel = &mut app.account_modal.panel;
    panel.changing = true;
    panel.skin_msg = None;
    let host = app.fx.host.clone();
    app.fx.spawn(
        async move {
            crate::commands::reset_skin(host).await?;
            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
            Ok::<_, crate::error::AppError>(crate::commands::download_image(format!("https://minotar.net/skin/{uuid}?ts={ts}")).await.unwrap_or_default())
        },
        {
            let uuid = app.account_modal.panel.uuid.clone().unwrap_or_default();
            move |app, result| {
                app.account_modal.panel.changing = false;
                match result {
                    Ok(uri) => {
                        skins::clear(app, &uuid);
                        if app.account_modal.panel.uuid.as_deref() == Some(&uuid) {
                            set_skin_data(app, uri);
                            flash(app, tr("account.skinReset"));
                        }
                    }
                    Err(e) => app.toasts.error(e.to_string()).ignore(),
                }
            }
        },
    );
}

fn label(theme: &Theme, caption: String) -> El {
    text(caption.to_uppercase()).font(rune::tok::TextStyle { tracking: 11.0 * 0.04, ..theme.sans(11.0, 400.0) }).color(theme.text_muted).mt(4.0)
}

pub fn view(app: &App, cx: &mut rune::Cx<App>, acc: &AccountInfo, layer: &mut Vec<El>) -> El {
    let theme = app.theme();
    let panel = &app.account_modal.panel;
    let scale = cx.env().scale;
    panel.viewer.resize(((VIEW_W * scale).round() as usize, (VIEW_H * scale).round() as usize));

    let mut stage = el()
        .key("skin-stage")
        .w(170.0)
        .min_h(250.0)
        .self_stretch()
        .center()
        .fill(theme.bg_app)
        .radial_gradient([0.5, 0.35], 183.0, [(0.0, theme.bg_card), (1.0, theme.bg_app)])
        .border(2.0, if panel.drag_over { theme.accent } else { theme.border_subtle })
        .on_drop_files(|app: &mut App, files: &[std::path::PathBuf], _: &mut Tx| {
            app.account_modal.panel.drag_over = false;
            if let Some(path) = files.first() {
                apply_skin_file(app, path.clone());
            }
        })
        .accepts_extensions(&["png"])
        .on_drop_hover(|app: &mut App, over, _: &mut Tx| app.account_modal.panel.drag_over = over);
    if panel.drag_over {
        stage = stage.border_dash(6.0, 4.0);
    }
    stage = match panel.mode {
        Mode::ThreeD if !panel.skin_data.is_empty() => stage.child(
            el().key("skin-viewer")
                .size(VIEW_W, VIEW_H)
                .pixels(&panel.viewer.pixels)
                .role("img")
                .label(tf("account.skinAlt", &[("username", &acc.username)]))
                .cursor(Cursor::Grab)
                .on_drag(|app: &mut App, drag: &mut Drag, tx: &mut Tx| {
                    let viewer = app.account_modal.panel.viewer.clone();
                    match drag.phase {
                        Phase::Down => {
                            let scene = viewer.scene();
                            app.account_modal.panel.drag_base = (scene.yaw, scene.pitch);
                        }
                        _ => {
                            let (yaw0, pitch0) = app.account_modal.panel.drag_base;
                            let radians_per_pt = 2.0 * PI / VIEW_H;
                            viewer.update(|scene| {
                                scene.yaw = yaw0 + drag.dx * radians_per_pt;
                                scene.pitch = (pitch0 + drag.dy * radians_per_pt).clamp(-PI / 2.0, PI / 2.0);
                            }, None);
                            tx.instant();
                        }
                    }
                }),
        ),
        Mode::ThreeD => stage.child(ui::skeleton(&theme, Some(110.0), 200.0)),
        Mode::TwoD => {
            let src = app.account_modal.skins.body(&acc.uuid).map_or_else(|| format!("https://minotar.net/armor/body/{}/210.png", acc.uuid), str::to_string);
            let img = el().key("skin-2d").size(115.0, 230.0).role("img").label(tf("account.skinAlt", &[("username", &acc.username)]));
            stage.child(match app.images.get(&app.fx, &src) {
                Some(picture) => img.pixels_fit(picture.pixels(), SurfaceFit::Fit),
                None => img,
            })
        }
    };
    if panel.drag_over {
        stage = stage.child(
            el().key("drop-hint")
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .center()
                .fill(Color::BLACK.alpha(0.55))
                .child(text(tr("account.dropPngSkin")).font(theme.sans(13.0, 400.0)).color(theme.accent)),
        );
    }

    let mode_btn = |key: &str, caption: &str, mode: Mode| {
        let on = panel.mode == mode;
        el().key(key.to_string())
            .px(14.0)
            .py(5.0)
            .fill(if on { theme.accent } else { theme.bg_card })
            .role("button")
            .label(caption.to_string())
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press(move |app: &mut App| app.account_modal.panel.mode = mode)
            .child(text(caption.to_string()).font(theme.pixel(12.0)).color(if on { theme.accent_contrast } else { theme.text_secondary }))
    };
    let toggle = row().key("mode-toggle").self_start().border(2.0, theme.border).child(mode_btn("mode-3d", "3D", Mode::ThreeD)).child(mode_btn("mode-2d", "2D", Mode::TwoD));

    let variants = [tr("account.modelClassic"), tr("account.modelSlim")];
    let current = if panel.variant == Model::Slim { &variants[1] } else { &variants[0] };
    let slim_label = variants[1].clone();
    let select = rune_kit::Select::new("model", &variants)
        .appearance(rune_kit::SelectAppearance::Field)
        .width(206.0)
        .label(tr("account.model"))
        .current(Some(current))
        .open(panel.variant_open)
        .on_toggle(|app: &mut App| app.account_modal.panel.variant_open = !app.account_modal.panel.variant_open)
        .on_close(|app: &mut App| app.account_modal.panel.variant_open = false)
        .on_choose(move |app: &mut App, choice: &str, _: &mut Tx| {
            let panel = &mut app.account_modal.panel;
            panel.variant_open = false;
            panel.variant = if choice == slim_label { Model::Slim } else { Model::Classic };
            let model = panel.variant;
            panel.viewer.update(|scene| scene.model = model, None);
        })
        .build(&theme.kit(), layer);

    let capes = if panel.capes_loading {
        row().key("capes").gap(6.0).children((0..3).map(|i| ui::skeleton(&theme, Some(44.0), 62.0).key(format!("cape-skel-{i}"))))
    } else if let Some(err) = &panel.capes_error {
        text(err.clone()).font(theme.sans(11.5, 400.0)).color(theme.danger).wrap().key("capes")
    } else if !panel.capes.is_empty() {
        let none_on = !panel.capes.iter().any(|cape| cape.active);
        let none_thumb = el().size(30.0, 48.0).center().fill(theme.bg_app).border(1.0, theme.border_subtle).child(ui::colored_icon(&icons::CLOSE, 16.0, theme.text_muted));
        let mut list = row().key("capes").gap(6.0).flex_wrap().child(cape_btn(app, &theme, "cape-none", &tr("account.capeNone"), none_on, none_thumb, None));
        for cape in &panel.capes {
            let thumb = match panel.cape_thumbs.get(&cape.id) {
                Some(src) => ui::image(app, src, 30.0, 48.0).border(1.0, theme.border_subtle).fill(theme.bg_app),
                None => el().size(30.0, 48.0).fill(theme.bg_app).border(1.0, theme.border_subtle),
            };
            list = list.child(cape_btn(app, &theme, &format!("cape-{}", cape.id), &cape.alias, cape.active, thumb, Some(cape.id.clone())));
        }
        list
    } else {
        text(tr("account.capeEmpty")).font(theme.sans(11.5, 400.0)).color(theme.text_muted).key("capes")
    };

    let actions = col()
        .key("skin-actions")
        .gap(6.0)
        .child(
            Btn::new(if panel.changing { tr("account.applying") } else { tr("account.changeSkin") })
                .key("change-skin")
                .primary()
                .icon(&icons::UPLOAD)
                .full()
                .disabled(panel.changing)
                .on_press_tx(|_: &mut App, tx: &mut Tx| {
                    let options = FileDialogOptions { title: tr("account.changeSkin"), ..Default::default() }.filter("PNG", &["png"]);
                    tx.open_file(options, |app: &mut App, result, _| match result {
                        Ok(Some(path)) => apply_skin_file(app, path),
                        Ok(None) => {}
                        Err(e) => app.toasts.error(e.to_string()).ignore(),
                    });
                })
                .build(&theme),
        )
        .child(
            Btn::new(tr("skinEditor.drawSkin"))
                .key("draw-skin")
                .icon(&icons::EDIT)
                .full()
                .on_press(editor::open)
                .build(&theme),
        )
        .child(
            Btn::new(tr("account.resetSkin"))
                .key("reset-skin")
                .ghost()
                .sm()
                .icon(&icons::REFRESH)
                .full()
                .disabled(panel.changing)
                .on_press(reset_skin)
                .build(&theme),
        );

    let controls = col()
        .key("skin-controls")
        .gap(8.0)
        .grow(1.0)
        .min_w(0.0)
        .justify_center()
        .child(toggle)
        .child(label(&theme, tr("account.model")))
        .child(select)
        .child(label(&theme, tr("account.cape")))
        .child(capes)
        .child(actions);

    let mut out = col()
        .key("skin-panel-wrap")
        .child(row().key("skin-panel").gap(16.0).padding(14.0).mb(8.0).fill(theme.bg_input).border(2.0, theme.border).child(stage).child(controls));
    if let Some((_, msg)) = &panel.skin_msg {
        out = out.child(text(msg.clone()).font(theme.sans(12.0, 400.0)).color(theme.accent).mb(2.0).key("skin-msg"));
    }
    out.child(text(tr("account.skinUploadHint")).font(theme.sans(11.0, 400.0)).color(theme.text_muted).mb(16.0).key("skin-hint"))
}

fn cape_btn(app: &App, theme: &Theme, key: &str, name: &str, on: bool, thumb: El, id: Option<String>) -> El {
    let hovered = app.account_modal.hovered.as_deref() == Some(key);
    let hover_key = key.to_string();
    col()
        .key(key.to_string())
        .items_center()
        .gap(4.0)
        .w(44.0)
        .px(4.0).pt(4.0).pb(5.0)
        .fill(theme.bg_card)
        .border(2.0, if on || hovered { theme.accent } else { theme.border })
        .role("button")
        .label(name.to_string())
        .help(name.to_string())
        .focusable(true)
        .cursor(Cursor::Pointer)
        .on_hover_change(move |app: &mut App, over, _| super::set_hovered(app, &hover_key, over))
        .on_press(move |app: &mut App| choose_cape(app, id.clone()))
        .child(thumb)
        .child(text(name.to_string()).font(theme.sans(10.0, 400.0)).color(if on { theme.accent } else { theme.text_secondary }).truncate().max_w(36.0))
}
