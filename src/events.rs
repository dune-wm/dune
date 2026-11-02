//! The event stream of the control socket (`subscribe`, `dune events`), like sway's IPC
//! `subscribe`: a JSON line for every change of the windows, the desktops shown, the
//! monitors, the strip of tiles and the dock each monitor shows, and the requests of keys
//! meant for the bar (`minimap`, the dock's `toggle`), Mod held alone (`mod`), and the mouse's clicks (`pointer`:
//! where on which monitor, for dune-eye-tracker to learn where you look). dune takes a snapshot of them after
//! each turn of its loop and writes what differs from the last one, so no action has to
//! remember to tell. Windows of locked desktops are left out and private ones go by their
//! masked title, as in `windows`.
//!
//! ```text
//! {"event":"window","change":"new","tile":3,"app_id":"foot","title":"~","place":"desk 1"}
//! {"event":"window","change":"title","tile":3,"app_id":"foot","title":"vim","place":"desk 1"}
//! {"event":"window","change":"move","tile":3,"app_id":"foot","title":"vim","place":"desk 2"}
//! {"event":"window","change":"focus","tile":3,"app_id":"foot","title":"vim","place":"desk 2"}
//! {"event":"window","change":"close","tile":3,"app_id":"foot","title":"vim","place":"desk 2"}
//! {"event":"desktop","change":"focus","output":"DP-1","desk":2}
//! {"event":"output","change":"added","output":"DP-1"}
//! {"event":"strip","change":"layout","output":"DP-1","view":0,"width":1920,"height":1080,"tiles":[{"tile":3,"x":8,"y":8,"w":948,"h":1064}]}
//! {"event":"minimap","change":"toggle","output":"DP-1"}
//! {"event":"dock","change":"entries","output":"DP-1","entries":[{"tile":3,"app_id":"foot","title":"vim","pinned":false,"tabs":[],"sound":false}],"scratchpad":[{"tile":7,"app_id":"foot","title":"~","sound":false}]}
//! {"event":"dock","change":"toggle","output":"DP-1"}
//! {"event":"mod","change":"start"}
//! {"event":"pointer","change":"click","button":"left","x":812,"y":440,"output":"DP-1","tile":3,"app_id":"foot","tile_x":200,"tile_y":96,"tile_w":1268,"tile_h":1064}
//! ```

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
};

use dune_common::protocol::{Request, watch};
use serde::Serialize;

use crate::{Dune, layout::TileId};

/// What a subscriber hears about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Window,
    Desktop,
    Output,
    Strip,
    /// The `minimap` key: show or hide the minimap of that monitor (dune-shell draws it).
    Minimap,
    /// Each monitor's dock, and the `dock_show` key: show or hide it (dune-shell draws it).
    Dock,
    /// Mod or Shift held alone: `start`, `end`, `cancel` (`crate::mod_hold`; dune-voice listens, or dictates).
    Mod,
    /// A mouse button pressed (or `dune pointer click`): which, and where on which monitor.
    /// Only when asked for, like `mod`; never while locked.
    Pointer,
    /// The `zoom` key: `in` (the center, the area magnified, the scale) or `out`.
    Zoom,
}

impl Kind {
    fn parse(name: &str) -> Option<Kind> {
        match name {
            "window" => Some(Kind::Window),
            "desktop" => Some(Kind::Desktop),
            "output" => Some(Kind::Output),
            "strip" => Some(Kind::Strip),
            "minimap" => Some(Kind::Minimap),
            "dock" => Some(Kind::Dock),
            "mod" => Some(Kind::Mod),
            "pointer" => Some(Kind::Pointer),
            "zoom" => Some(Kind::Zoom),
            _ => None,
        }
    }
}

/// A window as subscribers see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Window {
    pub tile: TileId,
    pub app_id: String,
    pub title: String,
    pub place: String,
}

