//! The last-mile zoom: a key magnifies the tile under the pointer (the whole monitor when
//! it is over none), for the last pixels of an eye tracker's or a shaky hand's pointing.
//! The magnified view is a lens: the tile less a frame of `DEAD_FRAME` px on each side, in
//! which the real tile shows (a dead zone for the eyes: they rest there without the lens
//! moving). What the pointer is on is seen in the middle of the lens, unless that would
//! pull a tile's edge more than `EDGE_IN` into it: by an edge the pointer is seen nearer
//! that side, the edge just `EDGE_IN` in from the border an eye tracker hardly reaches,
//! and past it the real tile shows through (no backdrop). A border rings the lens (`[zoom]`), and
//! what of the tile it does not magnify (the frame, the real tile past the edges) is shaded.
//! The lens stretches what the window already drew (no new scale for it, so it never
//! redraws mid-zoom): softer text, but nothing jumps; the lens sharpens it back some (`crate::rounded::Sharpened`). It grows in from the real size and back (`ANIMATION`), the
//! shade fading with it. The rest of the monitor stays as it is. Only what is drawn changes:
//! the pointer and the clicks stay in real coordinates, nothing in input is translated.
//! Subscribers hear both: `dune events zoom` gives the center, the area and the scale, and
//! a click while zoomed also says where it was seen (`zoom_x`, `zoom_y`), which is where
//! the eyes were. The same key again clicks where the pointer is, a plain click even with
//! Mod down (it never floats the tile), and so goes back out. Any button press (the mouse, the
//! right Ctrl+Shift tap, `dune pointer click`, a voice) goes back out, and so do
//! `zoom_out` and Mod tapped alone (`crate::mod_hold`), without a click.
//!
//! The right Ctrl and Shift tapped together alone (both let go, nothing else pressed)
//! click where the pointer is, zoomed or not: the hand on the keyboard when the mouse is
//! on the left.

use std::time::{Duration, Instant};

use dune_common::protocol::{MouseButton, PointerOp};
use smithay::{
    input::keyboard::{Keysym, ModifiersState},
    utils::{Logical, Physical, Point},
};

use crate::{Dune, layout::Rect, layout::TileId};

/// How much the zoom magnifies: a webcam tracker's ~200 px miss is ~65 px of the screen.
pub const ZOOM: u32 = 3;

/// The frame inside the tile left out of the lens, where the real tile shows: logical px.
pub const DEAD_FRAME: i32 = 50;

/// How far into the lens a tile's edge comes at most (logical px): off the border, where
/// the eyes hardly reach, without leaving half the lens unmagnified by an edge.
pub const EDGE_IN: i32 = DEAD_FRAME;

/// How long the lens takes to grow to `ZOOM` (and back to 1 on the way out), unless
/// `[zoom] animation_ms` says otherwise.
pub const ANIMATION: Duration = Duration::from_millis(300);

/// The zoom on one monitor: around `center` (the point that stays where it is, chosen so
/// the pointer is seen in the middle of `area`), drawn inside `area` (the lens: the tile's
/// rectangle, else the whole monitor, less the dead frame) and only as far as `bounds`
/// (that rectangle) magnified reaches, all from the monitor's corner, logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Zoom {
    pub output: String,
    pub tile: Option<TileId>,
    pub center: Point<f64, Logical>,
    pub area: Rect,
    pub bounds: Rect,
    pub scale: u32,
    /// When it went in (or, leaving, out): for the animation.
    pub since: Instant,
    /// How long it grows in and shrinks back (`[zoom] animation_ms`).
    pub duration: Duration,
}

impl Zoom {
    /// Where the real point `p` (monitor coordinates) is seen while zoomed.
    pub fn seen(&self, p: Point<f64, Logical>) -> Point<f64, Logical> {
        let z = self.scale as f64;
        self.center + (p - self.center).upscale(z)
    }

