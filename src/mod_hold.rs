//! Mod, or Shift, held alone, with no other key or button, for `HOLD`: subscribers hear
//! `mod` `start` (`dune events mod`) with the `keys` held; let go, `end`; another key, a
//! button or the wheel meanwhile, `cancel` (it was a shortcut, a capital letter or a
//! sideways scroll, after all).
//! dune-voice listens for a phrase while Mod is held, and dictates into the focused app
//! while Shift is. Mod only for a `modifier` that is one key (`super`, `alt`); neither
//! while the session is locked or a window holds the keys.
//!
//! ```text
//! {"event":"mod","change":"start","keys":"mod"}
//! {"event":"mod","change":"end","keys":"mod"}
//! {"event":"mod","change":"start","keys":"shift"}
//! ```

use std::time::Duration;

use smithay::{
    input::keyboard::{Keysym, ModifiersState},
    reexports::calloop::timer::{TimeoutAction, Timer},
};

use crate::{Dune, events::Event};

/// How long a key is held alone before it counts as held (shorter: a tap, or a shortcut on its way).
const HOLD: Duration = Duration::from_millis(300);

/// What can be held alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keys {
    Mod,
    Shift,
}

impl Keys {
    pub fn name(self) -> &'static str {
        match self {
            Keys::Mod => "mod",
            Keys::Shift => "shift",
        }
    }
}

/// Where a press of Mod or Shift alone is.
#[derive(Debug, Default)]
pub struct ModHold {
    /// Held down alone, nothing else pressed since.
    down: Option<Keys>,
    /// `start` was told.
    started: bool,
    /// Which press the timer is for (an older one's timer does nothing).
    press: u64,
}