/// The strip of tiles a monitor shows, in pixels: each tile's rectangle along the strip,
/// and the part of it on screen, from `view` to `view + width`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Strip {
    pub view: i32,
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Placed>,
}

/// The tile a click landed on (dune-eye-tracker learns where clicks go inside each app's
/// window, which stays put while the strip slides).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OnTile {
    pub tile: TileId,
    pub app_id: String,
    pub tile_x: i32,
    pub tile_y: i32,
    pub tile_w: i32,
    pub tile_h: i32,
}

/// A tile's rectangle in its strip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub tile: TileId,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The dock a monitor shows (`Dune::dock_entries`, the pinned entries first) and the
/// scratchpad's windows, the same on every monitor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Dock {
    pub entries: Vec<DockEntry>,
    pub scratchpad: Vec<DockWindow>,
}

/// A dock entry: its lead (the shown tab of a column), whether pinned, and the first tile of
/// each of its column's tabs (none for a tile, or a column of one tab).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DockEntry {
    pub tile: TileId,
    pub app_id: String,
    pub title: String,
    pub pinned: bool,
    pub tabs: Vec<DockWindow>,
    /// One of its windows plays sound (`crate::sound`).
    pub sound: bool,
}

/// A window of the dock: a tab of a docked column, or one waiting in the scratchpad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DockWindow {
    pub tile: TileId,
    pub app_id: String,
    pub title: String,
    pub sound: bool,
}

/// What dune tells about, at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub windows: BTreeMap<TileId, Window>,
    /// The focused window, when it is one of `windows`.
    pub focused: Option<TileId>,
    /// The desktop each monitor shows.
    pub desks: BTreeMap<String, u8>,
    /// The strip each monitor shows (of the windows in `windows`).
    pub strips: BTreeMap<String, Strip>,
    /// The dock each monitor shows.
    pub docks: BTreeMap<String, Dock>,
}

/// How often the mouse's motion is told at most (`Dune::tell_motion`).
const MOTION_EVERY: std::time::Duration = std::time::Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum Event {
    Window {
        change: &'static str,
        #[serde(flatten)]
        window: Window,
    },
    Desktop {
        change: &'static str,
        output: String,
        desk: u8,
    },
    Output {
        change: &'static str,
        output: String,
    },
    Strip {
        change: &'static str,
        output: String,
        #[serde(flatten)]
        strip: Strip,
    },
    Minimap {
        change: &'static str,
        output: String,
    },
    Dock {
        change: &'static str,
        output: String,
        #[serde(flatten)]
        dock: Dock,
    },
    /// The `dock_show` key, or `dune dock show`.
    #[serde(rename = "dock")]
    DockToggle {
        change: &'static str,
        output: String,
    },
    Mod {
        change: &'static str,
        /// `mod` or `shift`: what is held alone.
        keys: &'static str,
    },
    Pointer {
        /// `click` (a button pressed) or `move` (the mouse moved it, `MOTION_EVERY` at most).
        change: &'static str,
        /// `left`, `right`, `middle`; none for `move`.
        #[serde(skip_serializing_if = "str::is_empty")]
        button: &'static str,
        /// From the top-left corner of `output`, in logical pixels.
        x: i32,
        y: i32,
        output: String,
        /// A click on a tile: which, its app, where in it (from its top-left corner) and its
        /// size, all logical pixels. None for `move`, off any tile, or on a locked desk.
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        on: Option<OnTile>,
        /// A click while zoomed: where it was seen (the eyes were there, not at `x` `y`).
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        seen: Option<Seen>,
    },
    Zoom {
        /// `in` or `out`.
        change: &'static str,
        output: String,
        /// `in`: what is magnified, real coordinates from the monitor's corner.
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        zoom: Option<ZoomIn>,
    },
}

/// Where a click made while zoomed was seen on the monitor, and how magnified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Seen {
    pub zoom_x: i32,
    pub zoom_y: i32,
    pub zoom_scale: u32,
}

