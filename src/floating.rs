//! Floating tiles: out of the strip of columns, above it, at a place of their own on
//! their output. A floating tile is still a tile (same `TileId` and `Tile`), it just
//! lives in `Dune::floating` instead of a `Layout`.

use smithay::{
    utils::{Logical, Point},
    wayland::{compositor::with_states, shell::xdg::{SurfaceCachedState, ToplevelSurface, XdgToplevelSurfaceData}},
};

use crate::{
    Dune,
    layout::{Rect, Side, TileId},
};

/// Shares of the output's width and height a tile takes when it starts floating: taller
/// than wide, as most of what floats (terminals, editors, chats) reads top to bottom.
const FLOAT_WIDTH: f64 = 0.6;
const FLOAT_HEIGHT: f64 = 0.85;
/// Share of the output a dialog without a size of its own takes.
const DIALOG_SHARE: f64 = 0.5;
/// Smallest floating tile, so a resize cannot make it vanish.
const MIN_FLOAT: i32 = 100;
/// Share of the output's width a tile takes when it starts sticky out of the strip (a
/// camera, a video): 16:9, in the bottom right corner.
const STICKY_WIDTH: f64 = 0.22;
/// Room left between a tile that starts sticky and the output's edges.
const STICKY_MARGIN: i32 = 24;

/// Where a tile that starts sticky out of the strip goes: small, 16:9, bottom right of `area`.
pub fn sticky_start(area: Rect) -> Rect {
    let w = ((area.w as f64 * STICKY_WIDTH) as i32).max(MIN_FLOAT);
    let h = (w * 9 / 16).max(MIN_FLOAT);
    keep_inside(Rect::new(area.x + area.w - w - STICKY_MARGIN, area.y + area.h - h - STICKY_MARGIN, w, h), area)
}

/// `stack` (back to front) with the tiles `sticky` says at its front, each part in its order.
fn sticky_in_front(stack: &mut Vec<(TileId, Rect)>, sticky: impl Fn(TileId) -> bool) {
    let (front, back): (Vec<_>, Vec<_>) = stack.drain(..).partition(|(t, _)| sticky(*t));
    stack.extend(back);
    stack.extend(front);
}

/// A `w` x `h` rectangle in the middle of `area`.
pub fn centered(area: Rect, w: i32, h: i32) -> Rect {
    let (w, h) = (w.min(area.w), h.min(area.h));
    Rect::new(area.x + (area.w - w) / 2, area.y + (area.h - h) / 2, w, h)
}

/// Size of a tile that starts floating in `area`.
pub fn start_size(area: Rect) -> (i32, i32) {
    ((area.w as f64 * FLOAT_WIDTH) as i32, (area.h as f64 * FLOAT_HEIGHT) as i32)
}

/// `rect` grown by `dw`/`dh` shares of `area` around its middle, kept inside `area`.
fn scaled(rect: Rect, area: Rect, dw: f64, dh: f64) -> Rect {
    let w = (rect.w + (area.w as f64 * dw).round() as i32).clamp(MIN_FLOAT.min(area.w), area.w);
    let h = (rect.h + (area.h as f64 * dh).round() as i32).clamp(MIN_FLOAT.min(area.h), area.h);
    let (cx, cy) = (rect.x + rect.w / 2, rect.y + rect.h / 2);
    keep_inside(Rect::new(cx - w / 2, cy - h / 2, w, h), area)
}

/// `rect` moved (and shrunk if needed) so that it is all inside `area`.
pub fn keep_inside(rect: Rect, area: Rect) -> Rect {
    let (w, h) = (rect.w.clamp(MIN_FLOAT.min(area.w), area.w), rect.h.clamp(MIN_FLOAT.min(area.h), area.h));
    let x = rect.x.clamp(area.x, area.x + area.w - w);
    let y = rect.y.clamp(area.y, area.y + area.h - h);
    Rect::new(x, y, w, h)
}

impl Dune {
    pub fn is_floating(&self, id: TileId) -> bool {
        self.floating.values().any(|stack| stack.iter().any(|(t, _)| *t == id))
    }

    /// The rectangle of a floating tile.
    pub fn float_rect(&self, id: TileId) -> Option<Rect> {
        self.floating.values().flatten().find(|(t, _)| *t == id).map(|(_, r)| *r)
    }

    fn set_float_rect(&mut self, id: TileId, rect: Rect) {
        if let Some((_, r)) = self.floating.values_mut().flatten().find(|(t, _)| *t == id) {
            *r = rect;
        }
    }

    /// Float `id` at `rect` on `output`, on top of the other floats there.
    pub fn float_tile(&mut self, id: TileId, output: &str, rect: Rect) {
        self.remove_from_layouts(id);
        self.floating.entry(output.to_string()).or_default().push((id, rect));
    }

