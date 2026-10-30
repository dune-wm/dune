//! Window rules (`[[rules]]` in dune.toml): what dune does to a new window by its app id
//! and title, like sway's `for_window`. The actions are the tile options' (Mod+/), by the
//! names of their keys:
//!
//! ```toml
//! [[rules]]
//! app_id = "^pavucontrol$"      # a regex on the app id (an X11 app: its class)
//! actions = ["float"]
//!
//! [[rules]]
//! app_id = "(?i)steam"
//! title = "^Steam$"             # both given: both must match
//! actions = ["send_to_desk_3"]
//! ```
//!
//! A rule runs once, when the window first shows (its app id and title are set by then),
//! and only for a window that gets a tile of its own on the shown desktop: one opened as a
//! tab, in a terminal's tile or on another desktop is left as it was put. Every rule that
//! matches runs, in the file's order. `float` floats (a window already floating, like a
//! dialog, stays so); the rest work as their keys do.

use std::collections::HashSet;

use dune_common::protocol::TileAction;
use regex::Regex;
use serde::Deserialize;
use smithay::wayland::{
    compositor::with_states,
    shell::xdg::{ToplevelSurface, XdgToplevelSurfaceData},
};

use crate::{Dune, layout::TileId};

/// A rule as `dune.toml` writes it.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RuleFile {
    app_id: Option<String>,
    title: Option<String>,
    actions: Vec<String>,
}

/// A regex, compared by its text (for the reload's "what changed").
#[derive(Debug, Clone)]
pub struct Pattern(Regex);

impl PartialEq for Pattern {
    fn eq(&self, other: &Pattern) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    app_id: Option<Pattern>,
    title: Option<Pattern>,
    pub actions: Vec<TileAction>,
}

impl Rule {
    /// Whether a window with `app_id` and `title` matches; a rule with neither matches none.
    fn matches(&self, app_id: &str, title: &str) -> bool {
        if self.app_id.is_none() && self.title.is_none() {
            return false;
        }
        let hit = |pattern: &Option<Pattern>, text: &str| pattern.as_ref().is_none_or(|p| p.0.is_match(text));
        hit(&self.app_id, app_id) && hit(&self.title, title)
    }
}

/// The rules of the file; one with a pattern that is not a regex, no pattern, or an action
/// dune does not know is warned about and left out (or that action is).
pub fn rules(file: &[RuleFile]) -> Vec<Rule> {
    file.iter()
        .enumerate()
        .filter_map(|(i, rule)| {
            let n = i + 1;
            let pattern = |key: &str, text: &Option<String>| match text.as_deref().map(Regex::new) {
                None => Ok(None),
                Some(Ok(regex)) => Ok(Some(Pattern(regex))),
                Some(Err(err)) => {
                    tracing::warn!("ignoring rule {n}: `{key}` is not a regex: {err}");
                    Err(())
                }
            };
            let app_id = pattern("app_id", &rule.app_id).ok()?;
            let title = pattern("title", &rule.title).ok()?;
            if app_id.is_none() && title.is_none() {
                tracing::warn!("ignoring rule {n}: it has no `app_id` or `title` to match");
                return None;
            }
            let actions = rule
                .actions
                .iter()
                .filter_map(|code| {
                    let action = TileAction::parse(code);
                    if action.is_none() {
                        tracing::warn!("ignoring `{code}` in rule {n}: not a tile action");
                    }
                    action
                })
                .collect();
            Some(Rule { app_id, title, actions })
        })
        .collect()
}

/// What the rules do to a window with `app_id` and `title`: the actions of every rule that
/// matches, in order.
pub fn actions_for(rules: &[Rule], app_id: &str, title: &str) -> Vec<TileAction> {
    rules.iter().filter(|r| r.matches(app_id, title)).flat_map(|r| r.actions.iter().copied()).collect()
}

/// Whether `action` takes the tile off the shown desktop (or closes it): no action can
/// follow it.
fn takes_it_away(action: TileAction) -> bool {
    matches!(action, TileAction::SendToDesk(_) | TileAction::Dock | TileAction::Scratchpad | TileAction::Close)
}