/// A zoom going in: around (`x`, `y`), magnified `scale` times, drawn inside the area (the
/// tile's rectangle, `tile`; the whole monitor without one), all from the monitor's corner:
/// a point p is seen at center + (p - center)·scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZoomIn {
    pub x: i32,
    pub y: i32,
    pub scale: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile: Option<TileId>,
    pub area_x: i32,
    pub area_y: i32,
    pub area_w: i32,
    pub area_h: i32,
}

impl Event {
    fn kind(&self) -> Kind {
        match self {
            Event::Window { .. } => Kind::Window,
            Event::Desktop { .. } => Kind::Desktop,
            Event::Output { .. } => Kind::Output,
            Event::Strip { .. } => Kind::Strip,
            Event::Minimap { .. } => Kind::Minimap,
            Event::Dock { .. } | Event::DockToggle { .. } => Kind::Dock,
            Event::Mod { .. } => Kind::Mod,
            Event::Pointer { .. } => Kind::Pointer,
            Event::Zoom { .. } => Kind::Zoom,
        }
    }
}

/// What changed from `old` to `new`, in order: monitors added, windows opened and changed,
/// desktops shown, strips laid out anew, docks changed, the focus, windows closed, monitors
/// removed.
pub fn diff(old: &Snapshot, new: &Snapshot) -> Vec<Event> {
    let mut events = Vec::new();
    let outputs = |s: &Snapshot| s.desks.keys().cloned().collect::<BTreeSet<_>>();
    let (old_outputs, new_outputs) = (outputs(old), outputs(new));
    for output in new_outputs.difference(&old_outputs) {
        events.push(Event::Output { change: "added", output: output.clone() });
    }
    for (tile, window) in &new.windows {
        let change = match old.windows.get(tile) {
            None => "new",
            Some(was) if was.place != window.place => "move",
            Some(was) if was.title != window.title || was.app_id != window.app_id => "title",
            Some(_) => continue,
        };
        events.push(Event::Window { change, window: window.clone() });
    }
    for (output, desk) in &new.desks {
        if old.desks.get(output) != Some(desk) {
            events.push(Event::Desktop { change: "focus", output: output.clone(), desk: *desk });
        }
    }
    for (output, strip) in &new.strips {
        if old.strips.get(output) != Some(strip) {
            events.push(Event::Strip { change: "layout", output: output.clone(), strip: strip.clone() });
        }
    }
    for (output, dock) in &new.docks {
        if old.docks.get(output) != Some(dock) {
            events.push(Event::Dock { change: "entries", output: output.clone(), dock: dock.clone() });
        }
    }
    if new.focused != old.focused
        && let Some(window) = new.focused.and_then(|t| new.windows.get(&t))
    {
        events.push(Event::Window { change: "focus", window: window.clone() });
    }
    for (tile, window) in &old.windows {
        if !new.windows.contains_key(tile) {
            events.push(Event::Window { change: "close", window: window.clone() });
        }
    }
    for output in old_outputs.difference(&new_outputs) {
        events.push(Event::Output { change: "removed", output: output.clone() });
    }
    events
}

/// A `subscribe` connection and the kinds it asked for.
pub struct Subscriber {
    stream: UnixStream,
    kinds: Vec<Kind>,
}

impl Subscriber {
    /// Write the events it asked for; `false` once it is gone (or not reading).
    fn tell(&mut self, events: &[Event]) -> bool {
        events.iter().filter(|e| self.kinds.contains(&e.kind())).all(|e| match serde_json::to_string(e) {
            Ok(line) => writeln!(self.stream, "{line}").is_ok(),
            Err(_) => true,
        })
    }
}

