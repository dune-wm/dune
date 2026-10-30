//! The recording mode (`dune recording menu`, a click on the bar's record button): record a
//! region, the focused tile or the screen, or stop the recording running; and turn
//! recording the microphone on or off, which flips in place and leaves the panel open.
//! Typing narrows them, fuzzy; Enter or a click does one.

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use smithay_client_toolkit::seat::keyboard::{Keysym, Modifiers};

use crate::panel::{Row, View};

const HINT: &str = "Enter: do it · ↑↓ Tab: choose · Esc: close";

/// What a row does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Record {
    Region,
    Tile,
    Screen,
    /// Turn recording the microphone on or off.
    Microphone,
    Stop,
}

impl Record {
    fn label(self) -> &'static str {
        match self {
            Record::Region => "Record a region",
            Record::Tile => "Record the focused tile",
            Record::Screen => "Record the screen",
            Record::Microphone => "Microphone",
            Record::Stop => "Stop recording",
        }
    }

    /// Other words it is found by.
    fn also(self) -> &'static str {
        match self {
            Record::Region => "area select zona grabar",
            Record::Tile => "window ventana grabar",
            Record::Screen => "monitor output pantalla grabar",
            Record::Microphone => "mic audio sound micrófono sonido",
            Record::Stop => "end finish parar detener terminar",
        }
    }

    /// What `dune recording` is asked: `region`, `tile`, `screen`; the microphone and
    /// stopping are asked otherwise (`None`).
    pub fn what(self) -> Option<&'static str> {
        match self {
            Record::Region => Some("region"),
            Record::Tile => Some("tile"),
            Record::Screen => Some("screen"),
            Record::Microphone | Record::Stop => None,
        }
    }
}

/// What a key did in the recording mode.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Continue,
    Close,
    Do(Record),
}

pub struct Recording {
    /// A recording runs: stopping it instead of starting one.
    running: bool,
    /// Recordings take the microphone.
    mic: bool,
    query: String,
    /// Rows matching `query`, best first.
    matches: Vec<Record>,
    selected: usize,
    matcher: Matcher,
}

impl Recording {
    /// From dune's state line (`on SECONDS` or `off`, then ` mic`).
    pub fn new(state: &str) -> Self {
        let mut recording = Recording {
            running: false,
            mic: false,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
            matcher: Matcher::new(Config::DEFAULT),
        };
        recording.set_state(state);
        recording
    }

    /// dune's state line again (the microphone was turned on or off); the row chosen stays.
    pub fn set_state(&mut self, state: &str) {
        self.running = state.starts_with("on");
        self.mic = state.ends_with(" mic");
        let selected = self.matches.get(self.selected).copied();
        self.refilter();
        self.selected = selected.and_then(|r| self.matches.iter().position(|m| *m == r)).unwrap_or(0);
    }

    /// Nothing typed: `?` there lists the keys.
    pub fn nothing_typed(&self) -> bool {
        self.query.is_empty()
    }

