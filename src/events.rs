//! The event stream of the control socket (`subscribe`, `dune events`), like sway's IPC
//! `subscribe`: a JSON line for every change of the windows, the desktops shown and the
//! monitors. dune takes a snapshot of them after each turn of its loop and writes what
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
}

impl Kind {
    fn parse(name: &str) -> Option<Kind> {
        match name {
            "window" => Some(Kind::Window),
            "desktop" => Some(Kind::Desktop),
            "output" => Some(Kind::Output),
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

/// What dune tells about, at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub windows: BTreeMap<TileId, Window>,
    /// The focused window, when it is one of `windows`.
    pub focused: Option<TileId>,
    /// The desktop each monitor shows.
    pub desks: BTreeMap<String, u8>,
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
}

impl Event {
    fn kind(&self) -> Kind {
        match self {
            Event::Window { .. } => Kind::Window,
            Event::Desktop { .. } => Kind::Desktop,
            Event::Output { .. } => Kind::Output,
        }
    }
}

/// What changed from `old` to `new`, in order: monitors added, windows opened and changed,
/// desktops shown, the focus, windows closed, monitors removed.
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
        Snapshot { windows, focused, desks }
    }

    /// A `subscribe` connection: told the state now (as changes from nothing), then every
    /// change. Unknown kinds are left out; none asked for: all.
    pub fn subscribe(&mut self, stream: UnixStream, kinds: &[String]) {
        let mut kinds: Vec<Kind> = kinds.iter().filter_map(|k| Kind::parse(k)).collect();
        if kinds.is_empty() {
            kinds = vec![Kind::Window, Kind::Desktop, Kind::Output];
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
        if self.subscribers.is_empty() {
            return;
        }
        let now = self.snapshot();
        if now == self.last_snapshot {
            return;
        }
        let events = diff(&self.last_snapshot, &now);
        self.subscribers.retain_mut(|s| s.tell(&events));
        self.last_snapshot = now;
    }
}

/// `dune events [KINDS]`: the event stream on stdout, a JSON line per change, until dune
/// goes away. `KINDS`: `window`, `desktop`, `output`, comma separated; none: all.
pub fn events_command(args: &[String]) -> Result<(), String> {
    let kinds: Vec<String> = args.iter().flat_map(|a| a.split(',')).map(str::trim).filter(|k| !k.is_empty()).map(String::from).collect();
    if let Some(bad) = kinds.iter().find(|k| Kind::parse(k).is_none()) {
        return Err(format!("unknown event kind `{bad}`; usage: dune events [window,desktop,output]"));
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
        }
    }

    fn changes(events: &[Event]) -> Vec<String> {
        events
            .iter()
            .map(|e| match e {
                Event::Window { change, window } => format!("window {change} {}", window.tile),
                Event::Desktop { output, desk, .. } => format!("desktop {output} {desk}"),
                Event::Output { change, output } => format!("output {change} {output}"),
            })
            .collect()
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
    fn events_are_json_lines() {
        let event = Event::Window { change: "new", window: window(3, "a \"b\"", "desk 1") };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"event":"window","change":"new","tile":3,"app_id":"foot","title":"a \"b\"","place":"desk 1"}"#
        );
        let event = Event::Desktop { change: "focus", output: "DP-1".into(), desk: 2 };
        assert_eq!(serde_json::to_string(&event).unwrap(), r#"{"event":"desktop","change":"focus","output":"DP-1","desk":2}"#);
    }
}