impl Dune {
    /// The snapshot subscribers are told the changes of.
    fn snapshot(&self) -> Snapshot {
        let windows: BTreeMap<TileId, Window> = self
            .window_list(false)
            .into_iter()
            .map(|w| (w.tile, Window { tile: w.tile, app_id: w.app, title: w.title, place: w.place }))
            .collect();
        let focused = self.focused.filter(|t| windows.contains_key(t));
        let desks = self.space.outputs().map(|o| (o.name(), self.desk_of(&o.name()))).collect();
        let strips = self
            .space
            .outputs()
            .filter_map(|output| {
                let area = self.output_rect(output)?;
                let layout = self.layouts.get(&output.name())?;
                let view = layout.view();
                // Laid out in output coordinates, the view slid off: back to the strip's.
                let tiles = layout
                    .tiles()
                    .into_iter()
                    .filter(|t| windows.contains_key(t))
                    .filter_map(|tile| {
                        let (_, rect) = self.geometries.iter().find(|(t, _)| *t == tile)?;
                        Some(Placed { tile, x: rect.x - area.x + view, y: rect.y - area.y, w: rect.w, h: rect.h })
                    })
                    .collect();
                Some((output.name(), Strip { view, width: area.w, height: area.h, tiles }))
            })
            .collect();
        let scratchpad: Vec<DockWindow> = self.scratchpad_tiles().into_iter().map(|t| self.dock_window(t)).collect();
        let docks = self
            .space
            .outputs()
            .map(|output| {
                let entries = self
                    .dock_entries(&output.name())
                    .into_iter()
                    .map(|i| {
                        let entry = &self.dock[i];
                        let lead = self.dock_window(entry.lead());
                        let tabs = if entry.tabs() > 1 { entry.leads().into_iter().map(|t| self.dock_window(t)).collect() } else { Vec::new() };
                        let sound = entry.tiles().into_iter().any(|t| self.plays_sound(t));
                        DockEntry { tile: lead.tile, app_id: lead.app_id, title: lead.title, pinned: entry.pinned, tabs, sound }
                    })
                    .collect();
                (output.name(), Dock { entries, scratchpad: scratchpad.clone() })
            })
            .collect();
        Snapshot { windows, focused, desks, strips, docks }
    }

    /// `id` as the dock shows it: by its masked title when private.
    fn dock_window(&self, id: TileId) -> DockWindow {
        DockWindow { tile: id, app_id: self.tile_app_id(id).unwrap_or_default(), title: self.shown_title(id), sound: self.plays_sound(id) }
    }

    /// A `subscribe` connection: told the state now (as changes from nothing), then every
    /// change. Unknown kinds are left out; none asked for: all.
    pub fn subscribe(&mut self, stream: UnixStream, kinds: &[String]) {
        let mut kinds: Vec<Kind> = kinds.iter().filter_map(|k| Kind::parse(k)).collect();
        if kinds.is_empty() {
            kinds = vec![Kind::Window, Kind::Desktop, Kind::Output, Kind::Strip, Kind::Minimap, Kind::Dock];
        }
        if self.subscribers.is_empty() {
            self.last_snapshot = self.snapshot();
        }
        let mut subscriber = Subscriber { stream, kinds };
        if subscriber.tell(&diff(&Snapshot::default(), &self.last_snapshot)) && subscriber.stream.set_nonblocking(true).is_ok() {
            self.subscribers.push(subscriber);
        }
    }

    /// After every turn of the loop: subscribers hear what changed since the last one;
    /// those gone are dropped. Nothing is looked at while nobody listens.
    pub fn send_events(&mut self) {
        let asked = std::mem::take(&mut self.asked_events);
        if self.subscribers.is_empty() {
            return;
        }
        let now = self.snapshot();
        let mut events = if now == self.last_snapshot { Vec::new() } else { diff(&self.last_snapshot, &now) };
        events.extend(asked);
        if events.is_empty() {
            return;
        }
        self.subscribers.retain_mut(|s| s.tell(&events));
        self.last_snapshot = now;
    }