    /// Put the focused tile, or the one at `pos`, out of the strip in the middle of its
    /// output; a floating one goes back to where it was in the strip, or (when that place
    /// is gone, or it floated in from elsewhere) next to the tile under its middle.
    pub fn toggle_floating(&mut self, id: TileId) {
        let Some(output) = self.output_of(id) else { return };
        let Some(area) = self.space.outputs().find(|o| o.name() == output).and_then(|o| self.output_rect(o)) else {
            return;
        };
        if let Some(rect) = self.float_rect(id) {
            let home = self.float_homes.get(&id).filter(|(o, _)| *o == output).map(|(_, h)| *h);
            if let Some(home) = home {
                self.remove_from_layouts(id);
                if self.layouts.entry(output.clone()).or_default().restore(id, home) {
                    return self.focus_tile(id);
                }
            }
            let (cx, cy) = (rect.x as f64 + rect.w as f64 / 2.0, rect.y as f64 + rect.h as f64 / 2.0);
            let layout = self.layouts.entry(output.clone()).or_default();
            let target = self.shown.iter().find(|(t, r)| layout.contains(*t) && r.contains(cx, cy)).map(|(t, _)| *t);
            self.remove_from_layouts(id);
            self.layouts.entry(output).or_default().insert_column(id, target, Side::Right);
        } else {
            let (w, h) = start_size(area);
            let home = self.layouts.get(&output).and_then(|l| l.home(id));
            self.float_tile(id, &output, centered(area, w, h));
            if let Some(home) = home {
                self.float_homes.insert(id, (output, home));
            }
        }
        self.focus_tile(id);
    }

    /// A focused float comes in front of the others on its output (but the sticky ones).
    pub fn raise_float(&mut self, id: TileId) {
        for stack in self.floating.values_mut() {
            if let Some(i) = stack.iter().position(|(t, _)| *t == id) {
                let entry = stack.remove(i);
                stack.push(entry);
            }
            sticky_in_front(stack, |t| self.sticky.contains(&t));
        }
    }

    /// The `sticky` tile option: `id` floats in front of every other tile, on every desktop
    /// of its monitor, where it is (out of the strip: where the last sticky window of its
    /// app was, on that monitor, else small, in the bottom right corner of this one);
    /// again: an ordinary float.
    pub fn toggle_sticky(&mut self, id: TileId) {
        if self.sticky.remove(&id) {
            tracing::info!(tile = id, "sticky off");
            return self.arrange();
        }
        // On a desktop not shown it would stay there: only a tile shown can be sticky.
        let Some(output) = self.output_of(id) else { return };
        if !self.is_floating(id) {
            let Some(area) = self.space.outputs().find(|o| o.name() == output).and_then(|o| self.output_rect(o)) else {
                return;
            };
            let home = self.layouts.get(&output).and_then(|l| l.home(id));
            let (on, rect) =
                self.remembered_sticky_place(id, &output).unwrap_or_else(|| (output.clone(), sticky_start(area)));
            self.float_tile(id, &on, rect);
            if let Some(home) = home {
                self.float_homes.insert(id, (output, home));
            }
        }
        self.sticky.insert(id);
        tracing::info!(tile = id, "sticky on");
        self.raise_float(id);
        self.arrange();
    }

    /// A Mod+drag of a float: it follows the pointer from where it was (`from`) when the
    /// drag started at `start`.
    pub fn drag_float(&mut self, id: TileId, from: Rect, start: Point<f64, Logical>, pos: Point<f64, Logical>) {
        let (dx, dy) = ((pos.x - start.x).round() as i32, (pos.y - start.y).round() as i32);
        self.set_float_rect(id, Rect::new(from.x + dx, from.y + dy, from.w, from.h));
        self.arrange();
    }

    /// A Mod+right drag of a float: right/down grow it by the pointer's motion.
    pub fn resize_float(&mut self, id: TileId, dx: f64, dy: f64) {
        let Some(r) = self.float_rect(id) else { return };
        let (w, h) = ((r.w as f64 + dx).round() as i32, (r.h as f64 + dy).round() as i32);
        self.set_float_rect(id, Rect::new(r.x, r.y, w.max(MIN_FLOAT), h.max(MIN_FLOAT)));
        self.arrange();
    }

    /// A resize key or Alt+scroll on a float: it grows (or shrinks) around its middle by
    /// `dw`/`dh`, shares of its output, like a column's width.
    pub fn scale_float(&mut self, id: TileId, dw: f64, dh: f64) {
        let Some(r) = self.float_rect(id) else { return };
        let area = self.floating.iter().find(|(_, s)| s.iter().any(|(t, _)| *t == id)).and_then(|(name, _)| {
            self.space.outputs().find(|o| o.name() == *name).and_then(|o| self.output_rect(o))
        });
        let Some(area) = area else { return };
        self.set_float_rect(id, scaled(r, area, dw, dh));
        self.arrange();
        self.remember_sticky_place(id);
    }

