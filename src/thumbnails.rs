//! Thumbnails of the tiles, for shells (dune-shell's minimap): every
//! `capture.thumbnail_interval` seconds dune draws each window that changed since, scaled
//! down to `SIZE` pixels on its longer side, into `$DUNE_THUMBNAILS/TILE.png`
//! (`$XDG_RUNTIME_DIR/dune-SOCKET.thumbs`). A file is written beside it and renamed, so a
//! reader never gets half of one. As in `events`, windows of locked desktops and private
//! ones get none, and a tile's file goes when the tile does (or turns private).

use std::{
    collections::HashMap,
    io::BufWriter,
    path::{Path, PathBuf},
    time::Duration,
};

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Bind, ExportMem, Offscreen,
            damage::OutputDamageTracker,
            element::{
                Kind,
                surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
            },
            gles::{GlesRenderer, GlesTexture},
            utils::{CommitCounter, RendererSurfaceStateUserData},
        },
    },
    desktop::Window,
    reexports::{
        calloop::{
            LoopHandle,
            timer::{TimeoutAction, Timer},
        },
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Physical, Point, Rectangle, Size, Transform},
    wayland::compositor::{TraversalAction, with_surface_tree_downward},
};

use crate::{Dune, layout::TileId};

/// Pixels on a thumbnail's longer side.
pub const SIZE: i32 = 360;

/// How often to look again while thumbnails are off.
const IDLE: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub struct Thumbnails {
    dir: Option<PathBuf>,
    /// What each tile's file shows: the commits of its window's surfaces, and its size.
    drawn: HashMap<TileId, (Vec<CommitCounter>, Size<i32, Logical>)>,
}

/// The folder for the compositor whose control socket is `socket`.
pub fn dir(socket: &Path) -> PathBuf {
    socket.with_extension("thumbs")
}

/// Start drawing thumbnails into `dir` (emptied of a previous run's).
pub fn watch(loop_handle: &LoopHandle<'static, Dune>, state: &mut Dune, dir: PathBuf) {
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(err) = std::fs::create_dir_all(&dir) {
        tracing::warn!("no thumbnails of the tiles: cannot make {dir:?}: {err}");
        return;
    }
    state.thumbnails.dir = Some(dir);
    let timer = loop_handle.insert_source(Timer::from_duration(IDLE), |_, _, state| match state.config.thumbnail_interval {
        0 => {
            state.clear_thumbnails();
            TimeoutAction::ToDuration(IDLE)
        }
        seconds => {
            state.draw_thumbnails();
            TimeoutAction::ToDuration(Duration::from_secs(seconds))
        }
    });
    if let Err(err) = timer {
        tracing::warn!("no thumbnails of the tiles: {err}");
    }
}

impl Dune {
    /// Draw the thumbnails out of date, and remove those of tiles no longer listed.
    pub fn draw_thumbnails(&mut self) {
        let Some(dir) = self.thumbnails.dir.clone() else { return };
        let listed: Vec<TileId> =
            self.window_list(false).into_iter().map(|w| w.tile).filter(|t| !self.privacy.tile(*t)).collect();
        let gone: Vec<TileId> = self.thumbnails.drawn.keys().filter(|t| !listed.contains(t)).copied().collect();
        for tile in gone {
            self.thumbnails.drawn.remove(&tile);
            let _ = std::fs::remove_file(file(&dir, tile));
        }
        for tile in listed {
            let Some(window) = self.tiles.get(&tile).and_then(|t| t.top()).cloned() else { continue };
            let Some(surface) = window.toplevel().map(|t| t.wl_surface().clone()) else { continue };
            let size = window.geometry().size;
            if size.w <= 0 || size.h <= 0 {
                continue;
            }
            let seen = (commits(&surface), size);
            if self.thumbnails.drawn.get(&tile) == Some(&seen) {
                continue;
            }
            match self.draw_thumbnail(&window, &surface).and_then(|(pixels, w, h)| write(&file(&dir, tile), &pixels, w, h)) {
                Ok(()) => {
                    self.thumbnails.drawn.insert(tile, seen);
                }
                Err(err) => tracing::debug!("thumbnail of tile {tile}: {err}"),
            }
        }
    }

    /// Thumbnails turned off: none are left behind.
    fn clear_thumbnails(&mut self) {
        let Some(dir) = self.thumbnails.dir.clone() else { return };
        for (tile, _) in self.thumbnails.drawn.drain() {
            let _ = std::fs::remove_file(file(&dir, tile));
        }
    }