    /// The rows there are: stopping while a recording runs, else what to record; the
    /// microphone either way.
    fn all(&self) -> &'static [Record] {
        if self.running {
            &[Record::Stop, Record::Microphone]
        } else {
            &[Record::Region, Record::Tile, Record::Screen, Record::Microphone]
        }
    }

    /// A key pressed: `sym` with `mods`, typing `text`.
    pub fn feed(&mut self, mods: &Modifiers, sym: Keysym, text: Option<char>) -> Step {
        let n = self.matches.len().max(1);
        match sym {
            Keysym::Escape => return Step::Close,
            Keysym::Return | Keysym::KP_Enter => {
                return self.matches.get(self.selected).map_or(Step::Continue, |r| Step::Do(*r));
            }
            Keysym::Down | Keysym::Tab => self.selected = (self.selected + 1) % n,
            Keysym::n if mods.ctrl => self.selected = (self.selected + 1) % n,
            Keysym::Up | Keysym::ISO_Left_Tab => self.selected = (self.selected + n - 1) % n,
            Keysym::p if mods.ctrl => self.selected = (self.selected + n - 1) % n,
            Keysym::BackSpace => {
                self.query.pop();
                self.refilter();
            }
            Keysym::u if mods.ctrl => {
                self.query.clear();
                self.refilter();
            }
            _ => match text {
                Some(c) if !c.is_control() && !mods.ctrl && !mods.alt && !mods.logo => {
                    self.query.push(c);
                    self.refilter();
                }
                _ => {}
            },
        }
        Step::Continue
    }

    /// Row `row` of the list was clicked: do it.
    pub fn click(&mut self, row: usize) -> Step {
        self.matches.get(row).map_or(Step::Continue, |r| Step::Do(*r))
    }

    /// The rows matching the query again, fuzzy over label and other words, best first;
    /// all of them in order with nothing typed.
    fn refilter(&mut self) {
        self.selected = 0;
        if self.query.is_empty() {
            self.matches = self.all().to_vec();
            return;
        }
        let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();
        let mut scored: Vec<(u32, Record)> = self
            .all()
            .iter()
            .filter_map(|r| {
                let haystack = format!("{} {}", r.label(), r.also());
                pattern.score(Utf32Str::new(&haystack, &mut buf), &mut self.matcher).map(|score| (score, *r))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        self.matches = scored.into_iter().map(|(_, r)| r).collect();
    }

    /// What the panel shows.
    pub fn view(&self) -> View<'_> {
        let rows = self
            .matches
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let detail = (*r == Record::Microphone).then(|| if self.mic { "on ✓" } else { "off" }.to_string());
                Row { icon: None, text: r.label(), detail, sub: None, selected: i == self.selected }
            })
            .collect();
        View {
            prompt: format!("record {}▏", self.query),
            rows,
            placeholder: "",
            note: String::new(),
            side: None,
            hint: HINT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> Modifiers {
        Modifiers::default()
    }

    fn texts(r: &Recording) -> Vec<&str> {
        r.view().rows.iter().map(|row| row.text).collect()
    }

    #[test]
    fn idle_it_offers_what_to_record_and_the_microphone() {
        let mut r = Recording::new("off");
        assert_eq!(texts(&r), ["Record a region", "Record the focused tile", "Record the screen", "Microphone"]);
        assert_eq!(r.view().rows[3].detail.as_deref(), Some("off"));
        assert_eq!(r.feed(&none(), Keysym::Return, None), Step::Do(Record::Region));
        assert_eq!(r.feed(&none(), Keysym::Down, None), Step::Continue);
        assert_eq!(r.feed(&none(), Keysym::Return, None), Step::Do(Record::Tile));
        assert_eq!(r.click(2), Step::Do(Record::Screen));
        assert_eq!(r.click(9), Step::Continue);
        assert_eq!(r.feed(&none(), Keysym::Escape, None), Step::Close);
        assert_eq!((Record::Tile.what(), Record::Microphone.what()), (Some("tile"), None));
    }

    #[test]
    fn running_it_offers_to_stop() {
        let r = Recording::new("on 1700000000 mic");
        assert_eq!(texts(&r), ["Stop recording", "Microphone"]);
        assert_eq!(r.view().rows[1].detail.as_deref(), Some("on ✓"));
    }

    #[test]
    fn the_microphone_flips_in_place() {
        let mut r = Recording::new("off");
        for _ in 0..3 {
            r.feed(&none(), Keysym::Down, None);
        }
        assert_eq!(r.feed(&none(), Keysym::Return, None), Step::Do(Record::Microphone));
        r.set_state("off mic");
        assert_eq!(r.view().rows[3].detail.as_deref(), Some("on ✓"));
        assert!(r.view().rows[3].selected);
    }

    #[test]
    fn typing_narrows_them_in_either_language() {
        let mut r = Recording::new("off");
        for c in "pantalla".chars() {
            r.feed(&none(), Keysym::NoSymbol, Some(c));
        }
        assert_eq!(r.feed(&none(), Keysym::Return, None), Step::Do(Record::Screen));
        let mut r = Recording::new("off");
        for c in "mic".chars() {
            r.feed(&none(), Keysym::NoSymbol, Some(c));
        }
        assert_eq!(r.feed(&none(), Keysym::Return, None), Step::Do(Record::Microphone));
    }
}
