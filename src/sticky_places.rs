//! Where sticky floats were, by app id: a window that starts sticky again (the camera of
//! `dune-camera` opened anew) comes back on the monitor it was on, at the place and size it
//! had, and so with the same crop of its picture. Kept in `$XDG_STATE_HOME/dune/sticky.toml` (else
//! `~/.local/state/dune/sticky.toml`), across restarts of dune.

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    Dune,
    floating::keep_inside,
    layout::{Rect, TileId},
};

/// A sticky float's place: its output (none in a file from before outputs were kept), its
/// offset from that output's top left corner, and its size.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Place {
    /// The place of `rect` on the output `output`, whose area is `area`.
    pub fn of(rect: Rect, output: &str, area: Rect) -> Place {
        Place { output: Some(output.to_string()), x: rect.x - area.x, y: rect.y - area.y, w: rect.w, h: rect.h }
    }

    /// This place on the output whose area is `area`, all of it inside.
    pub fn on(&self, area: Rect) -> Rect {
        keep_inside(Rect::new(area.x + self.x, area.y + self.y, self.w, self.h), area)
    }
}

/// The places, by app id.
#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct StickyPlaces {
    #[serde(default)]
    places: BTreeMap<String, Place>,
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("dune").join("sticky.toml"))
}

impl StickyPlaces {
    /// The places saved, none when there is no file (or it cannot be read).
    pub fn load() -> StickyPlaces {
        let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else { return Self::default() };
        toml::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!("sticky places: {e}");
            Self::default()
        })
    }

    pub fn get(&self, app_id: &str) -> Option<&Place> {
        self.places.get(app_id)
    }

    /// `place` for `app_id`; saved to the file when it changed.
    pub fn set(&mut self, app_id: &str, place: Place) {
        if self.places.get(app_id) == Some(&place) {
            return;
        }
        self.places.insert(app_id.to_string(), place);
        if let Err(e) = self.save() {
            tracing::warn!("sticky places: {e}");
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = path().ok_or("no state folder ($XDG_STATE_HOME or $HOME)")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Written beside it and renamed: never half a file.
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, toml::to_string(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}

impl Dune {
    /// Remember where the sticky float `id` is, for the next window of its app.
    pub fn remember_sticky_place(&mut self, id: TileId) {
        if !self.sticky.contains(&id) {
            return;
        }
        let Some(app_id) = self.tile_app_id(id).filter(|a| !a.is_empty()) else { return };
        let Some((stack, rect)) =
            self.floating.iter().find_map(|(o, s)| s.iter().find(|(t, _)| *t == id).map(|(_, r)| (o.clone(), *r)))
        else {
            return;
        };
        // The output its middle is on, as `settle_floats` will have it: right after a drag
        // to another monitor it is still in the stack of the one it left.
        let areas: Vec<(String, Rect)> =
            self.space.outputs().filter_map(|o| Some((o.name(), self.output_rect(o)?))).collect();
        let (cx, cy) = (rect.x as f64 + rect.w as f64 / 2.0, rect.y as f64 + rect.h as f64 / 2.0);
        let Some((output, area)) =
            areas.iter().find(|(_, a)| a.contains(cx, cy)).or_else(|| areas.iter().find(|(n, _)| *n == stack))
        else {
            return;
        };
        let place = Place::of(keep_inside(rect, *area), output, *area);
        self.sticky_places.set(&app_id, place);
    }

    /// Where `id` floats when it starts sticky on `output`: where the last window of its
    /// app was, on the output it was on if that one is connected (else on `output`), if
    /// one was. The output and the rect.
    pub fn remembered_sticky_place(&self, id: TileId, output: &str) -> Option<(String, Rect)> {
        let app_id = self.tile_app_id(id)?;
        let place = self.sticky_places.get(&app_id)?;
        let area_of =
            |name: &str| self.space.outputs().find(|o| o.name() == name).and_then(|o| self.output_rect(o));
        let (name, area) = place
            .output
            .as_deref()
            .and_then(|n| Some((n.to_string(), area_of(n)?)))
            .or_else(|| Some((output.to_string(), area_of(output)?)))?;
        Some((name, place.on(area)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_goes_back_where_it_was_on_its_output() {
        let area = Rect::new(1920, 0, 2560, 1440);
        let rect = Rect::new(1920 + 40, 900, 400, 400);
        let place = Place::of(rect, "HDMI-A-1", area);
        assert_eq!(place, Place { output: Some("HDMI-A-1".into()), x: 40, y: 900, w: 400, h: 400 });
        assert_eq!(place.on(area), rect);
    }

    #[test]
    fn a_place_off_a_smaller_output_comes_inside_it() {
        let place = Place { output: None, x: 2000, y: 1200, w: 400, h: 300 };
        assert_eq!(place.on(Rect::new(0, 0, 1920, 1080)), Rect::new(1520, 780, 400, 300));
    }

    #[test]
    fn places_round_trip_through_toml() {
        let mut places = StickyPlaces::default();
        places.places.insert("dune-camera".into(), Place { output: Some("DP-1".into()), x: 10, y: 20, w: 300, h: 300 });
        let text = toml::to_string(&places).unwrap();
        assert_eq!(toml::from_str::<StickyPlaces>(&text).unwrap(), places);
    }

    #[test]
    fn a_file_without_outputs_still_reads() {
        let places: StickyPlaces = toml::from_str("[places.dune-camera]\nx = 1\ny = 2\nw = 3\nh = 4\n").unwrap();
        assert_eq!(places.get("dune-camera"), Some(&Place { output: None, x: 1, y: 2, w: 3, h: 4 }));
    }
}
