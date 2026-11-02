//! Tiles closing: when the last window of a tile goes, a snapshot of its last frame shrinks
//! towards the tile's middle and fades out, while the tiles left slide into its place
//! (`Dune::animate_resize`).
//!
//! The snapshot is taken when the toplevel is destroyed: its `wl_surface` (and buffer)
//! goes after it, so the window can still be drawn then.

use std::time::{Duration, Instant};

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            Bind, Offscreen,
            damage::OutputDamageTracker,
            element::{
                Kind,
                surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
                texture::{TextureBuffer, TextureRenderElement},
            },
            gles::{GlesRenderer, GlesTexture},
        },
    },
    desktop::Window,
    utils::{Buffer, Size, Transform},
};

use crate::{
    layout::{Rect, TileId},
    rounded::Radii,
    state::Dune,
    tiling::ease_out,
};


/// A tile on its way out: its last frame, where it was, and since when.
pub struct Closing {
    pub output: String,
    /// Where it was drawn (global), and the corners it had.
    pub rect: Rect,
    pub radii: Radii,
    pub texture: TextureBuffer<GlesTexture>,
    start: Instant,
    /// How long it takes to go (`animations.close_ms`), and how small it ends.
    duration: Duration,
    scale: f64,
}

impl Closing {
    /// Where it is drawn now (global) and how opaque; None once it is gone.
    pub fn now(&self) -> Option<(Rect, f32)> {
        let t = self.start.elapsed().as_secs_f64() / self.duration.as_secs_f64();
        (t < 1.0).then(|| shrink(self.rect, t, self.scale))
    }
}

/// `rect` at `t` (0 to 1) of closing: shrunk around its middle towards `end` of its size
/// and faded, eased out.
fn shrink(rect: Rect, t: f64, end: f64) -> (Rect, f32) {
    let p = ease_out(t);
    let scale = 1.0 - (1.0 - end) * p;
    let w = (rect.w as f64 * scale).round() as i32;
    let h = (rect.h as f64 * scale).round() as i32;
    (Rect::new(rect.x + (rect.w - w) / 2, rect.y + (rect.h - h) / 2, w, h), (1.0 - p) as f32)
}

impl Dune {
    /// `window`, the last of tile `id`, drawn into a texture the size of the tile as it is
    /// shown; None when the tile is not on screen (put away, scrolled off) or it cannot be.
    pub fn closing_snapshot(&mut self, id: TileId, window: &Window) -> Option<Closing> {
        // No animation: nothing to keep.
        let (duration, scale) = (self.config.animations.close, self.config.animations.close_scale);
        if duration.is_zero() {
            return None;
        }
        let output_name = self.output_of(id)?;
        let output = self.space.outputs().find(|o| o.name() == output_name)?.clone();
        let loc = self.space.element_location(window)? - window.geometry().loc;
        let rect = match self.is_floating(id) {
            true => {
                let size = window.geometry().size;
                Rect::new(loc.x + window.geometry().loc.x, loc.y + window.geometry().loc.y, size.w, size.h)
            }
            false => self.shown_rect(id, &output)?,
        };
        if rect.w <= 0 || rect.h <= 0 {
            return None;
        }
        let outer = self.with_tab_bar(rect);
        let mut radii = self.radii(id, outer);
        // Under a tab bar, the bar has the top corners.
        if outer.y < rect.y {
            radii[0] = 0;
            radii[1] = 0;
        }
        let surface = window.toplevel()?.wl_surface().clone();
        let texture = self
            .with_renderer(|_, renderer| -> Option<TextureBuffer<GlesTexture>> {
                let at = (loc.x - rect.x, loc.y - rect.y);
                let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
                    render_elements_from_surface_tree(renderer, &surface, at, 1.0, 1.0, Kind::Unspecified);
                let size: Size<i32, Buffer> = (rect.w, rect.h).into();
                let mut texture: GlesTexture = renderer.create_buffer(Fourcc::Abgr8888, size).ok()?;
                {
                    let mut target = renderer.bind(&mut texture).ok()?;
                    let mut damage = OutputDamageTracker::new((rect.w, rect.h), 1.0, Transform::Normal);
                    damage.render_output(renderer, &mut target, 0, &elements, [0.0; 4]).ok()?;
                }
                Some(TextureBuffer::from_texture(renderer, texture, 1, Transform::Normal, None))
            })
            .flatten()?;
        Some(Closing { output: output_name, rect, radii, texture, start: Instant::now(), duration, scale })
    }

    /// The closing tiles of `output`, front first: each one's element, where it is
    /// (output-local), its corners and the size of its texture. The finished ones are dropped.
    #[allow(clippy::type_complexity)]
    pub fn closing_elements(
        &mut self,
        origin: (i32, i32),
        output: &str,
    ) -> Vec<(TextureRenderElement<GlesTexture>, Rect, Radii, Size<i32, Buffer>)> {
        self.closing.retain(|c| c.now().is_some());
        self.closing
            .iter()
            .rev()
            .filter(|c| c.output == output)
            .filter_map(|c| {
                let (rect, alpha) = c.now()?;
                let local = Rect::new(rect.x - origin.0, rect.y - origin.1, rect.w, rect.h);
                let element = TextureRenderElement::from_texture_buffer(
                    (local.x as f64, local.y as f64),
                    &c.texture,
                    Some(alpha),
                    None,
                    Some((local.w, local.h).into()),
                    Kind::Unspecified,
                );
                // The corners shrink along with the tile.
                let scale = rect.w as f64 / c.rect.w as f64;
                let radii = c.radii.map(|r| (r as f64 * scale).round() as i32);
                Some((element, local, radii, (c.rect.w, c.rect.h).into()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closing_tile_shrinks_to_its_middle_and_fades() {
        let rect = Rect::new(100, 50, 400, 200);
        assert_eq!(shrink(rect, 0.0, 0.85), (rect, 1.0));
        let (end, alpha) = shrink(rect, 1.0, 0.85);
        assert_eq!(end, Rect::new(130, 65, 340, 170));
        assert_eq!(alpha, 0.0);
        // The same middle all the way (to the pixel an odd size rounds by).
        let (half, alpha) = shrink(rect, 0.5, 0.85);
        let ((x, y), (cx, cy)) = (half.center(), rect.center());
        assert!((x - cx).abs() <= 1 && (y - cy).abs() <= 1, "{:?}", half);
        assert!(half.w < rect.w && half.w > end.w);
        // Eased out: more than half faded at half the time.
        assert!(alpha < 0.5);
    }
}