    /// The `minimap` key: subscribers (dune-shell) hear it with the active monitor, after
    /// this turn of the loop. Nobody listening, it does nothing.
    /// A mouse button pressed at `pos` (layout coordinates): subscribers of `pointer`
    /// hear which, and where on the monitor there. Nobody listening, it does nothing.
    pub fn tell_click(&mut self, button: u32, pos: smithay::utils::Point<f64, smithay::utils::Logical>) {
        let name = match button {
            crate::seatop::BTN_LEFT => "left",
            crate::seatop::BTN_RIGHT => "right",
            crate::input::BTN_MIDDLE => "middle",
            _ => return,
        };
        if !self.subscribers.iter().any(|s| s.kinds.contains(&Kind::Pointer)) {
            return;
        }
        let Some(output) = self.space.outputs().find(|o| self.space.output_geometry(o).is_some_and(|g| g.to_f64().contains(pos))) else { return };
        let Some(geo) = self.space.output_geometry(output) else { return };
        let (x, y) = ((pos.x - geo.loc.x as f64).round() as i32, (pos.y - geo.loc.y as f64).round() as i32);
        let on = self.on_tile(pos);
        let seen = self.zoom_of(&output.name()).map(|z| {
            let at = z.seen((pos.x - geo.loc.x as f64, pos.y - geo.loc.y as f64).into());
            Seen { zoom_x: at.x.round() as i32, zoom_y: at.y.round() as i32, zoom_scale: z.scale }
        });
        self.asked_events.push(Event::Pointer { change: "click", button: name, x, y, output: output.name(), on, seen });
    }

    /// The tile at `pos` (layout coordinates) and where in it, for a click's event: a tile
    /// listed to subscribers (not on a locked desk), floats too.
    fn on_tile(&self, pos: smithay::utils::Point<f64, smithay::utils::Logical>) -> Option<OnTile> {
        let tile = self.tab_at(pos.x, pos.y).or_else(|| self.tile_at(pos.x, pos.y))?;
        let rect = self.shown.iter().find(|(t, _)| *t == tile).map(|(_, r)| *r)?;
        if !self.window_list(false).iter().any(|w| w.tile == tile) {
            return None;
        }
        Some(OnTile {
            tile,
            app_id: self.tile_app_id(tile).unwrap_or_default(),
            tile_x: (pos.x - rect.x as f64).round() as i32,
            tile_y: (pos.y - rect.y as f64).round() as i32,
            tile_w: rect.w,
            tile_h: rect.h,
        })
    }

    /// The mouse (or touchpad) moved the pointer to `pos`, not `dune pointer`: subscribers of
    /// `pointer` hear where, `MOTION_EVERY` at most (dune-eye-tracker leaves the pointer to
    /// the hand meanwhile). Nobody listening, it does nothing.
    pub fn tell_motion(&mut self, pos: smithay::utils::Point<f64, smithay::utils::Logical>) {
        if !self.subscribers.iter().any(|s| s.kinds.contains(&Kind::Pointer)) {
            return;
        }
        let now = std::time::Instant::now();
        if self.told_motion.is_some_and(|t| now.duration_since(t) < MOTION_EVERY) {
            return;
        }
        let Some(output) = self.space.outputs().find(|o| self.space.output_geometry(o).is_some_and(|g| g.to_f64().contains(pos))) else { return };
        let Some(geo) = self.space.output_geometry(output) else { return };
        let (x, y) = ((pos.x - geo.loc.x as f64).round() as i32, (pos.y - geo.loc.y as f64).round() as i32);
        self.told_motion = Some(now);
        self.asked_events.push(Event::Pointer { change: "move", button: "", x, y, output: output.name(), on: None, seen: None });
    }

