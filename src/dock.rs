//! The dock: tiles put away from a desktop (the `dock` key, `dock_click`, the tile options),
//! one dock per monitor and desktop. A tile of a tabbed column takes its whole column, tabs
//! and all. A pinned entry is in the dock of every desktop, and comes out on the one shown
//! (a tile floating, a column right of the focused one). Unlike the scratchpad, whose queue
//! is the same everywhere and only floats, the dock keeps each desktop's own windows.
//!
//! dune-shell draws it: `dune events dock` tells each monitor's dock (its entries and the
//! scratchpad's windows) and the `dock_show` key (Mod+Shift+e) or `dune dock show` asks it
//! to show or hide one (see `crate::events`). `dune dock take TILE` takes an entry out, with
//! that tab shown, and `dune dock pin TILE` pins or unpins it.
//!
//! Bars follow each monitor's dock with `dune dock watch` (`watch_dock`), told its windows
//! now and on every change.

use std::{io::Write, os::unix::net::UnixStream};

use crate::{
    Dune,
    floating::{centered, keep_inside, start_size},
    layout::{Detached, GroupId, Home, Rect, Side, TileId},
    scratchpad::json_escape,
};

/// What a dock entry holds.
#[derive(Debug)]
pub enum Piece {
    /// One tile, with its place in the strip or where it floated.
    Tile { id: TileId, home: Option<Home>, float: Option<Rect> },
    /// A tabbed column, whole, with the column it sat by.
    Column { column: Detached, by: Option<(TileId, Side)> },
}

/// What a pinned entry taken out of the dock is remembered by, to be pinned when it goes
/// back: a tile, or a tabbed column by its id, however many tabs it has by then.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pin {
    Tile(TileId),
    Group(GroupId),
}

/// A dock entry.
#[derive(Debug)]
pub struct Docked {
    pub piece: Piece,
    /// In the dock of every desktop.
    pub pinned: bool,
    /// The monitor and desktop whose dock has it (not pinned).
    pub output: String,
    pub desk: u8,
}

impl Docked {
    pub fn tiles(&self) -> Vec<TileId> {
        match &self.piece {
            Piece::Tile { id, .. } => vec![*id],
            Piece::Column { column, .. } => column.tiles(),
        }
    }

    /// The tile it is named by: the one, or the column's shown tab.
    pub fn lead(&self) -> TileId {
        match &self.piece {
            Piece::Tile { id, .. } => *id,
            Piece::Column { column, .. } => column.active().unwrap_or_default(),
        }
    }

    /// The first tile of each of its column's tabs (a tile: itself).
    pub fn leads(&self) -> Vec<TileId> {
        match &self.piece {
            Piece::Tile { id, .. } => vec![*id],
            Piece::Column { column, .. } => column.leads(),
        }
    }

    /// What it is remembered by when taken out pinned (see `Pin`): its column's id, or the tile.
    fn pin(&self) -> Pin {
        match &self.piece {
            Piece::Column { column, .. } => column.id().map_or(Pin::Tile(self.lead()), Pin::Group),
            Piece::Tile { id, .. } => Pin::Tile(*id),
        }
    }

    /// How many tabs its column has (none for a tile).
    pub fn tabs(&self) -> usize {
        match &self.piece {
            Piece::Tile { .. } => 0,
            Piece::Column { column, .. } => column.tabs(),
        }
    }
}

