//! SkinViewer.svelte without WebGL: skinview3d's player (head, body, arms, legs, their outer
//! layer and the cape) as textured boxes, ray-cast per pixel on the CPU into a `rune::Pixels`.
//! Each face is a parallelogram, so the ray/plane solve gives exact perspective-correct texel
//! coordinates and depth; a z-buffer sorts them. The idle sway and auto-rotation are
//! skinview3d's `IdleAnimation` and `autoRotateSpeed = 0.5`; drags orbit like its OrbitControls.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn scale(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// Row-major 3×3.
#[derive(Clone, Copy, Debug)]
struct M3([f32; 9]);

impl M3 {
    const I: M3 = M3([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    fn rx(a: f32) -> M3 {
        let (s, c) = a.sin_cos();
        M3([1.0, 0.0, 0.0, 0.0, c, -s, 0.0, s, c])
    }
    fn ry(a: f32) -> M3 {
        let (s, c) = a.sin_cos();
        M3([c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c])
    }
    fn rz(a: f32) -> M3 {
        let (s, c) = a.sin_cos();
        M3([c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0])
    }
    fn mul(self, o: M3) -> M3 {
        let (a, b) = (self.0, o.0);
        let mut r = [0.0; 9];
        for i in 0..3 {
            for j in 0..3 {
                r[i * 3 + j] = a[i * 3] * b[j] + a[i * 3 + 1] * b[3 + j] + a[i * 3 + 2] * b[6 + j];
            }
        }
        M3(r)
    }
    fn apply(self, v: V3) -> V3 {
        let m = self.0;
        [
            m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
            m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
            m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Model {
    #[default]
    Classic,
    Slim,
}

/// A skin or cape sheet as straight-alpha RGBA. Skins are square (legacy 64×32 ones are
/// converted); `unit` is texels per 1/64 of the width, so HD skins map the same.
#[derive(Clone, Debug)]
pub struct Texture {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

impl Texture {
    fn unit(&self) -> f32 {
        self.w as f32 / 64.0
    }

    fn px(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.w + x) * 4;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// A skin PNG's pixels, as skinview3d's `loadSkinToCanvas`: 64×32 sheets grow to 64×64 with
    /// the left arm and leg mirrored from the right, and a hat area with no transparency at all
    /// (old skins painted it black) is cleared.
    pub fn skin(w: u32, h: u32, rgba: Vec<u8>) -> Option<Texture> {
        let (w, h) = (w as usize, h as usize);
        if w == 0 || w % 64 != 0 || rgba.len() != w * h * 4 {
            return None;
        }
        let legacy = h * 2 == w;
        if !legacy && h != w {
            return None;
        }
        let mut texture = Texture { w, h: w, rgba: vec![0; w * w * 4] };
        texture.rgba[..w * h * 4].copy_from_slice(&rgba);
        let unit = w / 64;
        if legacy {
            const COPIES: [(usize, usize, usize, usize, usize, usize); 12] = [
                (4, 16, 4, 4, 20, 48),
                (8, 16, 4, 4, 24, 48),
                (0, 20, 4, 12, 24, 52),
                (4, 20, 4, 12, 20, 52),
                (8, 20, 4, 12, 16, 52),
                (12, 20, 4, 12, 28, 52),
                (44, 16, 4, 4, 36, 48),
                (48, 16, 4, 4, 40, 48),
                (40, 20, 4, 12, 40, 52),
                (44, 20, 4, 12, 36, 52),
                (48, 20, 4, 12, 32, 52),
                (52, 20, 4, 12, 44, 52),
            ];
            for (sx, sy, cw, ch, dx, dy) in COPIES {
                for y in 0..ch * unit {
                    for x in 0..cw * unit {
                        let pixel = texture.px(sx * unit + x, sy * unit + y);
                        let (tx, ty) = (dx * unit + cw * unit - 1 - x, dy * unit + y);
                        let i = (ty * w + tx) * 4;
                        texture.rgba[i..i + 4].copy_from_slice(&pixel);
                    }
                }
            }
        }
        let top_opaque = (0..w * w / 2).all(|i| texture.rgba[i * 4 + 3] == 255);
        if top_opaque {
            for y in 0..16 * unit {
                for x in 32 * unit..64 * unit {
                    texture.rgba[(y * w + x) * 4 + 3] = 0;
                }
            }
        }
        Some(texture)
    }

    /// A cape sheet (64×32, or a multiple).
    pub fn cape(w: u32, h: u32, rgba: Vec<u8>) -> Option<Texture> {
        let (w, h) = (w as usize, h as usize);
        (w >= 22 && h > 0 && rgba.len() == w * h * 4).then_some(Texture { w, h, rgba })
    }
}

/// Where a face takes its texels from, in 1/64 units of the sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One box face: `origin` is the 3D corner the region's top-left texel sits on, `es` runs
/// along the region's width and `et` down its height.
#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub origin: V3,
    pub es: V3,
    pub et: V3,
    pub region: Region,
}

/// Minecraft's box unwrap for a box at `(u, v)` with texture size `(w, h, d)`, on geometry
/// from `min` to `max`. The model faces +z, +y is up and the player's right is −x.
pub fn box_faces(min: V3, max: V3, uv: (f32, f32), dims: (f32, f32, f32)) -> [Face; 6] {
    let (u, v) = uv;
    let (w, h, d) = dims;
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let (gw, gh, gd) = (x1 - x0, y1 - y0, z1 - z0);
    let r = |x, y, w, h| Region { x, y, w, h };
    [
        // front (+z)
        Face { origin: [x0, y1, z1], es: [gw, 0.0, 0.0], et: [0.0, -gh, 0.0], region: r(u + d, v + d, w, h) },
        // back (−z)
        Face { origin: [x1, y1, z0], es: [-gw, 0.0, 0.0], et: [0.0, -gh, 0.0], region: r(u + 2.0 * d + w, v + d, w, h) },
        // the player's right (−x)
        Face { origin: [x0, y1, z0], es: [0.0, 0.0, gd], et: [0.0, -gh, 0.0], region: r(u, v + d, d, h) },
        // the player's left (+x)
        Face { origin: [x1, y1, z1], es: [0.0, 0.0, -gd], et: [0.0, -gh, 0.0], region: r(u + d + w, v + d, d, h) },
        // top
        Face { origin: [x0, y1, z0], es: [gw, 0.0, 0.0], et: [0.0, 0.0, gd], region: r(u + d, v, w, d) },
        // bottom
        Face { origin: [x0, y0, z1], es: [gw, 0.0, 0.0], et: [0.0, 0.0, -gd], region: r(u + d + w, v, w, d) },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    Skin,
    Cape,
}

/// A box hung from `pivot`, turned by `rot` (x, y, z radians, applied z, then y, then x).
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub name: &'static str,
    pub pivot: V3,
    pub rot: V3,
    pub min: V3,
    pub size: V3,
    pub uv: (f32, f32),
    pub dims: (f32, f32, f32),
    /// The outer layer: inflated, double-sided, transparent texels skipped.
    pub overlay: bool,
    pub sheet: Sheet,
}

impl Part {
    fn matrix(&self) -> M3 {
        M3::rx(self.rot[0]).mul(M3::ry(self.rot[1])).mul(M3::rz(self.rot[2]))
    }
}

/// skinview3d's `PlayerObject` at time `t` seconds of `IdleAnimation`, feet at y = 0.
pub fn parts(model: Model, t: f32, cape: bool) -> Vec<Part> {
    let aw = if model == Model::Slim { 3.0 } else { 4.0 };
    let t = t * 2.0;
    let arm_l = (t.cos() * 0.03 + PI * 0.02, 0.0);
    let arm_r = ((t + PI).cos() * 0.03 - PI * 0.02, 0.0);
    let cape_x = t.sin() * 0.01 + PI * 0.06;
    let mut out = Vec::with_capacity(13);
    let mut pair = |name: &'static str, pivot: V3, rot: V3, min: V3, size: V3, base: (f32, f32), layer: (f32, f32), dims: (f32, f32, f32), infl: f32| {
        out.push(Part { name, pivot, rot, min, size, uv: base, dims, overlay: false, sheet: Sheet::Skin });
        out.push(Part {
            name,
            pivot,
            rot,
            min: [min[0] - infl, min[1] - infl, min[2] - infl],
            size: [size[0] + 2.0 * infl, size[1] + 2.0 * infl, size[2] + 2.0 * infl],
            uv: layer,
            dims,
            overlay: true,
            sheet: Sheet::Skin,
        });
    };
    pair("head", [0.0, 24.0, 0.0], [0.0; 3], [-4.0, 0.0, -4.0], [8.0, 8.0, 8.0], (0.0, 0.0), (32.0, 0.0), (8.0, 8.0, 8.0), 0.5);
    pair("body", [0.0, 12.0, 0.0], [0.0; 3], [-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], (16.0, 16.0), (16.0, 32.0), (8.0, 12.0, 4.0), 0.25);
    let arm_min = [-aw / 2.0, -10.0, -2.0];
    let arm_size = [aw, 12.0, 4.0];
    pair("rightArm", [-(4.0 + aw / 2.0), 22.0, 0.0], [arm_r.1, 0.0, arm_r.0], arm_min, arm_size, (40.0, 16.0), (40.0, 32.0), (aw, 12.0, 4.0), 0.25);
    pair("leftArm", [4.0 + aw / 2.0, 22.0, 0.0], [arm_l.1, 0.0, arm_l.0], arm_min, arm_size, (32.0, 48.0), (48.0, 48.0), (aw, 12.0, 4.0), 0.25);
    let leg_min = [-2.0, -12.0, -2.0];
    let leg_size = [4.0, 12.0, 4.0];
    pair("rightLeg", [-2.0, 12.0, 0.0], [0.0; 3], leg_min, leg_size, (0.0, 16.0), (0.0, 32.0), (4.0, 12.0, 4.0), 0.25);
    pair("leftLeg", [2.0, 12.0, 0.0], [0.0; 3], leg_min, leg_size, (16.0, 48.0), (0.0, 48.0), (4.0, 12.0, 4.0), 0.25);
    if cape {
        out.push(Part {
            name: "cape",
            pivot: [0.0, 24.0, -3.0],
            rot: [cape_x, PI, 0.0],
            min: [-5.0, -16.0, -0.5],
            size: [10.0, 16.0, 1.0],
            uv: (0.0, 0.0),
            dims: (10.0, 16.0, 1.0),
            overlay: false,
            sheet: Sheet::Cape,
        });
    }
    out
}

/// The camera: `yaw` turns the player (auto-rotation plus drags), `pitch` orbits above or
/// below. Field of view and distance are skinview3d's (`fov` 50, `zoom` 0.9).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub yaw: f32,
    pub pitch: f32,
    /// Animation time in seconds.
    pub time: f32,
}

pub const FOV: f32 = 50.0 * PI / 180.0;
pub fn camera_distance() -> f32 {
    4.5 + 16.5 / (FOV / 2.0).tan() / 0.9
}

/// The model's centre: the player stands 32 tall, centred in view.
const CENTER: V3 = [0.0, 16.0, 0.0];

/// Reusable buffers; `render` fills `rgba` (straight alpha, transparent background).
#[derive(Default)]
pub struct Raster {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
    depth: Vec<f32>,
}

/// A face in camera space, ready to scan.
struct Prepared {
    origin: V3,
    es: V3,
    et: V3,
}

impl Raster {
    pub fn render(&mut self, w: usize, h: usize, skin: &Texture, cape: Option<&Texture>, model: Model, pose: Pose) {
        self.w = w;
        self.h = h;
        self.rgba.clear();
        self.rgba.resize(w * h * 4, 0);
        self.depth.clear();
        self.depth.resize(w * h, f32::INFINITY);
        if w == 0 || h == 0 {
            return;
        }
        let view = M3::rx(pose.pitch).mul(M3::ry(pose.yaw));
        let dist = camera_distance();
        let f = (h as f32 / 2.0) / (FOV / 2.0).tan();
        // Light from above, in front and a little to the left, fixed to the camera.
        let light = {
            let dir = [-0.35, 0.55, 0.75];
            let n = dot(dir, dir).sqrt();
            scale(dir, 1.0 / n)
        };
        for part in parts(model, pose.time, cape.is_some()) {
            let tex = match part.sheet {
                Sheet::Skin => skin,
                Sheet::Cape => match cape {
                    Some(cape) => cape,
                    None => continue,
                },
            };
            let m = view.mul(part.matrix());
            let pivot = view.apply(sub(part.pivot, CENTER));
            let max = add(part.min, part.size);
            for face in box_faces(part.min, max, part.uv, part.dims) {
                let mut origin = add(m.apply(face.origin), pivot);
                origin[2] -= dist;
                let prepared = Prepared { origin, es: m.apply(face.es), et: m.apply(face.et) };
                self.scan(&prepared, face.region, tex, part.overlay, f, light);
            }
        }
    }

    fn scan(&mut self, p: &Prepared, region: Region, tex: &Texture, overlay: bool, f: f32, light: V3) {
        let (w, h) = (self.w, self.h);
        let mut n = cross(p.et, p.es);
        let facing = -dot(n, p.origin);
        if facing <= 0.0 {
            if !overlay {
                return;
            }
            // Double-sided outer layer: seen from inside, the normal faces us.
            n = scale(n, -1.0);
        }
        let nl = dot(n, n).sqrt();
        if nl == 0.0 {
            return;
        }
        let shade = {
            let unit = scale(n, 1.0 / nl);
            (0.62 + 0.42 * dot(unit, light).max(0.0)).min(1.0)
        };
        // A is dual to es and B to et within the face plane.
        let a = cross(p.et, n);
        let b = cross(n, p.es);
        let (ka, kb) = (1.0 / dot(p.es, a), 1.0 / dot(p.et, b));
        let a = scale(a, ka);
        let b = scale(b, kb);
        let (pn, pa, pb) = (dot(p.origin, n), dot(p.origin, a), dot(p.origin, b));

        // Screen bounds of the four corners.
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let corners = [p.origin, add(p.origin, p.es), add(p.origin, p.et), add(add(p.origin, p.es), p.et)];
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for c in corners {
            if c[2] >= -0.01 {
                return;
            }
            let sx = cx + f * c[0] / -c[2];
            let sy = cy - f * c[1] / -c[2];
            x0 = x0.min(sx);
            x1 = x1.max(sx);
            y0 = y0.min(sy);
            y1 = y1.max(sy);
        }
        let xa = (x0.floor().max(0.0)) as usize;
        let xb = (x1.ceil().min(w as f32)) as usize;
        let ya = (y0.floor().max(0.0)) as usize;
        let yb = (y1.ceil().min(h as f32)) as usize;
        if xa >= xb || ya >= yb {
            return;
        }

        let unit = tex.unit();
        let (rx, ry) = (region.x * unit, region.y * unit);
        let (rw, rh) = (region.w * unit, region.h * unit);
        let (tx_max, ty_max) = (rw.max(1.0) - 1.0, rh.max(1.0) - 1.0);
        let (rxi, ryi) = (rx as usize, ry as usize);
        if rxi + (rw as usize) > tex.w || ryi + (rh as usize) > tex.h {
            return;
        }
        let shade_i = (shade * 256.0) as u32;

        // The ray through pixel (x, y) is r = ((x + .5 - cx) / f, -(y + .5 - cy) / f, -1), so
        // r·v is linear in x: v.x/f per pixel, plus a per-row base.
        let inv_f = 1.0 / f;
        for y in ya..yb {
            let ry_ = -((y as f32 + 0.5 - cy) * inv_f);
            let rx0 = (xa as f32 + 0.5 - cx) * inv_f;
            let mut dn = n[0] * rx0 + n[1] * ry_ - n[2];
            let mut da = a[0] * rx0 + a[1] * ry_ - a[2];
            let mut db = b[0] * rx0 + b[1] * ry_ - b[2];
            let (sn, sa, sb) = (n[0] * inv_f, a[0] * inv_f, b[0] * inv_f);
            let row = y * w;
            for x in xa..xb {
                let (cdn, cda, cdb) = (dn, da, db);
                dn += sn;
                da += sa;
                db += sb;
                if cdn.abs() < 1e-9 {
                    continue;
                }
                let lam = pn / cdn;
                if lam <= 0.0 {
                    continue;
                }
                let s = lam * cda - pa;
                let t = lam * cdb - pb;
                if !(0.0..1.0).contains(&s) || !(0.0..1.0).contains(&t) {
                    continue;
                }
                let i = row + x;
                if lam >= self.depth[i] {
                    continue;
                }
                let tx = rxi + (s * rw).min(tx_max) as usize;
                let ty = ryi + (t * rh).min(ty_max) as usize;
                let ti = (ty * tex.w + tx) * 4;
                let alpha = tex.rgba[ti + 3];
                if overlay && alpha < 128 {
                    continue;
                }
                self.depth[i] = lam;
                let o = i * 4;
                self.rgba[o] = ((tex.rgba[ti] as u32 * shade_i) >> 8).min(255) as u8;
                self.rgba[o + 1] = ((tex.rgba[ti + 1] as u32 * shade_i) >> 8).min(255) as u8;
                self.rgba[o + 2] = ((tex.rgba[ti + 2] as u32 * shade_i) >> 8).min(255) as u8;
                self.rgba[o + 3] = 255;
            }
        }
    }
}

/// What the render thread draws; the UI side edits it.
#[derive(Clone, Default)]
pub struct Scene {
    pub skin: Option<Arc<Texture>>,
    pub cape: Option<Arc<Texture>>,
    pub model: Model,
    /// Drag rotation, added to the auto-rotation.
    pub yaw: f32,
    pub pitch: f32,
    /// Device pixels.
    pub size: (usize, usize),
    pub animate: bool,
}

/// skinview3d's `autoRotateSpeed`, radians per second.
pub const AUTO_ROTATE: f32 = 0.5;

/// A live `SkinViewer`: a `Pixels` image and the thread that keeps redrawing it at ~30 fps
/// while it's running. Cheap to clone; every clone drives the same image.
#[derive(Clone)]
pub struct Viewer {
    pub pixels: rune::Pixels,
    scene: Arc<Mutex<Scene>>,
    running: Arc<AtomicBool>,
    start: Arc<Mutex<Instant>>,
}

impl Default for Viewer {
    fn default() -> Self {
        Viewer {
            pixels: rune::Pixels::new(),
            scene: Arc::default(),
            running: Arc::default(),
            start: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

impl Viewer {
    pub fn scene(&self) -> Scene {
        self.scene.lock().unwrap().clone()
    }

    /// Change the scene and draw it once now, at animation time `at` (None: the clock's).
    pub fn update(&self, f: impl FnOnce(&mut Scene), at: Option<f32>) {
        let scene = {
            let mut locked = self.scene.lock().unwrap();
            f(&mut locked);
            locked.clone()
        };
        let t = at.unwrap_or_else(|| self.start.lock().unwrap().elapsed().as_secs_f32());
        draw(&self.pixels, &scene, t, &mut Raster::default());
    }

    /// Draw at `size` device pixels from now on; redraws when it changed.
    pub fn resize(&self, size: (usize, usize)) {
        if self.scene.lock().unwrap().size != size {
            self.update(|scene| scene.size = size, None);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start the redraw thread, if it isn't going.
    pub fn run(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        let spawned = std::thread::Builder::new().name("skin-viewer".into()).spawn(move || {
            let mut raster = Raster::default();
            while me.running.load(Ordering::Relaxed) {
                let frame = Instant::now();
                let scene = me.scene.lock().unwrap().clone();
                if !scene.animate {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                let t = me.start.lock().unwrap().elapsed().as_secs_f32();
                draw(&me.pixels, &scene, t, &mut raster);
                std::thread::sleep(Duration::from_millis(33).saturating_sub(frame.elapsed()));
            }
        });
        if spawned.is_err() {
            self.running.store(false, Ordering::SeqCst);
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn draw(pixels: &rune::Pixels, scene: &Scene, t: f32, raster: &mut Raster) {
    let Some(skin) = &scene.skin else { return };
    let (w, h) = scene.size;
    if w == 0 || h == 0 {
        return;
    }
    let yaw = if scene.animate { scene.yaw + t * AUTO_ROTATE } else { scene.yaw };
    let pose = Pose { yaw, pitch: scene.pitch, time: if scene.animate { t } else { 0.0 } };
    raster.render(w, h, skin, scene.cape.as_deref(), scene.model, pose);
    pixels.set(w as u32, h as u32, raster.rgba.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet where every face region of every base and layer box is one flat colour, keyed
    /// by region position, so a pixel's colour says which region it sampled.
    pub(super) fn tagged_sheet() -> Texture {
        let mut sheet = Texture { w: 64, h: 64, rgba: vec![0; 64 * 64 * 4] };
        for part in parts(Model::Classic, 0.0, false) {
            let max = add(part.min, part.size);
            for face in box_faces(part.min, max, part.uv, part.dims) {
                let region = face.region;
                for y in region.y as usize..(region.y + region.h) as usize {
                    for x in region.x as usize..(region.x + region.w) as usize {
                        let i = (y * 64 + x) * 4;
                        let alpha = if part.overlay { 0 } else { 255 };
                        sheet.rgba[i..i + 4].copy_from_slice(&[region.x as u8 * 4, region.y as u8 * 4, 255, alpha]);
                    }
                }
            }
        }
        sheet
    }

    fn sample(raster: &Raster, x: usize, y: usize) -> [u8; 4] {
        let i = (y * raster.w + x) * 4;
        [raster.rgba[i], raster.rgba[i + 1], raster.rgba[i + 2], raster.rgba[i + 3]]
    }

    /// Which region (x, y) the pixel shows, undoing the shading by its blue tag of 7.
    pub(super) fn region_at(raster: &Raster, x: usize, y: usize) -> Option<(u8, u8)> {
        let pixel = sample(raster, x, y);
        if pixel[3] == 0 {
            return None;
        }
        let unshade = 255.0 / pixel[2].max(1) as f32;
        Some((((pixel[0] as f32 * unshade) / 4.0).round() as u8, ((pixel[1] as f32 * unshade) / 4.0).round() as u8))
    }

    #[test]
    fn box_unwrap_matches_minecraft() {
        let faces = box_faces([-4.0, 0.0, -4.0], [4.0, 8.0, 4.0], (0.0, 0.0), (8.0, 8.0, 8.0));
        let at = |i: usize| (faces[i].region.x, faces[i].region.y, faces[i].region.w, faces[i].region.h);
        assert_eq!(at(0), (8.0, 8.0, 8.0, 8.0)); // front
        assert_eq!(at(1), (24.0, 8.0, 8.0, 8.0)); // back
        assert_eq!(at(2), (0.0, 8.0, 8.0, 8.0)); // right
        assert_eq!(at(3), (16.0, 8.0, 8.0, 8.0)); // left
        assert_eq!(at(4), (8.0, 0.0, 8.0, 8.0)); // top
        assert_eq!(at(5), (16.0, 0.0, 8.0, 8.0)); // bottom
        // The front's top-left texel is the player's top right (viewer's top left), toward us.
        assert_eq!(faces[0].origin, [-4.0, 8.0, 4.0]);
        // Every face's es × et points into the box, so et × es is its outward normal.
        let normals = [[0.0, 0.0, 1.0], [0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
        for (face, want) in faces.iter().zip(normals) {
            let n = cross(face.et, face.es);
            let len = dot(n, n).sqrt();
            assert_eq!(scale(n, 1.0 / len), want);
        }
        // The right face's texture runs back to front, the left's front to back; the top
        // starts at the back edge and the bottom at the front, as Minecraft draws them.
        assert!(faces[2].es[2] > 0.0 && faces[3].es[2] < 0.0);
        assert_eq!(faces[4].origin[2], -4.0);
        assert_eq!(faces[5].origin[2], 4.0);
    }

    #[test]
    fn slim_arms_are_three_wide() {
        let slim = parts(Model::Slim, 0.0, false);
        let arm = slim.iter().find(|part| part.name == "leftArm" && !part.overlay).unwrap();
        assert_eq!(arm.size[0], 3.0);
        assert_eq!(arm.dims.0, 3.0);
        assert_eq!(arm.pivot[0], 5.5);
        let classic = parts(Model::Classic, 0.0, false);
        let arm = classic.iter().find(|part| part.name == "rightArm" && !part.overlay).unwrap();
        assert_eq!((arm.size[0], arm.pivot[0]), (4.0, -6.0));
        // Shoulders sit at the top of the body (y 24), the arm reaching down to y 12.
        assert_eq!(arm.pivot[1] + arm.min[1], 12.0);
        assert_eq!(arm.pivot[1] + arm.min[1] + arm.size[1], 24.0);
    }

    #[test]
    fn front_back_and_top_views_sample_the_right_regions() {
        let sheet = tagged_sheet();
        let mut raster = Raster::default();
        let (w, h) = (170, 250);
        let center = |raster: &Raster, y: f32| -> Option<(u8, u8)> {
            // Model y (0 feet … 32 top) to a screen row at the model's depth.
            let f = (h as f32 / 2.0) / (FOV / 2.0).tan();
            let row = h as f32 / 2.0 - f * (y - 16.0) / camera_distance();
            region_at(raster, w / 2, row as usize)
        };
        raster.render(w, h, &sheet, None, Model::Classic, Pose::default());
        // The face is nearer than the head's centre plane, so aim at the head's middle.
        assert_eq!(center(&raster, 28.0), Some((8, 8)), "head front");
        assert_eq!(center(&raster, 18.0), Some((20, 20)), "body front");
        // Viewer's left half of the legs is the player's right leg.
        let f = (h as f32 / 2.0) / (FOV / 2.0).tan();
        let row = (h as f32 / 2.0 - f * (6.0 - 16.0) / camera_distance()) as usize;
        let col = (w as f32 / 2.0 - f * 2.0 / camera_distance()) as usize;
        assert_eq!(region_at(&raster, col, row), Some((4, 20)), "right leg front");

        raster.render(w, h, &sheet, None, Model::Classic, Pose { yaw: PI, ..Pose::default() });
        assert_eq!(center(&raster, 28.0), Some((24, 8)), "head back");
        assert_eq!(center(&raster, 18.0), Some((32, 20)), "body back");

        raster.render(w, h, &sheet, None, Model::Classic, Pose { yaw: PI / 2.0, ..Pose::default() });
        // Turned a quarter left (front toward +x), we see the player's right side.
        assert_eq!(center(&raster, 28.0), Some((0, 8)), "head right side");

        raster.render(w, h, &sheet, None, Model::Classic, Pose { pitch: PI / 2.0 - 0.01, ..Pose::default() });
        assert_eq!(region_at(&raster, w / 2, h / 2), Some((8, 0)), "head top");
    }

    #[test]
    fn texels_land_where_minecraft_puts_them() {
        // One marked texel at the head front's top-left: it must show at the viewer's top left
        // of the face, and nowhere when seen from behind.
        let mut sheet = Texture { w: 64, h: 64, rgba: vec![255; 64 * 64 * 4] };
        let i = (8 * 64 + 8) * 4;
        sheet.rgba[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
        for y in 0..16 {
            for x in 32..64 {
                sheet.rgba[(y * 64 + x) * 4 + 3] = 0;
            }
        }
        let mut raster = Raster::default();
        raster.render(200, 300, &sheet, None, Model::Classic, Pose::default());
        let red: Vec<(usize, usize)> = (0..300)
            .flat_map(|y| (0..200).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let pixel = sample(&raster, x, y);
                pixel[3] == 255 && pixel[1] == 0 && pixel[0] > 100
            })
            .collect();
        assert!(!red.is_empty());
        let (mx, my) = red.iter().fold((0, 0), |sum, pos| (sum.0 + pos.0, sum.1 + pos.1));
        let (mx, my) = (mx / red.len(), my / red.len());
        assert!(mx < 100 && my < 150, "texel at {mx},{my}");
        raster.render(200, 300, &sheet, None, Model::Classic, Pose { yaw: PI, ..Pose::default() });
        assert!(!(0..200 * 300).any(|i| raster.rgba[i * 4 + 1] == 0 && raster.rgba[i * 4 + 3] == 255));
    }

    #[test]
    fn legacy_skins_grow_mirrored_left_limbs() {
        let mut rgba = vec![0u8; 64 * 32 * 4];
        // Right arm front (44,20) gets a marker at its first column.
        let i = (20 * 64 + 44) * 4;
        rgba[i..i + 4].copy_from_slice(&[1, 2, 3, 255]);
        let skin = Texture::skin(64, 32, rgba).unwrap();
        assert_eq!((skin.w, skin.h), (64, 64));
        // Left arm front is (36,52); mirrored, the marker lands at its last column.
        assert_eq!(skin.px(39, 52), [1, 2, 3, 255]);
        assert_eq!(skin.px(36, 52)[3], 0);
        assert!(Texture::skin(64, 48, vec![0; 64 * 48 * 4]).is_none());
    }

    #[test]
    fn opaque_hats_are_cleared() {
        let skin = Texture::skin(64, 64, vec![255; 64 * 64 * 4]).unwrap();
        assert_eq!(skin.px(40, 8)[3], 0);
        assert_eq!(skin.px(8, 8)[3], 255);
    }

    #[test]
    fn renders_fast_enough() {
        let sheet = tagged_sheet();
        let mut raster = Raster::default();
        let start = Instant::now();
        for i in 0..10 {
            raster.render(340, 500, &sheet, None, Model::Classic, Pose { yaw: i as f32 * 0.3, pitch: 0.2, time: i as f32 });
        }
        let per = start.elapsed() / 10;
        eprintln!("render 340x500: {per:?}");
        assert!(per < Duration::from_millis(if cfg!(debug_assertions) { 200 } else { 8 }));
    }

    /// `SKIN_SNAP=<skin.png>` `SKIN_SNAP_OUT=<dir>`: writes views of a real skin from a few
    /// angles, for looking at.
    #[test]
    #[ignore]
    fn snapshot_angles() {
        let src = std::env::var("SKIN_SNAP").expect("SKIN_SNAP");
        let out = std::path::PathBuf::from(std::env::var("SKIN_SNAP_OUT").expect("SKIN_SNAP_OUT"));
        let model = if std::env::var("SKIN_SLIM").is_ok() { Model::Slim } else { Model::Classic };
        let img = image::open(&src).unwrap().to_rgba8();
        let (w, h) = img.dimensions();
        let skin = Texture::skin(w, h, img.into_raw()).unwrap();
        // `SKIN_CAPE=test` draws a generated cape: a red outer face with a gold border.
        let cape = std::env::var("SKIN_CAPE").ok().map(|spec| {
            if spec == "test" {
                let mut rgba = vec![0u8; 64 * 32 * 4];
                for y in 0..17 {
                    for x in 0..22 {
                        let outer = (1..11).contains(&x) && (1..17).contains(&y);
                        let edge = outer && (x == 1 || x == 10 || y == 1 || y == 16);
                        let color = if edge { [232, 178, 58, 255] } else if outer { [180, 30, 40, 255] } else { [90, 20, 25, 255] };
                        rgba[(y * 64 + x) * 4..][..4].copy_from_slice(&color);
                    }
                }
                return Texture::cape(64, 32, rgba).unwrap();
            }
            let img = image::open(spec).unwrap().to_rgba8();
            let (w, h) = img.dimensions();
            Texture::cape(w, h, img.into_raw()).unwrap()
        });
        let stem = std::path::Path::new(&src).file_stem().unwrap().to_string_lossy().into_owned();
        let stem = if cape.is_some() { format!("{stem}-cape") } else { stem };
        let mut raster = Raster::default();
        let views = [("front", 0.0, 0.0, 0.0), ("three-quarter", 0.6, 0.25, 1.0), ("side", PI / 2.0, 0.0, 0.0), ("back", PI, 0.15, 0.5), ("above", -0.5, 0.9, 0.0)];
        let (vw, vh) = (340u32, 500u32);
        let mut strip = image::RgbaImage::from_pixel(vw * views.len() as u32, vh, image::Rgba([32, 30, 28, 255]));
        for (i, (name, yaw, pitch, time)) in views.into_iter().enumerate() {
            raster.render(vw as usize, vh as usize, &skin, cape.as_ref(), model, Pose { yaw, pitch, time });
            let img = image::RgbaImage::from_raw(vw, vh, raster.rgba.clone()).unwrap();
            img.save(out.join(format!("{stem}-{name}.png"))).unwrap();
            for (x, y, pixel) in img.enumerate_pixels() {
                if pixel[3] > 0 {
                    strip.put_pixel(i as u32 * vw + x, y, *pixel);
                }
            }
        }
        strip.save(out.join(format!("{stem}-strip.png"))).unwrap();
    }
}