    /// Floats go to the output their middle is on (they may have been dragged to another
    /// one, or their output went away) and are kept inside it.
    pub fn settle_floats(&mut self) {
        let areas: Vec<(String, Rect)> =
            self.space.outputs().filter_map(|o| Some((o.name(), self.output_rect(o)?))).collect();
        let Some((first, _)) = areas.first().cloned() else { return };
        let mut settled: std::collections::HashMap<String, Vec<(TileId, Rect)>> = Default::default();
        let mut names: Vec<String> = self.floating.keys().cloned().collect();
        names.sort();
        for name in names {
            for (id, rect) in self.floating.remove(&name).unwrap() {
                let (cx, cy) = (rect.x as f64 + rect.w as f64 / 2.0, rect.y as f64 + rect.h as f64 / 2.0);
                let (owner, area) = areas
                    .iter()
                    .find(|(_, a)| a.contains(cx, cy))
                    .or_else(|| areas.iter().find(|(n, _)| *n == name))
                    .or_else(|| areas.iter().find(|(n, _)| *n == first))
                    .unwrap();
                settled.entry(owner.clone()).or_default().push((id, keep_inside(rect, *area)));
            }
        }
        for stack in settled.values_mut() {
            sticky_in_front(stack, |t| self.sticky.contains(&t));
        }
        self.floating = settled;
    }

    /// Before its first configure, a dialog (a toplevel with a parent, or one with a fixed
    /// size) leaves the strip and floats in the middle of its output.
    pub fn float_if_dialog(&mut self, surface: &ToplevelSurface) {
        let Some(id) = self.tiles.iter().find(|(_, t)| t.top().and_then(|w| w.toplevel()).is_some_and(|t| t == surface)).map(|(id, _)| *id)
        else {
            return;
        };
        if self.is_floating(id) || self.tiles[&id].stack.len() > 1 {
            return;
        }
        let wl = surface.wl_surface();
        let initial_sent = with_states(wl, |states| {
            states.data_map.get::<XdgToplevelSurfaceData>().is_some_and(|d| d.lock().unwrap().initial_configure_sent)
        });
        if initial_sent {
            return;
        }
        let (min, max) = with_states(wl, |states| {
            let mut cached = states.cached_state.get::<SurfaceCachedState>();
            let s = cached.pending();
            (s.min_size, s.max_size)
        });
        let fixed = min.w > 0 && min.h > 0 && min == max;
        if !fixed && surface.parent().is_none() {
            return;
        }
        let Some(output) = self.output_of(id) else { return };
        let Some(area) = self.space.outputs().find(|o| o.name() == output).and_then(|o| self.output_rect(o)) else {
            return;
        };
        let (w, h) = if min.w > 0 && min.h > 0 {
            (min.w, min.h)
        } else {
            ((area.w as f64 * DIALOG_SHARE) as i32, (area.h as f64 * DIALOG_SHARE) as i32)
        };
        tracing::info!(tile = id, fixed, "dialog floats");
        self.float_tile(id, &output, centered(area, w, h));
        self.arrange();
        self.update_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect { x: 100, y: 0, w: 1000, h: 600 };

    #[test]
    fn a_sticky_tile_starts_small_in_the_bottom_right_corner() {
        let r = sticky_start(Rect::new(0, 0, 2000, 1000));
        assert_eq!((r.w, r.h), (440, 247));
        assert_eq!((r.x + r.w, r.y + r.h), (2000 - STICKY_MARGIN, 1000 - STICKY_MARGIN));
        // A small output: still all inside it.
        let r = sticky_start(Rect::new(0, 0, 300, 200));
        assert!(r.x >= 0 && r.y >= 0 && r.x + r.w <= 300 && r.y + r.h <= 200);
    }

    #[test]
    fn sticky_floats_stay_in_front() {
        let r = Rect::new(0, 0, 10, 10);
        let mut stack = vec![(1, r), (2, r), (3, r), (4, r)];
        sticky_in_front(&mut stack, |t| t == 2 || t == 1);
        assert_eq!(stack.iter().map(|(t, _)| *t).collect::<Vec<_>>(), [3, 4, 1, 2]);
    }

    #[test]
    fn floats_start_in_the_middle() {
        assert_eq!(centered(AREA, 600, 360), Rect::new(300, 120, 600, 360));
        // Never bigger than the output.
        assert_eq!(centered(AREA, 5000, 360), Rect::new(100, 120, 1000, 360));
    }

    #[test]
    fn floats_start_taller_than_their_share_of_the_width() {
        assert_eq!(start_size(AREA), (600, 510));
    }

    #[test]
    fn floats_resize_around_their_middle() {
        let r = Rect::new(300, 120, 600, 360);
        assert_eq!(scaled(r, AREA, 0.1, 0.0), Rect::new(250, 120, 700, 360));
        assert_eq!(scaled(r, AREA, 0.0, -0.1), Rect::new(300, 150, 600, 300));
        // Never smaller than the minimum, never bigger than the output.
        assert_eq!(scaled(r, AREA, -5.0, 0.0).w, MIN_FLOAT);
        assert_eq!(scaled(r, AREA, 5.0, 5.0), AREA);
    }

    #[test]
    fn floats_stay_on_their_output() {
        assert_eq!(keep_inside(Rect::new(300, 120, 600, 360), AREA), Rect::new(300, 120, 600, 360));
        assert_eq!(keep_inside(Rect::new(-50, 500, 600, 360), AREA), Rect::new(100, 240, 600, 360));
        assert_eq!(keep_inside(Rect::new(900, 0, 2000, 10), AREA), Rect::new(100, 0, 1000, 100));
    }
}
