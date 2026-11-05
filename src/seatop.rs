//! What the pointer is doing, as sway's seat operations (`seatop_*.c`): pointing at
//! things (`Seatop::Default`), or a Mod+drag that owns the motion and the buttons until it
//! ends. One mode at a time: a drag starts only from the default one, ends with its
//! button, with the lock, or when its tile goes away.

use smithay::{
    backend::input::ButtonState,
    utils::{Logical, Point},
};

use crate::{
    Dune,
    layout::{Rect, Side, TileId},
};

/// The left and right buttons, as evdev numbers them.
pub const BTN_LEFT: u32 = 0x110;
pub const BTN_RIGHT: u32 = 0x111;

/// How far the pointer may move during a Mod+click before it counts as a drag.
pub const CLICK_SLOP: f64 = 4.0;

/// A Mod+left button drag of a tile.
#[derive(Clone, Copy, Debug)]
pub struct MoveDrag {
    pub tile: TileId,
    /// Where the button went down.
    pub start: Point<f64, Logical>,
    /// The pointer went far enough to make it a drag rather than a click.
    pub moved: bool,
    /// Where a floating tile was when the drag started.
    pub from: Option<Rect>,
    /// Pressed near this edge of the tile (`split_edge`): a click there opens a split.
    pub split: Option<Side>,
    /// It is the `dock_click`: a click puts the tile in the dock instead of floating it.
    pub dock: bool,
}

/// A Mod+right button drag resizing a tile.
#[derive(Clone, Copy, Debug)]
pub struct ResizeDrag {
    pub tile: TileId,
    /// Where the button went down.
    pub start: Point<f64, Logical>,
    /// The pointer when last seen.
    pub last: Point<f64, Logical>,
    /// It is the `dock_click`: a click (no drag) puts the tile in the dock.
    pub dock: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Seatop {
    /// Motion and buttons go to what is under the pointer.
    #[default]
    Default,
    /// Mod+left: the tile is dropped where the button is let go.
    Move(MoveDrag),
    /// Mod+right: the tile follows the pointer, right/down growing it.
    Resize(ResizeDrag),
}

impl Seatop {
    pub fn is_default(&self) -> bool {
        matches!(self, Seatop::Default)
    }

    /// The tile a drag holds.
    fn tile(&self) -> Option<TileId> {
        match self {
            Seatop::Default => None,
            Seatop::Move(d) => Some(d.tile),
            Seatop::Resize(d) => Some(d.tile),
        }
    }

    /// The move drag going on, if one is.
    pub fn moving(&self) -> Option<MoveDrag> {
        match self {
            Seatop::Move(d) => Some(*d),
            _ => None,
        }
    }

    /// The move past a click, or a resize: what lights no split hint.
    pub fn dragging(&self) -> bool {
        match self {
            Seatop::Default => false,
            Seatop::Move(d) => d.moved,
            Seatop::Resize(_) => true,
        }
    }
}

/// Whether the pointer went from `start` to `pos` without leaving the click's slop.
fn still(start: Point<f64, Logical>, pos: Point<f64, Logical>) -> bool {
    (pos.x - start.x).hypot(pos.y - start.y) <= CLICK_SLOP
}

impl Dune {
    /// The pointer moved to `pos`: the drag going on follows it. One whose tile went away
    /// ends.
    pub fn seatop_motion(&mut self, pos: Point<f64, Logical>) {
        if self.seatop.tile().is_some_and(|t| !self.tiles.contains_key(&t)) {
            self.seatop = Seatop::Default;
        }
        match &mut self.seatop {
            Seatop::Default => {}
            Seatop::Resize(drag) => {
                let (id, last) = (drag.tile, std::mem::replace(&mut drag.last, pos));
                self.drag_resize(id, last, pos);
            }
            Seatop::Move(drag) => {
                if !still(drag.start, pos) {
                    drag.moved = true;
                }
                let drag = *drag;
                if let (true, Some(from)) = (drag.moved, drag.from) {
                    self.drag_float(drag.tile, from, drag.start, pos);
                }
            }
        }
    }