    /// Where the real point `p` is drawn while zoomed: in the lens when its magnified
    /// place falls there (and within the tile magnified), else the real `p` itself.
    pub fn drawn(&self, p: Point<f64, Logical>) -> Option<Point<f64, Logical>> {
        let at = self.seen(p);
        let inside = |r: &Rect| at.x >= r.x as f64 && at.x < (r.x + r.w) as f64 && at.y >= r.y as f64 && at.y < (r.y + r.h) as f64;
        let b = self.bounds;
        let magnified = Rect::new(
            self.seen((b.x as f64, b.y as f64).into()).x.floor() as i32,
            self.seen((b.x as f64, b.y as f64).into()).y.floor() as i32,
            b.w * self.scale as i32,
            b.h * self.scale as i32,
        );
        (inside(&self.area) && inside(&magnified)).then_some(at)
    }

    /// The scale drawn `elapsed` after `since`: from 1 up to `scale` going in, back down
    /// leaving; and how far along (0 to 1, eased), for the shade's fade.
    pub fn drawn_scale(&self, leaving: bool) -> (f64, f64) {
        let t = if self.duration.is_zero() { 1.0 } else { (self.since.elapsed().as_secs_f64() / self.duration.as_secs_f64()).min(1.0) };
        let a = crate::tiling::ease_out(t);
        let a = if leaving { 1.0 - a } else { a };
        (1.0 + a * (self.scale as f64 - 1.0), a)
    }

    /// Whether the way out is over (nothing more to draw).
    pub fn gone(&self) -> bool {
        self.since.elapsed() >= self.duration
    }
}

/// The lens inside `area`: `DEAD_FRAME` off each side, less on a small one (at most a
/// sixth of the side, so most of it is still magnified).
pub fn lens(area: Rect) -> Rect {
    let dx = DEAD_FRAME.min(area.w / 6);
    let dy = DEAD_FRAME.min(area.h / 6);
    Rect::new(area.x + dx, area.y + dy, area.w - 2 * dx, area.h - 2 * dy)
}

/// The center that shows `pointer` in the middle of `area` magnified `zoom` times: c with
/// c + (p − c)·z the area's middle m, so c = (z·p − m)/(z − 1). Near an edge it is past
/// the edge, and what is past it fills the other side: padding, not a cut.
pub fn centered(pointer: Point<f64, Logical>, area: Rect, zoom: f64) -> Point<f64, Logical> {
    let m = Point::<f64, Logical>::from((area.x as f64 + area.w as f64 / 2.0, area.y as f64 + area.h as f64 / 2.0));
    Point::from(((zoom * pointer.x - m.x) / (zoom - 1.0), (zoom * pointer.y - m.y) / (zoom - 1.0)))
}

/// `centered`, but held so the edges of `bounds` (the tile) magnified come at most
/// `edge_in` into `area`: seen(b) = z·b − (z − 1)·c, so on x c ≥ (z·b.x − area.x −
/// edge_in)/(z − 1) and c ≤ (z·b.right − area.right + edge_in)/(z − 1); the same on y. A
/// tile too small for both keeps the pointer in the middle.
pub fn held(pointer: Point<f64, Logical>, area: Rect, bounds: Rect, zoom: f64, edge_in: f64) -> Point<f64, Logical> {
    let c = centered(pointer, area, zoom);
    let hold = |c: f64, a: i32, aw: i32, b: i32, bw: i32| {
        let lo = (zoom * b as f64 - a as f64 - edge_in) / (zoom - 1.0);
        let hi = (zoom * (b + bw) as f64 - (a + aw) as f64 + edge_in) / (zoom - 1.0);
        if lo <= hi { c.clamp(lo, hi) } else { c }
    };
    Point::from((hold(c.x, area.x, area.w, bounds.x, bounds.w), hold(c.y, area.y, area.h, bounds.y, bounds.h)))
}

/// Where a `RescaleRenderElement` of scale `output_scale · zoom` has its origin for
/// `center` to stay where it is: it draws g at o + (g − o)·S, and that is
/// s·(C + (g − C)·z) for o = s·C·(1 − z) / (1 − s·z). None where no origin does it (s·z
/// is 1 with z not 1: the element is not scaled at all).
pub fn origin(center: Point<f64, Logical>, output_scale: f64, zoom: f64) -> Option<Point<i32, Physical>> {
    let d = 1.0 - output_scale * zoom;
    let k = if d.abs() > 1e-9 {
        output_scale * (1.0 - zoom) / d
    } else if (zoom - 1.0).abs() < 1e-9 {
        1.0 // s = z = 1: as it is
    } else {
        return None;
    };
    Some(Point::from(((center.x * k).round() as i32, (center.y * k).round() as i32)))
}