    /// `window` scaled down to `SIZE`: its pixels (straight RGBA) and size.
    fn draw_thumbnail(&mut self, window: &Window, surface: &WlSurface) -> Result<(Vec<u8>, u32, u32), String> {
        let geometry = window.geometry();
        let (w, h, scale) = fit(geometry.size);
        // The window's own rectangle at the corner, without the shadows around it.
        let at = Point::<i32, Physical>::from((
            (-geometry.loc.x as f64 * scale).round() as i32,
            (-geometry.loc.y as f64 * scale).round() as i32,
        ));
        self.with_renderer(|_, renderer| {
            let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
                render_elements_from_surface_tree(renderer, surface, at, scale, 1.0, Kind::Unspecified);
            let size = Size::from((w, h));
            let mut texture: GlesTexture = renderer.create_buffer(Fourcc::Abgr8888, size).map_err(|e| e.to_string())?;
            // Elements are sized by the scale they are drawn at, not the one made with.
            let mut damage = OutputDamageTracker::new((w, h), scale, Transform::Normal);
            let mapping = {
                let mut target = renderer.bind(&mut texture).map_err(|e| e.to_string())?;
                damage.render_output(renderer, &mut target, 0, &elements, [0.0; 4]).map_err(|e| format!("{e:?}"))?;
                renderer.copy_framebuffer(&target, Rectangle::from_size(size), Fourcc::Abgr8888).map_err(|e| e.to_string())?
            };
            let mut pixels = renderer.map_texture(&mapping).map_err(|e| e.to_string())?.to_vec();
            pixels.truncate(w as usize * h as usize * 4);
            unpremultiply(&mut pixels);
            Ok((pixels, w as u32, h as u32))
        })
        .ok_or("no renderer")?
    }
}

fn file(dir: &Path, tile: TileId) -> PathBuf {
    dir.join(format!("{tile}.png"))
}

/// The thumbnail's size for a window of `size`, and the scale it is drawn at: `SIZE` on
/// the longer side, never larger than the window.
fn fit(size: Size<i32, Logical>) -> (i32, i32, f64) {
    let scale = (SIZE as f64 / size.w.max(size.h) as f64).min(1.0);
    let side = |n: i32| ((n as f64 * scale).round() as i32).max(1);
    (side(size.w), side(size.h), scale)
}

/// The commits of every surface of the tree at `surface`: a new one means new pixels.
fn commits(surface: &WlSurface) -> Vec<CommitCounter> {
    let mut commits = Vec::new();
    with_surface_tree_downward(
        surface,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        |_, states, _| {
            if let Some(state) = states.data_map.get::<RendererSurfaceStateUserData>() {
                commits.push(state.lock().unwrap().current_commit());
            }
        },
        |_, _, _| true,
    );
    commits
}

/// GL's premultiplied RGBA to the straight alpha of PNG.
fn unpremultiply(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// Write `pixels` as a PNG at `path`, whole or not at all.
fn write(path: &Path, pixels: &[u8], w: u32, h: u32) -> Result<(), String> {
    let partial = path.with_extension("png.part");
    let encode = || -> Result<(), png::EncodingError> {
        let mut encoder = png::Encoder::new(BufWriter::new(std::fs::File::create(&partial)?), w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixels)?;
        writer.finish()
    };
    encode().map_err(|e| e.to_string())?;
    std::fs::rename(&partial, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_fit_the_longer_side_and_never_grow() {
        assert_eq!(fit((1920, 1080).into()), (360, 203, 0.1875));
        assert_eq!(fit((500, 1000).into()), (180, 360, 0.36));
        assert_eq!(fit((200, 100).into()), (200, 100, 1.0));
        assert_eq!(fit((5000, 2).into()).1, 1);
    }

    #[test]
    fn alpha_is_taken_out_of_the_colors() {
        let mut px = [64, 32, 0, 128, 10, 20, 30, 255, 0, 0, 0, 0];
        unpremultiply(&mut px);
        assert_eq!(px, [128, 64, 0, 128, 10, 20, 30, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn a_thumbnail_is_a_png_once_written() {
        let dir = std::env::temp_dir().join(format!("dune-thumbs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = file(&dir, 7);
        write(&path, &[255, 0, 0, 255, 0, 255, 0, 128], 2, 1).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert!(!path.with_extension("png.part").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
