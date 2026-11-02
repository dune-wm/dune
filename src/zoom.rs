//! The last-mile zoom: a key magnifies the tile under the pointer around the pointer (the
//! whole monitor when it is over none), for the last pixels of an eye tracker's or a shaky
//! hand's pointing. The rest of the monitor stays as it is. Only what is drawn changes:
//! the pointer and the clicks stay in real coordinates, nothing in input is translated.
//! Subscribers hear both: `dune events zoom` gives the center, the area and the scale, and
//! a click while zoomed also says where it was seen (`zoom_x`, `zoom_y`), which is where
//! the eyes were. The same key again clicks where the pointer is; any button press (the
//! mouse, `dune pointer click`, a voice) goes back out, `zoom_out` without a click.

use dune_common::protocol::{MouseButton, PointerOp};
use smithay::utils::{Logical, Physical, Point};

use crate::{Dune, layout::Rect, layout::TileId};

/// How much the zoom magnifies: a webcam tracker's ~200 px miss is ~65 px of the screen.
pub const ZOOM: u32 = 3;

/// The zoom on one monitor: around `center`, drawn inside `area` (the tile's rectangle,
/// else the whole monitor), both from the monitor's corner, logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Zoom {
    pub output: String,
    pub tile: Option<TileId>,
    pub center: Point<f64, Logical>,
    pub area: Rect,
    pub scale: u32,
}

impl Zoom {
    /// Where the real point `p` (monitor coordinates) is seen while zoomed.
    pub fn seen(&self, p: Point<f64, Logical>) -> Point<f64, Logical> {
        let z = self.scale as f64;
        self.center + (p - self.center).upscale(z)
    }
}

/// Where a `RescaleRenderElement` of scale `output_scale · zoom` has its origin for
/// `center` to stay where it is: it draws g at o + (g − o)·S, and that is
/// s·(C + (g − C)·z) for o = s·C·(1 − z) / (1 − s·z).
pub fn origin(center: Point<f64, Logical>, output_scale: f64, zoom: f64) -> Point<i32, Physical> {
    let k = output_scale * (1.0 - zoom) / (1.0 - output_scale * zoom);
    Point::from(((center.x * k).round() as i32, (center.y * k).round() as i32))
}

impl Dune {
    /// The `zoom` key: in around the pointer, on the tile under it; zoomed already, a click
    /// where it is (which goes back out).
    pub fn zoom_key(&mut self) {
        if self.zoom.is_some() {
            self.pointer_request(PointerOp::Click(MouseButton::Left));
            return;
        }
        let here = self.seat.get_pointer().unwrap().current_location();
        let Some(output) = self.space.outputs().find(|o| self.space.output_geometry(o).is_some_and(|g| g.to_f64().contains(here))).cloned() else {
            return;
        };
        let Some(geo) = self.space.output_geometry(&output) else { return };
        let tile = self.tab_at(here.x, here.y).or_else(|| self.tile_at(here.x, here.y));
        let area = tile
            .and_then(|t| self.shown.iter().find(|(id, _)| *id == t).map(|(_, r)| *r))
            .map(|r| Rect::new(r.x - geo.loc.x, r.y - geo.loc.y, r.w, r.h))
            .unwrap_or_else(|| Rect::new(0, 0, geo.size.w, geo.size.h));
        let zoom = Zoom { output: output.name(), tile, center: here - geo.loc.to_f64(), area, scale: ZOOM };
        self.tell_zoom(true, &zoom);
        self.zoom = Some(zoom);
    }

    /// Back to the monitor as it is (the `zoom_out` key, or a button pressed while in).
    pub fn zoom_out(&mut self) {
        if let Some(zoom) = self.zoom.take() {
            self.tell_zoom(false, &zoom);
        }
    }

    /// The zoom of `output`, if it is the zoomed one.
    pub fn zoom_of(&self, output: &str) -> Option<&Zoom> {
        self.zoom.as_ref().filter(|z| z.output == output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where a scene element at `g` (logical) is drawn (physical), as `RescaleRenderElement`
    /// draws it with that origin and scale s·z.
    fn drawn(g: f64, center: f64, s: f64, z: f64) -> f64 {
        let o = origin((center, center).into(), s, z).x as f64;
        o + (g - o) * s * z
    }

    #[test]
    fn the_center_stays_and_the_rest_grows_around_it() {
        assert_eq!(origin((800.0, 400.0).into(), 1.0, 3.0), Point::from((800, 400)));
        assert_eq!(drawn(800.0, 800.0, 1.0, 3.0), 800.0);
        assert_eq!(drawn(810.0, 800.0, 1.0, 3.0), 830.0); // 10 px off it: 30
        // on a monitor of scale 2: still where it was (2·C), 10 px off it 60 px off
        assert!((drawn(800.0, 800.0, 2.0, 3.0) - 1600.0).abs() < 3.0);
        assert!((drawn(810.0, 800.0, 2.0, 3.0) - drawn(800.0, 800.0, 2.0, 3.0) - 60.0).abs() < 1e-9);
    }

    #[test]
    fn a_real_point_is_seen_three_times_as_far_from_the_center() {
        let z = Zoom { output: "M".into(), tile: Some(3), center: (800.0, 400.0).into(), area: Rect::new(0, 0, 1600, 900), scale: 3 };
        assert_eq!(z.seen((800.0, 400.0).into()), Point::from((800.0, 400.0)));
        assert_eq!(z.seen((820.0, 390.0).into()), Point::from((860.0, 370.0)));
    }
}
