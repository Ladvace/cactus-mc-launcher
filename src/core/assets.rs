//! `assets/static`, embedded: what the Svelte app served from `/` (`/decor/sprites/00.png`, `/flags/…`).

use include_dir::{Dir, include_dir};

static STATIC: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/static");

pub const PIXELIFY: &[u8] = include_bytes!("../../assets/fonts/PixelifySans.ttf");
pub const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");

/// A static file by its web path, with or without the leading slash.
pub fn file(path: &str) -> Option<&'static [u8]> {
    STATIC.get_file(path.trim_start_matches('/')).map(|entry| entry.contents())
}