impl Dune {
    /// A new window committed: before its first configure, run the rules for it once
    /// (only for the tiles `pending_rules` holds).
    pub fn apply_rules(&mut self, surface: &ToplevelSurface) {
        let Some(id) = self.pending_rule_tile(surface) else { return };
        self.pending_rules.remove(&id);
        if self.config.rules.is_empty() || self.put_away(id) {
            return;
        }
        let (app_id, title) = with_states(surface.wl_surface(), |states| {
            let data = states.data_map.get::<XdgToplevelSurfaceData>().map(|d| d.lock().unwrap());
            let app_id = data.as_ref().and_then(|d| d.app_id.clone()).unwrap_or_default();
            let title = data.as_ref().and_then(|d| d.title.clone()).unwrap_or_default();
            (app_id, title)
        });
        let actions = actions_for(&self.config.rules, &app_id, &title);
        if actions.is_empty() {
            return;
        }
        tracing::info!(tile = id, app_id, ?actions, "window rules");
        for action in actions {
            if !self.tiles.contains_key(&id) {
                break;
            }
            match action {
                TileAction::Float if self.is_floating(id) => {}
                TileAction::Mute if self.tile_muted(id) => {}
                _ => self.run_tile_action(Some(id), action),
            }
            if takes_it_away(action) {
                break;
            }
        }
        // Sent away, it is no longer mapped and the commit would not configure it: it
        // gets its first configure here, to draw while it waits there.
        let mapped = self.space.elements().any(|w| w.toplevel().is_some_and(|t| t == surface));
        if !mapped && surface.alive() && !surface.is_initial_configure_sent() {
            surface.send_configure();
        }
    }

    /// The tile waiting for its rules whose only window is `surface`, not configured yet.
    fn pending_rule_tile(&mut self, surface: &ToplevelSurface) -> Option<TileId> {
        self.pending_rules.retain(|t| self.tiles.contains_key(t));
        let id = *self.pending_rules.iter().find(|t| {
            let tile = &self.tiles[t];
            tile.stack.len() == 1 && tile.top().and_then(|w| w.toplevel()).is_some_and(|t| t == surface)
        })?;
        let initial_sent = with_states(surface.wl_surface(), |states| {
            states.data_map.get::<XdgToplevelSurfaceData>().is_some_and(|d| d.lock().unwrap().initial_configure_sent)
        });
        if initial_sent {
            // Too late to place it otherwise: it is already shown.
            self.pending_rules.remove(&id);
            return None;
        }
        Some(id)
    }
}

/// Tiles whose rules still have to run (see `Dune::apply_rules`).
pub type PendingRules = HashSet<TileId>;

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> Vec<Rule> {
        #[derive(Deserialize)]
        struct F {
            rules: Vec<RuleFile>,
        }
        rules(&toml::from_str::<F>(text).unwrap().rules)
    }

    #[test]
    fn matching_rules_give_their_actions_in_order() {
        let rules = file(
            "[[rules]]\napp_id = \"^pavucontrol$\"\nactions = [\"float\"]\n\
             [[rules]]\napp_id = \"(?i)steam\"\ntitle = \"^Steam$\"\nactions = [\"send_to_desk_3\", \"mute_tile\"]\n\
             [[rules]]\ntitle = \"Picture-in-Picture\"\nactions = [\"float\", \"center\"]\n",
        );
        assert_eq!(actions_for(&rules, "pavucontrol", "Volume"), [TileAction::Float]);
        assert_eq!(actions_for(&rules, "org.pulseaudio.pavucontrol", "Volume"), []);
        // Both patterns must match.
        assert_eq!(actions_for(&rules, "Steam", "Steam"), [TileAction::SendToDesk(3), TileAction::Mute]);
        assert_eq!(actions_for(&rules, "steam", "Friends List"), []);
        assert_eq!(actions_for(&rules, "firefox", "Picture-in-Picture"), [TileAction::Float, TileAction::Center]);
        assert_eq!(actions_for(&rules, "firefox", "Mozilla Firefox"), []);
    }

    #[test]
    fn bad_rules_are_left_out() {
        let rules = file(
            "[[rules]]\napp_id = \"(\"\nactions = [\"float\"]\n\
             [[rules]]\nactions = [\"float\"]\n\
             [[rules]]\napp_id = \"foot\"\nactions = [\"fly\", \"full_width\"]\n",
        );
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].actions, [TileAction::FullWidth]);
        assert_eq!(actions_for(&rules, "foot", ""), [TileAction::FullWidth]);
    }

    #[test]
    fn moving_it_away_ends_the_rule() {
        assert!(takes_it_away(TileAction::SendToDesk(2)) && takes_it_away(TileAction::Close));
        assert!(!takes_it_away(TileAction::Float) && !takes_it_away(TileAction::Center));
    }
}