/// The right Ctrl and Shift tapped alone, the keyboard's left click: both down together
/// with nothing else, then let go of both. Another key on the way (the left Ctrl or Shift
/// too) makes it a shortcut, not a tap.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ClickTap {
    /// The right Ctrl and Shift were down together, alone, and nothing else since.
    armed: bool,
    /// The right Ctrl is down.
    ctrl: bool,
    /// The right Shift is down.
    shift: bool,
}

impl ClickTap {
    /// A key went down or up, `mods` held after it: true when the tap is done (the last of
    /// the two let go, so the click is no Shift+click or Ctrl+click).
    fn key(&mut self, sym: Keysym, mods: &ModifiersState, pressed: bool) -> bool {
        match sym {
            Keysym::Control_R => self.ctrl = pressed,
            Keysym::Shift_R => self.shift = pressed,
            _ => {}
        }
        let right = matches!(sym, Keysym::Control_R | Keysym::Shift_R);
        let alone = self.ctrl && self.shift && !mods.logo && !mods.alt;
        match (pressed, right) {
            (true, true) => {
                self.armed |= alone;
                false
            }
            (true, false) => {
                self.armed = false;
                false
            }
            (false, _) if self.armed && !mods.ctrl && !mods.shift => {
                self.armed = false;
                true
            }
            (false, _) => false,
        }
    }
}

impl Dune {
    /// Every key that comes in, with the modifiers held after it: the right Ctrl+Shift
    /// tapped alone clicks where the pointer is (zoomed, that goes back out). The keys
    /// still go to the app as ever.
    pub fn track_click_tap(&mut self, sym: Keysym, mods: &ModifiersState, pressed: bool) {
        if self.session_lock.is_locked() {
            self.click_tap = ClickTap::default();
            return;
        }
        if self.click_tap.key(sym, mods, pressed) {
            self.click_by = "key";
            self.pointer_request(PointerOp::Click(MouseButton::Left));
        }
    }

