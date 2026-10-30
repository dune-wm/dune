//! Monitor scale (`scale` in `[output.NAME]`), fractional like 1.25 or 1.5. dune lays
//! everything out in logical pixels, the monitor's pixels divided by its scale, and draws
//! the scene scaled by it (`render::ScaledElements`). Clients hear the scale through
//! wp-fractional-scale (and wl_output's whole one, rounded up, for the rest), so they draw
//! their buffers at the monitor's pixels and stay sharp.

use smithay::{
    output::{Output, Scale},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        compositor::{get_parent, with_states},
        fractional_scale::{FractionalScaleHandler, with_fractional_scale},
    },
};

use crate::Dune;

/// The smallest and largest scale a monitor takes.
pub const MIN_SCALE: f64 = 0.5;
pub const MAX_SCALE: f64 = 4.0;

/// `value` as wp-fractional-scale can tell it (a multiple of 1/120), or `None` out of range.
pub fn valid_scale(value: f64) -> Option<f64> {
    (MIN_SCALE..=MAX_SCALE).contains(&value).then(|| (value * 120.0).round() / 120.0)
}

/// The width an output of `pixels` takes in logical pixels at `scale`.
pub fn logical_width(pixels: i32, scale: f64) -> i32 {
    (pixels as f64 / scale).round() as i32
}

impl Dune {
    /// The scale the config gives the output `name`: 1 without one.
    pub fn configured_scale(&self, name: &str) -> f64 {
        self.config.outputs.get(name).and_then(|c| c.scale).unwrap_or(1.0)
    }

    /// Give every output the scale of the config; `true` when one changed.
    pub fn apply_output_scales(&mut self) -> bool {
        let mut changed = false;
        for output in self.space.outputs().cloned().collect::<Vec<_>>() {
            let scale = self.configured_scale(&output.name());
            if output.current_scale().fractional_scale() != scale {
                tracing::info!("monitor {}: scale {scale}", output.name());
                output.change_current_state(None, None, Some(Scale::Fractional(scale)), None);
                changed = true;
            }
        }
        changed
    }

    /// Before a frame of `output` goes out: the surfaces on it hear its scale.
    pub fn send_preferred_scale(&self, output: &Output) {
        let scale = output.current_scale().fractional_scale();
        let set = |_: &WlSurface, states: &smithay::wayland::compositor::SurfaceData| {
            with_fractional_scale(states, |fractional| fractional.set_preferred_scale(scale));
        };
        for window in self.space.elements().filter(|w| self.space.outputs_for_element(w).contains(output)) {
            window.with_surfaces(set);
        }
        for layer in smithay::desktop::layer_map_for_output(output).layers() {
            layer.with_surfaces(set);
        }
    }

    /// The output a surface is (or will be) on: its window's, else the active one.
    fn output_for_surface(&self, surface: &WlSurface) -> Option<Output> {
        let mut root = surface.clone();
        while let Some(parent) = get_parent(&root) {
            root = parent;
        }
        let window = self.space.elements().find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == &root));
        window
            .and_then(|w| self.space.outputs_for_element(w).first().cloned())
            .or_else(|| self.active_output.as_ref().and_then(|name| self.space.outputs().find(|o| o.name() == *name).cloned()))
            .or_else(|| self.space.outputs().next().cloned())
    }
}

impl FractionalScaleHandler for Dune {
    // A new window hears its scale before its first frame, so it draws at it right away.
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        let Some(output) = self.output_for_surface(&surface) else { return };
        let scale = output.current_scale().fractional_scale();
        with_states(&surface, |states| {
            with_fractional_scale(states, |fractional| fractional.set_preferred_scale(scale));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_in_range_round_to_a_120th() {
        assert_eq!(valid_scale(1.5), Some(1.5));
        assert_eq!(valid_scale(1.333), Some(160.0 / 120.0));
        assert_eq!(valid_scale(0.4), None);
        assert_eq!(valid_scale(4.5), None);
    }

    #[test]
    fn logical_width_divides_by_the_scale() {
        assert_eq!(logical_width(2560, 1.0), 2560);
        assert_eq!(logical_width(2560, 1.25), 2048);
        assert_eq!(logical_width(3840, 1.5), 2560);
    }
}