    /// A button at `pos` (`mod_held`: the modifier is down; `dock`: it is the
    /// `dock_click`). The drag going on ends with its own button let go; in the default
    /// mode, Mod+left or Mod+right on a tile starts one. `true`: the button was the
    /// mode's, the client does not get it.
    pub fn seatop_button(&mut self, button: u32, state: ButtonState, pos: Point<f64, Logical>, mod_held: bool, dock: bool) -> bool {
        if self.seatop.tile().is_some_and(|t| !self.tiles.contains_key(&t)) {
            self.seatop = Seatop::Default;
        }
        match (self.seatop, button, state) {
            (Seatop::Resize(drag), BTN_RIGHT, ButtonState::Released) => {
                self.seatop = Seatop::Default;
                self.remember_sticky_place(drag.tile);
                if drag.dock && still(drag.start, pos) {
                    self.send_to_dock(drag.tile);
                }
                true
            }
            // Mod+left drags move the tile: it lands where the button is released, on this
            // monitor or another one; a float just goes where it is dragged. Let go on a tab
            // bar, either one joins that container as a tab. A Mod+click without moving
            // floats the tile, or puts a float back; near an edge of a tile of the strip
            // (`split_hint`), it opens an empty split there.
            (Seatop::Move(drag), BTN_LEFT, ButtonState::Released) => {
                self.seatop = Seatop::Default;
                if !drag.moved {
                    match drag.split {
                        Some(side) => self.open_empty_split(drag.tile, side),
                        None if drag.dock => self.send_to_dock(drag.tile),
                        None => self.toggle_floating(drag.tile),
                    }
                } else if drag.from.is_none() {
                    self.drop_tile(drag.tile, pos);
                } else if let Some(target) = self.drop_spot(drag.tile, pos).filter(|s| s.tab).and_then(|s| s.target) {
                    // A float let go on a tab bar becomes a tab there.
                    self.join_tab(drag.tile, target);
                }
                // A sticky float: its place, for the next window of its app.
                self.remember_sticky_place(drag.tile);
                true
            }
            // While dragging, the other buttons are the drag's too.
            (Seatop::Move(_) | Seatop::Resize(_), ..) => true,
            (Seatop::Default, BTN_LEFT | BTN_RIGHT, ButtonState::Pressed) if mod_held => {
                let Some(id) = self.tile_at(pos.x, pos.y).or_else(|| self.tab_at(pos.x, pos.y)) else { return false };
                let split = (button == BTN_LEFT).then(|| self.split_hint_at(pos).filter(|(t, ..)| *t == id).map(|(_, side, _)| side)).flatten();
                self.focus_tile(id);
                self.seatop = if button == BTN_LEFT {
                    Seatop::Move(MoveDrag { tile: id, start: pos, moved: false, from: self.float_rect(id), split, dock })
                } else {
                    Seatop::Resize(ResizeDrag { tile: id, start: pos, last: pos, dock })
                };
                true
            }
            (Seatop::Default, ..) => false,
        }
    }

    /// Mod+right drag from `last` to `pos`: a float grows by as much, a tile of the strip by
    /// that share of its monitor.
    fn drag_resize(&mut self, id: TileId, last: Point<f64, Logical>, pos: Point<f64, Logical>) {
        if self.is_floating(id) {
            return self.resize_float(id, pos.x - last.x, pos.y - last.y);
        }
        let output = self.space.outputs().find(|o| self.layouts.get(&o.name()).is_some_and(|l| l.contains(id)));
        let Some(area) = output.and_then(|o| self.space.output_geometry(o)) else { return };
        let (dx, dy) = ((pos.x - last.x) / area.size.w as f64, (pos.y - last.y) / area.size.h as f64);
        self.change_tile(id, |l, id| {
            l.resize_width(id, dx);
            l.resize_height(id, dy);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_stays_inside_the_slop() {
        let start = Point::from((100.0, 100.0));
        assert!(still(start, Point::from((103.0, 102.0))));
        assert!(!still(start, Point::from((105.0, 100.0))));
    }

    #[test]
    fn only_a_real_drag_counts_as_dragging() {
        let start = Point::from((0.0, 0.0));
        let drag = MoveDrag { tile: 1, start, moved: false, from: None, split: None, dock: false };
        assert!(!Seatop::Move(drag).dragging());
        assert!(Seatop::Move(MoveDrag { moved: true, ..drag }).dragging());
        assert!(Seatop::Resize(ResizeDrag { tile: 1, start, last: start, dock: false }).dragging());
        assert!(!Seatop::Default.dragging() && Seatop::Default.is_default());
        assert_eq!(Seatop::Move(drag).moving().map(|d| d.tile), Some(1));
    }
}