    /// The zoom going in (`Some`) or out: subscribers of `zoom` (dune-eye-tracker maps the
    /// gaze back to real coordinates while it is in) hear it.
    pub fn tell_zoom(&mut self, going_in: bool, z: &crate::zoom::Zoom) {
        let (change, output) = (if going_in { "in" } else { "out" }, z.output.clone());
        let zoom = going_in.then(|| ZoomIn {
            x: z.center.x.round() as i32,
            y: z.center.y.round() as i32,
            scale: z.scale,
            tile: z.tile,
            area_x: z.area.x,
            area_y: z.area.y,
            area_w: z.area.w,
            area_h: z.area.h,
        });
        self.asked_events.push(Event::Zoom { change, output, zoom });
    }

    pub fn ask_minimap(&mut self) {
        if let Some(output) = self.active_output() {
            self.asked_events.push(Event::Minimap { change: "toggle", output: output.name() });
        }
    }

    /// The `dock_show` key, or `dune dock show`: subscribers (dune-shell) hear it with
    /// `output` (none, or gone: the active monitor), after this turn of the loop. Nobody
    /// listening, it does nothing.
    pub fn ask_dock(&mut self, output: Option<String>) {
        let exists = |o: &String| self.space.outputs().any(|out| out.name() == *o);
        if let Some(output) = output.filter(exists).or_else(|| self.active_output().map(|o| o.name())) {
            self.asked_events.push(Event::DockToggle { change: "toggle", output });
        }
    }
}