/// The line bars get for a dock holding windows with `titles`, and the scratchpad (the
/// dock shows it too) `scratch`: the dock's count (0 too: the module stays, to be clicked), all
/// the titles as tooltip, class `empty` or `filled`.
pub fn status_line(titles: &[String], scratch: &[String]) -> String {
    let dock = std::iter::once("Dock".to_string()).chain(titles.iter().cloned());
    let scratch = (!scratch.is_empty()).then(|| std::iter::once("Scratchpad".to_string()).chain(scratch.iter().cloned()));
    let tooltip = dock.chain(scratch.into_iter().flatten()).collect::<Vec<_>>().join("\n");
    let class = if titles.is_empty() { "empty" } else { "filled" };
    format!(r#"{{"text":"{}","tooltip":"{}","class":"{class}"}}"#, titles.len(), json_escape(&tooltip))
}

impl Dune {
    /// Whether `id` is in a dock.
    pub fn in_dock(&self, id: TileId) -> bool {
        self.dock.iter().any(|d| d.tiles().contains(&id))
    }

    /// `id` (with its column, when that has tabs) into the dock of the desktop it is on,
    /// pinned when it came out of the dock pinned. Focus goes to the tile focused before it there.
    pub fn send_to_dock(&mut self, id: TileId) {
        if !self.tiles.contains_key(&id) || self.in_dock(id) || self.in_scratchpad(id) {
            return;
        }
        let Some(output) = self.output_of(id) else { return };
        let desk = self.desk_of(&output);
        // Its column, by the id it keeps however many tabs it has: one pinned out of the
        // dock goes back pinned, though left with one tab (then only the tile goes).
        let group = self.layouts.get(&output).and_then(|l| l.group_id(id));
        let piece = match self.float_rect(id) {
            Some(rect) => {
                self.remove_from_layouts(id);
                Piece::Tile { id, home: None, float: Some(rect) }
            }
            None => {
                let Some(layout) = self.layouts.get_mut(&output) else { return };
                if layout.group(id).is_some() {
                    let Some((column, by)) = layout.take_column(id) else { return };
                    Piece::Column { column, by }
                } else {
                    let home = layout.home(id);
                    self.remove_from_layouts(id);
                    Piece::Tile { id, home, float: None }
                }
            }
        };
        let mut entry = Docked { piece, pinned: false, output: output.clone(), desk };
        let tiles = entry.tiles();
        // A tile pinned alone that has gained tabs since goes back pinned too.
        let pins = group.map(Pin::Group).into_iter().chain(tiles.iter().copied().map(Pin::Tile));
        entry.pinned = pins.fold(false, |pinned, p| self.dock_pins.remove(&p) | pinned);
        tracing::info!(?tiles, output, desk, pinned = entry.pinned, "into the dock");
        self.hide_windows(&tiles);
        self.dock.push(entry);
        if self.focused.is_some_and(|f| tiles.contains(&f)) {
            let on = |t: TileId| self.output_of(t).as_deref() == Some(output.as_str());
            self.focused = self.recent.iter().copied().find(|t| on(*t)).or_else(|| {
                let floats = self.floating.get(&output).into_iter().flatten().map(|(t, _)| *t);
                floats.chain(self.layouts.get(&output).map(|l| l.tiles()).unwrap_or_default()).next()
            });
        }
        self.arrange();
        self.update_focus();
    }

    /// The entries of the dock `output` shows, by their index in `Dune::dock`: the pinned
    /// ones first, then those of the desktop it shows.
    pub fn dock_entries(&self, output: &str) -> Vec<usize> {
        let desk = self.desk_of(output);
        let pinned = (0..self.dock.len()).filter(|i| self.dock[*i].pinned);
        let local = (0..self.dock.len()).filter(|i| {
            let d = &self.dock[*i];
            !d.pinned && d.output == output && d.desk == desk
        });
        pinned.chain(local).collect()
    }

    /// Entry `i` of the dock out, focused: back where it was, or (pinned, or its place
    /// gone) on the desktop the active monitor shows.
    pub fn undock(&mut self, i: usize) {
        if i >= self.dock.len() {
            return;
        }
        let entry = self.dock.remove(i);
        let output = match entry.pinned {
            false => Some(entry.output.clone()),
            true => self.active_output().map(|o| o.name()),
        };
        let area = self.space.outputs().find(|o| Some(o.name()) == output).and_then(|o| self.output_rect(o));
        let (Some(output), Some(area)) = (output, area) else {
            self.dock.insert(i, entry);
            return;
        };
        let lead = entry.lead();
        // Next to the focused tile of that monitor's strip, when its place is gone.
        let layout_has = |d: &Dune, t: TileId| d.layouts.get(&output).is_some_and(|l| l.contains(t));
        let beside = self.focused.filter(|f| layout_has(self, *f)).map(|f| (f, Side::Right));
        tracing::info!(tiles = ?entry.tiles(), output, pinned = entry.pinned, "out of the dock");
        if entry.pinned {
            self.dock_pins.insert(entry.pin());
        }
        match entry.piece {
            Piece::Tile { id, float, .. } if entry.pinned => self.float_tile(id, &output, back_at(float, area)),
            Piece::Tile { id, float: Some(rect), .. } => self.float_tile(id, &output, keep_inside(rect, area)),
            Piece::Tile { id, home, float: None } => {
                let layout = self.layouts.entry(output.clone()).or_default();
                if !home.is_some_and(|h| layout.restore(id, h)) {
                    layout.insert_column(id, beside.map(|(t, _)| t), Side::Right);
                }
            }
            Piece::Column { column, by } => {
                let layout = self.layouts.entry(output.clone()).or_default();
                let by = if entry.pinned { beside } else { by };
                if let Err(column) = layout.put_column(column, by) {
                    let _ = layout.put_column(column, beside);
                }
            }
        }
        self.focus_tile(lead);
    }

    /// Entry `i` of the dock out (see `undock`), the tab of its column `tab` is in shown.
    pub fn undock_showing(&mut self, i: usize, tab: TileId) {
        if let Some(Docked { piece: Piece::Column { column, .. }, .. }) = self.dock.get_mut(i) {
            column.show(tab);
        }
        self.undock(i);
    }

    /// `dune dock take TILE`: the dock entry holding `id` out (see `undock`), with the tab
    /// `id` is in shown; `id` waiting in the scratchpad floats out of it instead.
    pub fn take_from_dock(&mut self, id: TileId) {
        match self.dock.iter().position(|d| d.tiles().contains(&id)) {
            Some(i) => self.undock_showing(i, id),
            None => self.show_scratchpad(id),
        }
    }

    /// `dune dock pin TILE [OUTPUT]`: pin the dock entry holding `id`, or unpin it into the
    /// dock of the desktop `output` shows (none, or gone: the active monitor).
    pub fn pin_in_dock(&mut self, id: TileId, output: Option<String>) {
        let Some(i) = self.dock.iter().position(|d| d.tiles().contains(&id)) else { return };
        let exists = |o: &String| self.space.outputs().any(|out| out.name() == *o);
        let Some(output) = output.filter(exists).or_else(|| self.active_output().map(|o| o.name())) else { return };
        self.toggle_dock_pin(i, &output);
    }

    /// Pin entry `i` of the dock, or unpin it: then it belongs to the dock of the desktop
    /// `output` shows.
    pub fn toggle_dock_pin(&mut self, i: usize, output: &str) {
        let desk = self.desk_of(output);
        let Some(entry) = self.dock.get_mut(i) else { return };
        entry.pinned = !entry.pinned;
        if !entry.pinned {
            entry.output = output.to_string();
            entry.desk = desk;
        }
        tracing::info!(tile = entry.lead(), pinned = entry.pinned, "dock pin");
        self.refresh_dock();
    }

    /// What bars call entry `entry`: its shown tile's title, with how many
    /// other tabs its column has and a pin when pinned.
    pub fn dock_title(&self, entry: &Docked) -> String {
        let mut title = self.shown_title(entry.lead());
        if entry.tabs() > 1 {
            title = format!("{title} (+{})", entry.tabs() - 1);
        }
        if entry.pinned {
            title = format!("📌 {title}");
        }
        title
    }

    /// The line bars following `output`'s dock (none: the active monitor's) get.
    fn dock_line(&self, output: Option<&str>) -> String {
        let output = output.map(String::from).or_else(|| self.active_output().map(|o| o.name()));
        let entries = output.map(|o| self.dock_entries(&o)).unwrap_or_default();
        status_line(&entries.into_iter().map(|i| self.dock_title(&self.dock[i])).collect::<Vec<_>>(), &self.scratchpad_titles())
    }

    /// A `watch_dock` connection for `output`'s dock: told the state now, and on every change.
    pub fn watch_dock(&mut self, mut stream: UnixStream, output: Option<String>) {
        let line = self.dock_line(output.as_deref());
        if writeln!(stream, "{line}").is_ok() && stream.set_nonblocking(true).is_ok() {
            self.dock_watchers.push((stream, output, line));
        }
    }

    /// After every `arrange`: the watchers get their dock's new state when it changed; those
    /// gone (or not reading) are dropped.
    pub fn refresh_dock(&mut self) {
        let lines: Vec<String> = self.dock_watchers.iter().map(|(_, output, _)| self.dock_line(output.as_deref())).collect();
        let watchers = std::mem::take(&mut self.dock_watchers);
        self.dock_watchers = watchers
            .into_iter()
            .zip(lines)
            .filter_map(|((mut stream, output, last), line)| match line == last {
                true => Some((stream, output, last)),
                false => writeln!(stream, "{line}").ok().map(|_| (stream, output, line)),
            })
            .collect();
    }

    /// `id` leaves the dock (it closed, or went somewhere else).
    pub fn remove_from_dock(&mut self, id: TileId) {
        self.dock.retain_mut(|d| match &mut d.piece {
            Piece::Tile { id: t, .. } => *t != id,
            Piece::Column { column, .. } => !column.tiles().contains(&id) || column.remove(id),
        });
    }

    /// The docks of monitors that went away go to `first`.
    pub fn adopt_docks(&mut self, alive: &[String], first: &str) {
        for entry in self.dock.iter_mut().filter(|d| !alive.contains(&d.output)) {
            entry.output = first.to_string();
        }
    }
}

/// Where a pinned tile out of the dock floats on `area`: where it floated before (`rect`),
/// kept inside it, else in the middle.
fn back_at(rect: Option<Rect>, area: Rect) -> Rect {
    rect.map(|r| keep_inside(r, area)).unwrap_or_else(|| {
        let (w, h) = start_size(area);
        centered(area, w, h)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_line_counts_the_docked_windows() {
        assert_eq!(status_line(&[], &[]), r#"{"text":"0","tooltip":"Dock","class":"empty"}"#);
        let line = status_line(&["📌 vim \"a\"".into(), "htop".into()], &[]);
        assert_eq!(line, r#"{"text":"2","tooltip":"Dock\n📌 vim \"a\"\nhtop","class":"filled"}"#);
        // The scratchpad's windows after them, not counted.
        let line = status_line(&["htop".into()], &["foot".into()]);
        assert_eq!(line, r#"{"text":"1","tooltip":"Dock\nhtop\nScratchpad\nfoot","class":"filled"}"#);
    }

    #[test]
    fn a_pinned_tile_comes_back_inside_the_monitor() {
        let area = Rect::new(0, 0, 1000, 800);
        assert_eq!(back_at(Some(Rect::new(1500, 100, 300, 200)), area), Rect::new(700, 100, 300, 200));
        let r = back_at(None, area);
        assert_eq!((r.x + r.w / 2, r.y + r.h / 2), (500, 400));
    }
}
