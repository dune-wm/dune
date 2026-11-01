//! The event stream of the control socket (`subscribe`, `dune events`), like sway's IPC
//! `subscribe`: a JSON line for every change of the windows, the desktops shown, the
//! monitors and the strip of tiles each monitor shows, and the requests of keys meant for
//! the bar (`minimap`). dune takes a snapshot of them after each turn of its loop and writes what
//! differs from the last one, so no action has to remember to tell. Windows of locked
//! desktops are left out and private ones go by their masked title, as in `windows`.
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
}

impl Kind {
    fn parse(name: &str) -> Option<Kind> {
        match name {
            "window" => Some(Kind::Window),
            "desktop" => Some(Kind::Desktop),
            "output" => Some(Kind::Output),
            "strip" => Some(Kind::Strip),
            "minimap" => Some(Kind::Minimap),
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

/// A tile's rectangle in its strip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub tile: TileId,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
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
}

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
}

impl Event {
    fn kind(&self) -> Kind {
        match self {
            Event::Window { .. } => Kind::Window,
            Event::Desktop { .. } => Kind::Desktop,
            Event::Output { .. } => Kind::Output,
            Event::Strip { .. } => Kind::Strip,
            Event::Minimap { .. } => Kind::Minimap,
        }
    }
}

/// What changed from `old` to `new`, in order: monitors added, windows opened and changed,
/// desktops shown, strips laid out anew, the focus, windows closed, monitors removed.
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
        Snapshot { windows, focused, desks, strips }
    }

    /// A `subscribe` connection: told the state now (as changes from nothing), then every
    /// change. Unknown kinds are left out; none asked for: all.
    pub fn subscribe(&mut self, stream: UnixStream, kinds: &[String]) {
        let mut kinds: Vec<Kind> = kinds.iter().filter_map(|k| Kind::parse(k)).collect();
        if kinds.is_empty() {
            kinds = vec![Kind::Window, Kind::Desktop, Kind::Output, Kind::Strip, Kind::Minimap];
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
    pub fn ask_minimap(&mut self) {
        if let Some(output) = self.active_output() {
            self.asked_events.push(Event::Minimap { change: "toggle", output: output.name() });
        }
    }
}

/// `dune events [KINDS]`: the event stream on stdout, a JSON line per change, until dune
/// goes away. `KINDS`: `window`, `desktop`, `output`, `strip`, `minimap`, comma
/// separated; none: all.
pub fn events_command(args: &[String]) -> Result<(), String> {
    let kinds: Vec<String> = args.iter().flat_map(|a| a.split(',')).map(str::trim).filter(|k| !k.is_empty()).map(String::from).collect();
    if let Some(bad) = kinds.iter().find(|k| Kind::parse(k).is_none()) {
        return Err(format!("unknown event kind `{bad}`; usage: dune events [window,desktop,output,strip,minimap]"));
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
        let event = Event::Desktop { change: "focus", output: "DP-1".into(), desk: 2 };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"desktop","change":"focus","output":"DP-1","desk":2}"#);
        let strip = Strip { view: 4, width: 100, height: 50, tiles: vec![Placed { tile: 3, x: 1, y: 2, w: 30, h: 40 }] };
        let event = Event::Strip { change: "layout", output: "DP-1".into(), strip };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"strip","change":"layout","output":"DP-1","view":4,"width":100,"height":50,"tiles":[{"tile":3,"x":1,"y":2,"w":30,"h":40}]}"#
        );
    }
}