/// `dune events [KINDS]`: the event stream on stdout, a JSON line per change, until dune
/// goes away. `KINDS`: `window`, `desktop`, `output`, `strip`, `minimap`, `dock`, `mod`,
/// `pointer`, `zoom`, comma separated; none: all but `mod`, `pointer` and `zoom`.
pub fn events_command(args: &[String]) -> Result<(), String> {
    let kinds: Vec<String> = args.iter().flat_map(|a| a.split(',')).map(str::trim).filter(|k| !k.is_empty()).map(String::from).collect();
    if let Some(bad) = kinds.iter().find(|k| Kind::parse(k).is_none()) {
        return Err(format!("unknown event kind `{bad}`; usage: dune events [window,desktop,output,strip,minimap,dock,mod,pointer,zoom]"));
    }
    let stream = watch(&Request::Subscribe { kinds })?;
    let mut out = std::io::stdout().lock();
    for line in BufReader::new(stream).lines() {
        let line = line.map_err(|e| e.to_string())?;
        writeln!(out, "{line}").and_then(|_| out.flush()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(tile: TileId, title: &str, place: &str) -> Window {
        Window { tile, app_id: "foot".into(), title: title.into(), place: place.into() }
    }

    fn snapshot(windows: &[Window], focused: Option<TileId>, desks: &[(&str, u8)]) -> Snapshot {
        Snapshot {
            windows: windows.iter().map(|w| (w.tile, w.clone())).collect(),
            focused,
            desks: desks.iter().map(|(o, d)| (o.to_string(), *d)).collect(),
            strips: BTreeMap::new(),
            docks: BTreeMap::new(),
        }
    }

    fn changes(events: &[Event]) -> Vec<String> {
        events
            .iter()
            .map(|e| match e {
                Event::Window { change, window } => format!("window {change} {}", window.tile),
                Event::Desktop { output, desk, .. } => format!("desktop {output} {desk}"),
                Event::Output { change, output } => format!("output {change} {output}"),
                Event::Strip { output, strip, .. } => format!("strip {output} {}", strip.view),
                Event::Minimap { change, output } => format!("minimap {change} {output}"),
                Event::Dock { change, output, dock } => format!("dock {change} {output} {}", dock.entries.len()),
                Event::DockToggle { change, output } => format!("dock {change} {output}"),
                Event::Mod { change, keys } => format!("mod {change} {keys}"),
                Event::Pointer { change, button, x, y, output, .. } => format!("pointer {change} {button} {x} {y} {output}"),
                Event::Zoom { change, output, .. } => format!("zoom {change} {output}"),
            })
            .collect()
    }

    #[test]
    fn the_minimap_line() {
        let event = Event::Minimap { change: "toggle", output: "DP-1".into() };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"minimap","change":"toggle","output":"DP-1"}"#);
        assert_eq!(event.kind(), Kind::Minimap);
        assert_eq!(Kind::parse("minimap"), Some(Kind::Minimap));
    }

    #[test]
    fn the_dock_toggle_line() {
        let event = Event::DockToggle { change: "toggle", output: "DP-1".into() };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"dock","change":"toggle","output":"DP-1"}"#);
        assert_eq!(event.kind(), Kind::Dock);
        assert_eq!(Kind::parse("dock"), Some(Kind::Dock));
    }

    fn dock_window(tile: TileId, title: &str) -> DockWindow {
        DockWindow { tile, app_id: "foot".into(), title: title.into(), sound: false }
    }

    #[test]
    fn docks_are_told_when_they_change() {
        let entry = |pinned| DockEntry { tile: 3, app_id: "foot".into(), title: "vim".into(), pinned, tabs: vec![], sound: false };
        let dock = |pinned| Dock { entries: vec![entry(pinned)], scratchpad: vec![] };
        let mut old = snapshot(&[window(1, "~", "desk 1")], Some(1), &[("DP-1", 1), ("HDMI-A-1", 1)]);
        old.docks.insert("DP-1".into(), dock(false));
        old.docks.insert("HDMI-A-1".into(), Dock::default());
        let mut new = old.clone();
        assert_eq!(diff(&old, &new), []);
        new.docks.insert("DP-1".into(), dock(true));
        assert_eq!(changes(&diff(&old, &new)), ["dock entries DP-1 1"]);
        // From nothing, every monitor's dock, the empty ones too.
        assert_eq!(
            changes(&diff(&Snapshot::default(), &old)),
            ["output added DP-1", "output added HDMI-A-1", "window new 1", "desktop DP-1 1", "desktop HDMI-A-1 1", "dock entries DP-1 1", "dock entries HDMI-A-1 0", "window focus 1"]
        );
    }

    #[test]
    fn from_nothing_everything_is_new() {
        let now = snapshot(&[window(1, "~", "desk 1"), window(2, "vim", "desk 2")], Some(1), &[("DP-1", 1)]);
        assert_eq!(
            changes(&diff(&Snapshot::default(), &now)),
            ["output added DP-1", "window new 1", "window new 2", "desktop DP-1 1", "window focus 1"]
        );
    }

    #[test]
    fn changes_between_snapshots() {
        let old = snapshot(&[window(1, "~", "desk 1"), window(2, "vim", "desk 1"), window(3, "top", "desk 1")], Some(1), &[("DP-1", 1)]);
        let new = snapshot(&[window(1, "~/src", "desk 1"), window(2, "vim", "desk 2"), window(4, "~", "desk 1")], Some(4), &[("DP-1", 2), ("HDMI-A-1", 1)]);
        assert_eq!(
            changes(&diff(&old, &new)),
            [
                "output added HDMI-A-1",
                "window title 1",
                "window move 2",
                "window new 4",
                "desktop DP-1 2",
                "desktop HDMI-A-1 1",
                "window focus 4",
                "window close 3",
            ]
        );
        assert_eq!(diff(&new, &new), []);
    }

    #[test]
    fn strips_are_told_when_they_change() {
        let strip = |view| Strip { view, width: 1000, height: 600, tiles: vec![Placed { tile: 1, x: 0, y: 0, w: 1500, h: 600 }] };
        let mut old = snapshot(&[window(1, "~", "desk 1")], Some(1), &[("DP-1", 1)]);
        old.strips.insert("DP-1".into(), strip(0));
        let mut new = old.clone();
        assert_eq!(diff(&old, &new), []);
        new.strips.insert("DP-1".into(), strip(500));
        assert_eq!(changes(&diff(&old, &new)), ["strip DP-1 500"]);
        assert_eq!(changes(&diff(&Snapshot::default(), &old)), ["output added DP-1", "window new 1", "desktop DP-1 1", "strip DP-1 0", "window focus 1"]);
    }

    #[test]
    fn events_are_json_lines() {
        let event = Event::Window { change: "new", window: window(3, "a \"b\"", "desk 1") };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"window","change":"new","tile":3,"app_id":"foot","title":"a \"b\"","place":"desk 1"}"#
        );
        let event = Event::Pointer { change: "click", button: "left", x: 812, y: 440, output: "DP-1".into(), on: None, seen: None };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"pointer","change":"click","button":"left","x":812,"y":440,"output":"DP-1"}"#
        );
        // while zoomed: where it was seen too
        let seen = Some(Seen { zoom_x: 860, zoom_y: 370, zoom_scale: 3 });
        let event = Event::Pointer { change: "click", button: "left", x: 820, y: 390, output: "DP-1".into(), on: None, seen };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"pointer","change":"click","button":"left","x":820,"y":390,"output":"DP-1","zoom_x":860,"zoom_y":370,"zoom_scale":3}"#
        );
        assert_eq!(Kind::parse("zoom"), Some(Kind::Zoom));
        let zoom = Some(ZoomIn { x: 800, y: 400, scale: 3, tile: Some(3), area_x: 7, area_y: 7, area_w: 1268, area_h: 1064 });
        let event = Event::Zoom { change: "in", output: "DP-1".into(), zoom };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"zoom","change":"in","output":"DP-1","x":800,"y":400,"scale":3,"tile":3,"area_x":7,"area_y":7,"area_w":1268,"area_h":1064}"#
        );
        let event = Event::Zoom { change: "out", output: "DP-1".into(), zoom: None };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"zoom","change":"out","output":"DP-1"}"#);
        let on = OnTile { tile: 3, app_id: "foot".into(), tile_x: 200, tile_y: 96, tile_w: 1268, tile_h: 1064 };
        let event = Event::Pointer { change: "click", button: "left", x: 812, y: 440, output: "DP-1".into(), on: Some(on), seen: None };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"pointer","change":"click","button":"left","x":812,"y":440,"output":"DP-1","tile":3,"app_id":"foot","tile_x":200,"tile_y":96,"tile_w":1268,"tile_h":1064}"#
        );
        assert_eq!(Kind::parse("pointer"), Some(Kind::Pointer));
        let event = Event::Pointer { change: "move", button: "", x: 5, y: 6, output: "DP-1".into(), on: None, seen: None };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"pointer","change":"move","x":5,"y":6,"output":"DP-1"}"#);
        let event = Event::Desktop { change: "focus", output: "DP-1".into(), desk: 2 };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"desktop","change":"focus","output":"DP-1","desk":2}"#);
        let strip = Strip { view: 4, width: 100, height: 50, tiles: vec![Placed { tile: 3, x: 1, y: 2, w: 30, h: 40 }] };
        let event = Event::Strip { change: "layout", output: "DP-1".into(), strip };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"strip","change":"layout","output":"DP-1","view":4,"width":100,"height":50,"tiles":[{"tile":3,"x":1,"y":2,"w":30,"h":40}]}"#
        );
        let tabs = vec![dock_window(3, "vim"), DockWindow { sound: true, ..dock_window(5, "htop") }];
        let entry = DockEntry { tile: 3, app_id: "foot".into(), title: "vim".into(), pinned: false, tabs, sound: true };
        let dock = Dock { entries: vec![entry], scratchpad: vec![dock_window(7, "~")] };
        let event = Event::Dock { change: "entries", output: "DP-1".into(), dock };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"dock","change":"entries","output":"DP-1","entries":[{"tile":3,"app_id":"foot","title":"vim","pinned":false,"tabs":[{"tile":3,"app_id":"foot","title":"vim","sound":false},{"tile":5,"app_id":"foot","title":"htop","sound":true}],"sound":true}],"scratchpad":[{"tile":7,"app_id":"foot","title":"~","sound":false}]}"#
        );
    }
}