/// A change subscribers hear.
type Told = Option<(&'static str, Keys)>;

/// What a key did to the hold.
#[derive(Debug, PartialEq, Eq)]
struct Reaction {
    told: Told,
    /// A press of Mod or Shift alone: wait `HOLD`, then `elapsed`.
    wait: bool,
}

impl ModHold {
    /// A key went down or up: `key` is a key of Mod or Shift, `alone` the only modifier
    /// held after it (and nothing else that counts).
    fn key(&mut self, key: Option<Keys>, alone: Option<Keys>, pressed: bool) -> Reaction {
        let none = Reaction { told: None, wait: false };
        match (pressed, key) {
            // The other one of a pair (Shift_R after Shift_L), or key repeat.
            (true, Some(k)) if self.down == Some(k) && alone == Some(k) => none,
            (true, Some(k)) if alone == Some(k) => {
                let told = self.stop("cancel");
                *self = ModHold { down: Some(k), started: false, press: self.press + 1 };
                Reaction { told, wait: true }
            }
            (true, _) => Reaction { told: self.stop("cancel"), wait: false },
            (false, Some(k)) if self.down == Some(k) => Reaction { told: self.stop("end"), wait: false },
            (false, _) => none,
        }
    }

    /// A button went down or the wheel turned: it was Mod+drag, Shift+click or
    /// Shift+scroll, not a hold.
    fn button(&mut self) -> Told {
        self.stop("cancel")
    }

    /// `HOLD` after `press`: still down alone, it is held now.
    fn elapsed(&mut self, press: u64) -> Told {
        match self.down {
            Some(keys) if !self.started && press == self.press => {
                self.started = true;
                Some(("start", keys))
            }
            _ => None,
        }
    }

    /// No more hold; `told` if `start` was told.
    fn stop(&mut self, told: &'static str) -> Told {
        let (down, started) = (self.down.take(), std::mem::take(&mut self.started));
        down.filter(|_| started).map(|keys| (told, keys))
    }
}

/// Which of Mod and Shift `sym` is a key of, and which of them is the only modifier in
/// `mods`; Mod is neither for a modifier of two keys (`ctrl+alt`).
fn hold_key(modifier: &str, sym: Keysym, mods: &ModifiersState) -> (Option<Keys>, Option<Keys>) {
    let is_mod = match modifier {
        "super" | "logo" => matches!(sym, Keysym::Super_L | Keysym::Super_R),
        "alt" => matches!(sym, Keysym::Alt_L | Keysym::Alt_R | Keysym::Meta_L | Keysym::Meta_R),
        _ => false,
    };
    let key = if is_mod {
        Some(Keys::Mod)
    } else if matches!(sym, Keysym::Shift_L | Keysym::Shift_R) {
        Some(Keys::Shift)
    } else {
        None
    };
    let mod_alone = match modifier {
        "super" | "logo" => mods.logo && !mods.alt,
        "alt" => mods.alt && !mods.logo,
        _ => false,
    } && !mods.ctrl
        && !mods.shift;
    let shift_alone = mods.shift && !mods.logo && !mods.alt && !mods.ctrl;
    let alone = if mod_alone {
        Some(Keys::Mod)
    } else if shift_alone {
        Some(Keys::Shift)
    } else {
        None
    };
    (key, alone)
}

impl Dune {
    /// Every key that comes in, with the modifiers held after it.
    pub fn track_mod_hold(&mut self, sym: Keysym, mods: &ModifiersState, pressed: bool) {
        let (key, alone) = hold_key(&self.config.modifier, sym, mods);
        // Locked or keys held by a window: no hold starts (one going on still ends).
        let alone = alone.filter(|_| !self.session_lock.is_locked() && !self.shortcuts_inhibited());
        let reaction = self.mod_hold.key(key, alone, pressed);
        self.tell_mod(reaction.told);
        if reaction.wait {
            let press = self.mod_hold.press;
            let timer = self.loop_handle.insert_source(Timer::from_duration(HOLD), move |_, _, state| {
                let told = state.mod_hold.elapsed(press);
                state.tell_mod(told);
                TimeoutAction::Drop
            });
            if let Err(e) = timer {
                tracing::warn!("mod hold timer: {e}");
            }
        }
    }

    /// A mouse button went down, or the wheel (or a touchpad) scrolled.
    pub fn mod_hold_button(&mut self) {
        let told = self.mod_hold.button();
        self.tell_mod(told);
    }

    fn tell_mod(&mut self, told: Told) {
        if let Some((change, keys)) = told
            && !self.subscribers.is_empty()
        {
            self.asked_events.push(Event::Mod { change, keys: keys.name() });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD: Option<Keys> = Some(Keys::Mod);
    const SHIFT: Option<Keys> = Some(Keys::Shift);

    #[test]
    fn held_alone_then_let_go() {
        let mut hold = ModHold::default();
        assert_eq!(hold.key(MOD, MOD, true), Reaction { told: None, wait: true });
        assert_eq!(hold.elapsed(hold.press), Some(("start", Keys::Mod)));
        assert_eq!(hold.key(MOD, None, false).told, Some(("end", Keys::Mod)));
        // Shift the same.
        hold.key(SHIFT, SHIFT, true);
        assert_eq!(hold.elapsed(hold.press), Some(("start", Keys::Shift)));
        assert_eq!(hold.key(SHIFT, None, false).told, Some(("end", Keys::Shift)));
    }

    #[test]
    fn a_tap_or_a_shortcut_tells_nothing() {
        let mut hold = ModHold::default();
        hold.key(MOD, MOD, true);
        let press = hold.press;
        // Mod+1 before the time is up.
        assert_eq!(hold.key(None, None, true).told, None);
        assert_eq!(hold.elapsed(press), None);
        assert_eq!(hold.key(MOD, None, false).told, None);
        // A tap.
        hold.key(MOD, MOD, true);
        let press = hold.press;
        assert_eq!(hold.key(MOD, None, false).told, None);
        assert_eq!(hold.elapsed(press), None);
        // A capital letter.
        hold.key(SHIFT, SHIFT, true);
        let press = hold.press;
        hold.key(None, SHIFT, true);
        assert_eq!(hold.elapsed(press), None);
    }

    #[test]
    fn a_key_or_a_button_once_held_cancels() {
        let mut hold = ModHold::default();
        hold.key(MOD, MOD, true);
        hold.elapsed(hold.press);
        assert_eq!(hold.key(None, None, true).told, Some(("cancel", Keys::Mod)));
        assert_eq!(hold.key(MOD, None, false).told, None);

        hold.key(SHIFT, SHIFT, true);
        hold.elapsed(hold.press);
        assert_eq!(hold.button(), Some(("cancel", Keys::Shift)));
    }

    #[test]
    fn a_scroll_before_the_time_is_up_is_no_hold() {
        // Shift+wheel, a sideways scroll: the wheel turns before `HOLD`.
        let mut hold = ModHold::default();
        hold.key(SHIFT, SHIFT, true);
        let press = hold.press;
        assert_eq!(hold.button(), None);
        assert_eq!(hold.elapsed(press), None);
        assert_eq!(hold.key(SHIFT, None, false).told, None);
    }

    #[test]
    fn mod_then_shift_is_neither() {
        let mut hold = ModHold::default();
        hold.key(MOD, MOD, true);
        hold.elapsed(hold.press);
        // Mod+Shift: the Mod hold is a shortcut on its way; no Shift hold starts.
        assert_eq!(hold.key(SHIFT, None, true), Reaction { told: Some(("cancel", Keys::Mod)), wait: false });
        assert_eq!(hold.elapsed(hold.press), None);
        assert_eq!(hold.key(SHIFT, MOD, false).told, None);
    }

    #[test]
    fn the_other_shift_or_repeat_changes_nothing() {
        let mut hold = ModHold::default();
        hold.key(SHIFT, SHIFT, true);
        let press = hold.press;
        assert_eq!(hold.key(SHIFT, SHIFT, true), Reaction { told: None, wait: false });
        assert_eq!(hold.elapsed(press), Some(("start", Keys::Shift)));
    }

    #[test]
    fn an_old_timer_does_nothing() {
        let mut hold = ModHold::default();
        hold.key(MOD, MOD, true);
        let old = hold.press;
        hold.key(MOD, None, false);
        hold.key(MOD, MOD, true);
        assert_eq!(hold.elapsed(old), None);
        assert_eq!(hold.elapsed(hold.press), Some(("start", Keys::Mod)));
    }

    #[test]
    fn which_keys_are_alone() {
        let logo = ModifiersState { logo: true, ..Default::default() };
        assert_eq!(hold_key("super", Keysym::Super_L, &logo), (MOD, MOD));
        let logo_shift = ModifiersState { logo: true, shift: true, ..Default::default() };
        assert_eq!(hold_key("super", Keysym::Shift_L, &logo_shift), (SHIFT, None));
        let shift = ModifiersState { shift: true, ..Default::default() };
        assert_eq!(hold_key("super", Keysym::Shift_R, &shift), (SHIFT, SHIFT));
        assert_eq!(hold_key("super", Keysym::a, &logo), (None, MOD));
        let both = ModifiersState { alt: true, ctrl: true, ..Default::default() };
        assert_eq!(hold_key("ctrl+alt", Keysym::Alt_L, &both), (None, None));
        let ctrl_shift = ModifiersState { ctrl: true, shift: true, ..Default::default() };
        assert_eq!(hold_key("super", Keysym::Shift_L, &ctrl_shift), (SHIFT, None));
    }

    #[test]
    fn the_mod_line() {
        assert_eq!(
            serde_json::to_string(&Event::Mod { change: "start", keys: "shift" }).unwrap(),
            r#"{"event":"mod","change":"start","keys":"shift"}"#
        );
    }
}