    /// The `zoom` key: in on the tile under the pointer, the pointer in its middle; zoomed
    /// already, a left click where the pointer is, which goes back out (`zoom_out` goes out
    /// without one). The click is a plain one, Mod held or not: it never floats the tile.
    pub fn zoom_key(&mut self) {
        if self.zoom.is_some() {
            self.click_by = "key";
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
        let lens = lens(area);
        let center = held(here - geo.loc.to_f64(), lens, area, ZOOM as f64, EDGE_IN as f64);
        let zoom = Zoom { output: output.name(), tile, center, area: lens, bounds: area, scale: ZOOM, since: Instant::now(), duration: self.config.zoom.animation };
        self.tell_zoom(true, &zoom);
        self.zoom_leaving = None;
        self.zoom = Some(zoom);
    }

    /// Back to the monitor as it is (the `zoom_out` key, or a button pressed while in):
    /// at once for input and subscribers; the lens shrinks back as it goes (`Zoom::duration`),
    /// from the scale it had reached.
    pub fn zoom_out(&mut self) {
        if let Some(mut zoom) = self.zoom.take() {
            self.tell_zoom(false, &zoom);
            // caught growing: shrink from there, as long as it had grown
            let grown = zoom.since.elapsed().min(zoom.duration);
            zoom.since = Instant::now() - (zoom.duration - grown);
            self.zoom_leaving = Some(zoom);
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
        let o = origin((center, center).into(), s, z).unwrap().x as f64;
        o + (g - o) * s * z
    }

    #[test]
    fn the_center_stays_and_the_rest_grows_around_it() {
        assert_eq!(origin((800.0, 400.0).into(), 1.0, 3.0), Some(Point::from((800, 400))));
        // the animation's first frame, scale 1: as it is; s·z = 1 otherwise: none
        assert_eq!(origin((800.0, 400.0).into(), 1.0, 1.0), Some(Point::from((800, 400))));
        assert_eq!(origin((800.0, 400.0).into(), 0.5, 2.0), None);
        assert_eq!(drawn(800.0, 800.0, 1.0, 3.0), 800.0);
        assert_eq!(drawn(810.0, 800.0, 1.0, 3.0), 830.0); // 10 px off it: 30
        // on a monitor of scale 2: still where it was (2·C), 10 px off it 60 px off
        assert!((drawn(800.0, 800.0, 2.0, 3.0) - 1600.0).abs() < 3.0);
        assert!((drawn(810.0, 800.0, 2.0, 3.0) - drawn(800.0, 800.0, 2.0, 3.0) - 60.0).abs() < 1e-9);
    }

    fn zoom(center: Point<f64, Logical>, area: Rect, bounds: Rect) -> Zoom {
        Zoom { output: "M".into(), tile: None, center, area, bounds, scale: 3, since: Instant::now(), duration: ANIMATION }
    }

    #[test]
    fn the_lens_leaves_a_dead_frame_inside_the_tile() {
        assert_eq!(lens(Rect::new(100, 50, 1200, 900)), Rect::new(150, 100, 1100, 800));
        // a small tile keeps most of it: a sixth of the side at most
        assert_eq!(lens(Rect::new(0, 0, 240, 120)), Rect::new(40, 20, 160, 80));
    }

    #[test]
    fn a_point_is_drawn_in_the_lens_only_when_it_lands_there() {
        let tile = Rect::new(0, 0, 1200, 900);
        let lens = lens(tile); // (50, 50) 1100x800
        let z = zoom(centered((600.0, 450.0).into(), lens, 3.0), lens, tile);
        assert_eq!(z.drawn((600.0, 450.0).into()), Some(Point::from((600.0, 450.0))));
        // 250 px off the pointer is 750 seen: past the lens, the real point shows
        assert_eq!(z.drawn((850.0, 450.0).into()), None);
        // near the tile's left edge: past it (magnified) the real tile shows, not the zoom
        let z = zoom(centered((10.0, 450.0).into(), lens, 3.0), lens, tile);
        assert!(z.drawn((5.0, 450.0).into()).is_some());
        assert!(z.drawn((-5.0, 450.0).into()).is_none());
    }

    #[test]
    fn the_lens_grows_from_the_real_size_and_shrinks_back() {
        let mut z = zoom((0.0, 0.0).into(), Rect::new(0, 0, 10, 10), Rect::new(0, 0, 10, 10));
        assert!((z.drawn_scale(false).0 - 1.0).abs() < 0.2);
        z.since = Instant::now() - ANIMATION;
        assert_eq!(z.drawn_scale(false), (3.0, 1.0));
        assert_eq!(z.drawn_scale(true), (1.0, 0.0));
        assert!(z.gone());
    }

    fn mods(ctrl: bool, shift: bool, logo: bool) -> ModifiersState {
        ModifiersState { ctrl, shift, logo, ..Default::default() }
    }

    #[test]
    fn right_ctrl_shift_tapped_alone_clicks_on_the_last_release() {
        let mut t = ClickTap::default();
        assert!(!t.key(Keysym::Control_R, &mods(true, false, false), true));
        assert!(!t.key(Keysym::Shift_R, &mods(true, true, false), true));
        // the first let go: Ctrl still down, no click yet (it would be a Ctrl+click)
        assert!(!t.key(Keysym::Shift_R, &mods(true, false, false), false));
        assert!(t.key(Keysym::Control_R, &mods(false, false, false), false));
        // and once: nothing more on the next releases
        assert!(!t.key(Keysym::Control_R, &mods(false, false, false), false));
    }

    #[test]
    fn a_shortcut_or_super_on_the_way_is_no_tap() {
        let mut t = ClickTap::default();
        t.key(Keysym::Control_R, &mods(true, false, false), true);
        t.key(Keysym::Shift_R, &mods(true, true, false), true);
        t.key(Keysym::t, &mods(true, true, false), true); // Ctrl+Shift+T
        t.key(Keysym::t, &mods(true, true, false), false);
        t.key(Keysym::Shift_R, &mods(true, false, false), false);
        assert!(!t.key(Keysym::Control_R, &mods(false, false, false), false));

        let mut t = ClickTap::default();
        t.key(Keysym::Control_R, &mods(true, false, true), true);
        t.key(Keysym::Shift_R, &mods(true, true, true), true); // with Super: not alone
        t.key(Keysym::Shift_R, &mods(true, false, true), false);
        assert!(!t.key(Keysym::Control_R, &mods(false, false, true), false));

        // the left ones are the apps' Ctrl+Shift, no tap
        let mut t = ClickTap::default();
        t.key(Keysym::Control_L, &mods(true, false, false), true);
        t.key(Keysym::Shift_L, &mods(true, true, false), true);
        t.key(Keysym::Shift_L, &mods(true, false, false), false);
        assert!(!t.key(Keysym::Control_L, &mods(false, false, false), false));

        // nor one left and one right
        let mut t = ClickTap::default();
        t.key(Keysym::Control_L, &mods(true, false, false), true);
        t.key(Keysym::Shift_R, &mods(true, true, false), true);
        t.key(Keysym::Shift_R, &mods(true, false, false), false);
        assert!(!t.key(Keysym::Control_L, &mods(false, false, false), false));

        // one of them alone is no tap either
        let mut t = ClickTap::default();
        t.key(Keysym::Shift_R, &mods(false, true, false), true);
        assert!(!t.key(Keysym::Shift_R, &mods(false, false, false), false));
    }

    #[test]
    fn the_pointer_is_seen_in_the_middle_of_the_tile_even_by_its_edge() {
        let area = Rect::new(100, 50, 600, 400); // middle (400, 250)
        for p in [(400.0, 250.0), (110.0, 60.0), (690.0, 440.0), (250.0, 400.0)] {
            let z = zoom(centered(p.into(), area, 3.0), area, area);
            let seen = z.seen(p.into());
            assert!((seen.x - 400.0).abs() < 1e-9 && (seen.y - 250.0).abs() < 1e-9, "{p:?} seen at {seen:?}");
        }
        // 10 px from the left edge: the edge is seen 30 px left of the middle, not cut
        let z = zoom(centered((110.0, 250.0).into(), area, 3.0), area, area);
        assert!((z.seen((100.0, 250.0).into()).x - 370.0).abs() < 1e-9);
    }

    #[test]
    fn by_an_edge_the_tile_comes_in_only_edge_in() {
        let tile = Rect::new(0, 0, 1200, 900);
        let lens = lens(tile); // (50, 50) 1100x800
        // in the middle: the pointer in the middle
        let z = zoom(held((600.0, 450.0).into(), lens, tile, 3.0, 50.0), lens, tile);
        assert!((z.seen((600.0, 450.0).into()).x - 600.0).abs() < 1e-9);
        // 20 px from the left edge: the edge 50 px into the lens, not by its middle
        let z = zoom(held((20.0, 450.0).into(), lens, tile, 3.0, 50.0), lens, tile);
        assert!((z.seen((0.0, 450.0).into()).x - 100.0).abs() < 1e-9);
        assert!((z.seen((20.0, 450.0).into()).x - 160.0).abs() < 1e-9);
        // and the right and bottom edges alike
        let z = zoom(held((1190.0, 890.0).into(), lens, tile, 3.0, 50.0), lens, tile);
        assert!((z.seen((1200.0, 900.0).into()).x - 1100.0).abs() < 1e-9);
        assert!((z.seen((1200.0, 900.0).into()).y - 800.0).abs() < 1e-9);
    }

    #[test]
    fn a_real_point_is_seen_three_times_as_far_from_the_center() {
        let z = zoom((800.0, 400.0).into(), Rect::new(0, 0, 1600, 900), Rect::new(0, 0, 1600, 900));
        assert_eq!(z.seen((800.0, 400.0).into()), Point::from((800.0, 400.0)));
        assert_eq!(z.seen((820.0, 390.0).into()), Point::from((860.0, 370.0)));
    }
}
