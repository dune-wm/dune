use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent, InputTime, KeyState,
        PointerMotionEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, Keycode, Keysym, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, PointerHandle, RelativeMotionEvent},
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
    wayland::{
        compositor::RegionAttributes,
        pointer_constraints::{PointerConstraint, with_pointer_constraint},
    },
};

use dune_common::protocol::{MouseButton, PointerOp, TileAction};

use crate::{
    config::{Binding, Config, TerminalHere},
    launcher::FinderMode,
    layout::{Direction, Layout, Rect, Side, TileId, split_band, split_edge},
    screenshot::Capture,
    seatop::{BTN_LEFT, BTN_RIGHT},
    state::Dune,
};

/// The middle button (linux/input-event-codes.h).
pub const BTN_MIDDLE: u32 = 0x112;

/// What compositor keys do: the config's bindings (`Binding`), and VT switching.
#[derive(Debug, Clone, Copy)]
enum Action {
    /// Mod+Enter: an empty container, dune-finder to choose what goes in it.
    EmptyContainer,
    /// Mod+Shift+t: an empty tile as the next tab of the focused tile's container.
    EmptyTab,
    Focus(Direction),
    Swap(Direction),
    /// Focus the monitor that way.
    FocusMonitor(Direction),
    /// Focus the next monitor, top to bottom and left to right, back to the first.
    FocusMonitorNext,
    /// The focused tile to that next monitor.
    MoveMonitorNext,
    /// Column width / tile height, as a share of the output.
    Width(f64),
    Height(f64),
    /// Width of a tile inside its row, as a share of the column.
    RowWidth(f64),
    CycleWidth,
    FullWidth,
    /// The focused tile over its whole monitor (again: back).
    Fullscreen,
    /// Center the focused column on its monitor (again: stop centering it).
    Center,
    /// Float the focused tile in the middle of its monitor (again: back into the strip).
    ToggleFloat,
    /// Mod+Shift+`: the focused tile into the scratchpad.
    SendToScratchpad,
    /// Mod+`: the shown scratchpad window focused back into it, else the next one out.
    Scratchpad,
    Close,
    /// Close every tile of the focused tile's column, tabs and all.
    CloseGroup,
    Quit,
    /// Mod+d: show dune-finder on the apps (again: hide it).
    Launcher,
    /// dune-finder on the windows (again: hide it); no key by default (`/` in the finder).
    Switcher,
    /// Mod+/: dune-finder on the focused tile's options (again: hide it).
    TileOptions,
    /// Block the focused tile on screen and in captures (again: show it).
    PrivateTile,
    /// Mute the sound of the focused tile's app (again: unmute it).
    MuteTile,
    /// The focused tile floats in front of every other, on every desktop (again: off).
    Sticky,
    /// The focused tile (with its tabs) into the dock of its desktop.
    Dock,
    /// Mod+Shift+e: dune-shell's dock of the active monitor (again: hide it), through `dune events`.
    DockShow,
    /// Mod+a: dune-shell's minimap of the active monitor, through `dune events`.
    Minimap,
    /// Mod+z: the tile under the pointer magnified, the pointer in its middle; again: out (`crate::zoom`).
    Zoom,
    /// Mod+Shift+z: out of the zoom without a click.
    ZoomOut,
    /// Mod+v / Mod+s / Mod+Shift+t: a terminal to the right of the focused tile, below it
    /// or as a tab of its container, in its folder.
    Terminal(TerminalHere),
    /// Mod+Tab / Mod+Shift+Tab: the next / previous tab of the focused column.
    CycleTab(i32),
    /// Mod+Ctrl+direction: join the column that way as a tab, or leave a tabbed one.
    MoveTab(Direction),
    /// Mod+t: the focused tile leaves the tiles it shares a tab with for a tab of its own
    /// in its column; alone, the windows stacked in it become tabs of its column; a tab
    /// with none goes back where it was in its column, or out of the tabs.
    StackToTabs,
    /// Mod+1..9: show that desktop on the active monitor.
    Desk(u8),
    /// Mod+Shift+1..9: send the focused tile to that desktop.
    SendToDesk(u8),
    /// Mod+Escape: run `lock_command` (swaylock).
    Lock,
    /// Mod+Shift+Escape: the focused window's hold on every key off (again: on).
    ShortcutsInhibit,
    /// Print / Ctrl+Print / Shift+Print: a screenshot of a region chosen with the mouse,
    /// of the focused tile, of the active monitor.
    Screenshot(Capture),
    /// Mod+Shift+r / Mod+Ctrl+r / Mod+Ctrl+Shift+r: record the same (again: stop).
    Record(Capture),
    /// A key of `[spawn]`: its command.
    Spawn(usize),
    /// Ctrl+Alt+F1..F12 on a real session.
    SwitchVt(i32),
}

impl From<Binding> for Action {
    fn from(binding: Binding) -> Action {
        match binding {
            Binding::Focus(dir) => Action::Focus(dir),
            Binding::Swap(dir) => Action::Swap(dir),
            Binding::MoveTab(dir) => Action::MoveTab(dir),
            Binding::FocusMonitor(dir) => Action::FocusMonitor(dir),
            Binding::FocusMonitorNext => Action::FocusMonitorNext,
            Binding::MoveMonitorNext => Action::MoveMonitorNext,
            Binding::Desk(n) => Action::Desk(n),
            Binding::SendToDesk(n) => Action::SendToDesk(n),
            Binding::EmptyContainer => Action::EmptyContainer,
            Binding::EmptyTab => Action::EmptyTab,
            Binding::NextTab => Action::CycleTab(1),
            Binding::PrevTab => Action::CycleTab(-1),
            Binding::Wider => Action::Width(RESIZE_STEP),
            Binding::Narrower => Action::Width(-RESIZE_STEP),
            Binding::Taller => Action::Height(RESIZE_STEP),
            Binding::Shorter => Action::Height(-RESIZE_STEP),
            Binding::RowWider => Action::RowWidth(RESIZE_STEP),
            Binding::RowNarrower => Action::RowWidth(-RESIZE_STEP),
            Binding::CycleWidth => Action::CycleWidth,
            Binding::FullWidth => Action::FullWidth,
            Binding::Fullscreen => Action::Fullscreen,
            Binding::Center => Action::Center,
            Binding::Float => Action::ToggleFloat,
            Binding::SendToScratchpad => Action::SendToScratchpad,
            Binding::Scratchpad => Action::Scratchpad,
            Binding::Close => Action::Close,
            Binding::CloseGroup => Action::CloseGroup,
            Binding::Launcher => Action::Launcher,
            Binding::Switcher => Action::Switcher,
            Binding::TileOptions => Action::TileOptions,
            Binding::PrivateTile => Action::PrivateTile,
            Binding::MuteTile => Action::MuteTile,
            Binding::Sticky => Action::Sticky,
            Binding::Dock => Action::Dock,
            Binding::DockShow => Action::DockShow,
            Binding::Minimap => Action::Minimap,
            Binding::Zoom => Action::Zoom,
            Binding::ZoomOut => Action::ZoomOut,
            Binding::Terminal(here) => Action::Terminal(here),
            Binding::StackToTabs => Action::StackToTabs,
            Binding::Quit => Action::Quit,
            Binding::Lock => Action::Lock,
            Binding::ShortcutsInhibit => Action::ShortcutsInhibit,
            Binding::Screenshot => Action::Screenshot(Capture::Region),
            Binding::ScreenshotTile => Action::Screenshot(Capture::Tile),
            Binding::ScreenshotScreen => Action::Screenshot(Capture::Screen),
            Binding::Record => Action::Record(Capture::Region),
            Binding::RecordTile => Action::Record(Capture::Tile),
            Binding::RecordScreen => Action::Record(Capture::Screen),
            Binding::Spawn(n) => Action::Spawn(n),
        }
    }
}

/// How much one press of the resize keys changes a width or height.
const RESIZE_STEP: f64 = 0.05;
/// Scroll distance of one wheel notch (as smithay reports it).
const SCROLL_NOTCH: f64 = 15.0;

/// Pixels the pointer pushes against a screen edge before the strip starts sliding.
const EDGE_RESISTANCE: f64 = 50.0;
/// Pixels the strip slides per pixel the pointer pushes past the resistance.
const EDGE_SPEED: f64 = 2.0;

/// The pointer, already at a left/right screen edge (`at_edge`), went `over` pixels past it
/// (right: positive; 0: not pushing): what has been pushed so far goes in `pushed`, and once
/// past `EDGE_RESISTANCE` the pixels to slide the strip. Leaving the edge starts over.
fn edge_slide(pushed: &mut f64, at_edge: bool, over: f64) -> i32 {
    if !at_edge || over == 0.0 {
        *pushed = 0.0;
        return 0;
    }
    let before = *pushed;
    *pushed += over.abs();
    let past = *pushed - before.max(EDGE_RESISTANCE);
    if past <= 0.0 {
        return 0;
    }
    (past * EDGE_SPEED * over.signum()).round() as i32
}

/// Pixels the strip slides per pixel of a Super+scroll: a wheel notch (`SCROLL_NOTCH`)
/// slides 120 px; a touchpad's scroll, already in pixels, a bit more than it moves.
const STRIP_WHEEL_SPEED: f64 = 8.0;
const STRIP_FINGER_SPEED: f64 = 2.0;

/// How far a Super+scroll slides the strip: down or right shows what is to the right.
fn strip_scroll_dx(source: AxisSource, horizontal: f64, vertical: f64) -> i32 {
    let speed = match source {
        AxisSource::Wheel | AxisSource::WheelTilt => STRIP_WHEEL_SPEED,
        _ => STRIP_FINGER_SPEED,
    };
    ((horizontal + vertical) * speed).round() as i32
}

/// Width/height change for an Alt+scroll: a wheel notch is one resize step, up grows.
fn scroll_resize_delta(vertical_scroll: f64) -> f64 {
    -vertical_scroll / SCROLL_NOTCH * RESIZE_STEP
}

fn modifier_held(modifier: &str, mods: &ModifiersState) -> bool {
    match modifier {
        "super" | "logo" => mods.logo,
        "ctrl+alt" => mods.ctrl && mods.alt,
        _ => mods.alt,
    }
}

fn action_for(config: &Config, mods: &ModifiersState, sym: Keysym) -> Option<Action> {
    // The keymap turns Ctrl+Alt+Fn into XF86Switch_VT_n.
    let vt = sym.raw().wrapping_sub(Keysym::XF86_Switch_VT_1.raw());
    if vt < 12 {
        return Some(Action::SwitchVt(vt as i32 + 1));
    }
    let (binding, _) = config.bindings.iter().find(|(_, key)| key.matches(mods, sym))?;
    Some((*binding).into())
}

/// While the session is locked, only VT switching stays a compositor key; the rest go to
/// the lock surface.
fn allowed_while_locked(action: &Action) -> bool {
    matches!(action, Action::SwitchVt(_))
}

/// While the focused window holds every key (a VM, a remote desktop): the keys dune keeps,
/// the one that takes them back among them.
fn allowed_while_inhibited(action: &Action) -> bool {
    matches!(action, Action::SwitchVt(_) | Action::ShortcutsInhibit | Action::Lock)
}

/// The closest point to `pos` inside one of `outputs` (pointer motion stays on the screens).
fn clamp_to(outputs: &[Rectangle<i32, Logical>], pos: Point<f64, Logical>) -> Point<f64, Logical> {
    if outputs.iter().any(|g| g.to_f64().contains(pos)) {
        return pos;
    }
    outputs
        .iter()
        .map(|g| {
            let g = g.to_f64();
            let x = pos.x.clamp(g.loc.x, g.loc.x + g.size.w - 1.0);
            let y = pos.y.clamp(g.loc.y, g.loc.y + g.size.h - 1.0);
            Point::from((x, y))
        })
        .min_by(|a, b| {
            let d = |p: &Point<f64, Logical>| (p.x - pos.x).powi(2) + (p.y - pos.y).powi(2);
            d(a).total_cmp(&d(b))
        })
        .unwrap_or(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_stays_on_the_screens() {
        // Two monitors side by side, of different heights (like 2560x1080 + 1920x1080, shorter).
        let outputs = [Rectangle::new((0, 0).into(), (2560, 1080).into()), Rectangle::new((2560, 0).into(), (1920, 900).into())];
        let p = |x: f64, y: f64| Point::<f64, Logical>::from((x, y));
        assert_eq!(clamp_to(&outputs, p(100.0, 100.0)), p(100.0, 100.0));
        assert_eq!(clamp_to(&outputs, p(-50.0, 100.0)), p(0.0, 100.0));
        assert_eq!(clamp_to(&outputs, p(3000.0, 1000.0)), p(3000.0, 899.0)); // below the short monitor
        assert_eq!(clamp_to(&outputs, p(9000.0, -5.0)), p(4479.0, 0.0));
    }

    #[test]
    fn pushing_a_screen_edge_slides_past_the_resistance() {
        let mut pushed = 0.0;
        // Arriving at the edge does not count.
        assert_eq!(edge_slide(&mut pushed, false, 30.0), 0);
        assert_eq!(edge_slide(&mut pushed, true, 30.0), 0);
        assert_eq!(edge_slide(&mut pushed, true, 30.0), (10.0 * EDGE_SPEED) as i32);
        assert_eq!(edge_slide(&mut pushed, true, 5.0), (5.0 * EDGE_SPEED) as i32);
        // Moving off the edge starts over; the left edge slides the other way.
        assert_eq!(edge_slide(&mut pushed, true, 0.0), 0);
        assert_eq!(edge_slide(&mut pushed, true, -60.0), -(10.0 * EDGE_SPEED) as i32);
    }

    #[test]
    fn scroll_resize_steps() {
        assert_eq!(scroll_resize_delta(-15.0), RESIZE_STEP); // one notch up grows
        // A notch down slides the strip 120 px to the right, one up back; a touchpad less.
        assert_eq!(strip_scroll_dx(AxisSource::Wheel, 0.0, SCROLL_NOTCH), 120);
        assert_eq!(strip_scroll_dx(AxisSource::Wheel, 0.0, -SCROLL_NOTCH), -120);
        assert_eq!(strip_scroll_dx(AxisSource::Finger, -10.0, 0.0), -20);
        assert_eq!(scroll_resize_delta(30.0), -2.0 * RESIZE_STEP);
    }

    fn test_config(modifier: &str) -> Config {
        crate::config::from_text(&format!("modifier = \"{modifier}\"\n"))
    }

    #[test]
    fn vt_switch_keys() {
        let config = test_config("alt");
        let mods = ModifiersState { ctrl: true, alt: true, ..Default::default() };
        assert!(matches!(action_for(&config, &mods, Keysym::XF86_Switch_VT_3), Some(Action::SwitchVt(3))));
        assert!(matches!(action_for(&config, &mods, Keysym::XF86_Switch_VT_12), Some(Action::SwitchVt(12))));
        assert!(action_for(&config, &mods, Keysym::F3).is_none());
    }

    #[test]
    fn only_vt_switching_while_locked() {
        let config = test_config("alt");
        let alt = ModifiersState { alt: true, ..Default::default() };
        let ctrl_alt = ModifiersState { ctrl: true, alt: true, ..Default::default() };
        let allowed = |mods: &ModifiersState, sym| action_for(&config, mods, sym).is_some_and(|a| allowed_while_locked(&a));
        assert!(allowed(&ctrl_alt, Keysym::XF86_Switch_VT_2));
        for sym in [Keysym::Return, Keysym::d, Keysym::slash, Keysym::q, Keysym::_1, Keysym::Left, Keysym::E, Keysym::Escape] {
            assert!(action_for(&config, &alt, sym).is_none_or(|a| !allowed_while_locked(&a)), "{sym:?}");
        }
        // Mod+Shift+e (the dock) stays off too.
        let alt_shift = ModifiersState { alt: true, shift: true, ..Default::default() };
        assert!(matches!(action_for(&config, &alt_shift, Keysym::e), Some(Action::DockShow)));
        assert!(!allowed(&alt_shift, Keysym::e));
    }

    #[test]
    fn mod_a_is_the_minimap() {
        let config = test_config("alt");
        let alt = ModifiersState { alt: true, ..Default::default() };
        assert!(matches!(action_for(&config, &alt, Keysym::a), Some(Action::Minimap)));
        assert!(!allowed_while_locked(&Action::Minimap));
    }

    #[test]
    fn ctrl_resizes_inside_the_row() {
        let alt_ctrl = ModifiersState { alt: true, ctrl: true, ..Default::default() };
        let config = test_config("alt");
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::plus), Some(Action::RowWidth(d)) if d > 0.0));
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::minus), Some(Action::RowWidth(d)) if d < 0.0));
        // With Ctrl in the modifier, the plain keys stay the column's width.
        let config = test_config("ctrl+alt");
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::plus), Some(Action::Width(_))));
    }

    #[test]
    fn terminal_keys() {
        let config = test_config("super");
        let logo = ModifiersState { logo: true, ..Default::default() };
        assert!(matches!(action_for(&config, &logo, Keysym::v), Some(Action::Terminal(TerminalHere::Right))));
        assert!(matches!(action_for(&config, &logo, Keysym::s), Some(Action::Terminal(TerminalHere::Below))));
        // A key set for one replaces its Mod key; the others keep theirs.
        let config = crate::config::from_text("modifier = \"super\"\n[keys]\nterminal_vertical = \"mod+n\"\n");
        assert!(matches!(action_for(&config, &logo, Keysym::n), Some(Action::Terminal(TerminalHere::Right))));
        assert!(action_for(&config, &logo, Keysym::v).is_none());
        assert!(matches!(action_for(&config, &logo, Keysym::s), Some(Action::Terminal(TerminalHere::Below))));
    }

    #[test]
    fn tab_keys() {
        let config = test_config("alt");
        let alt = ModifiersState { alt: true, ..Default::default() };
        let alt_shift = ModifiersState { alt: true, shift: true, ..Default::default() };
        let alt_ctrl = ModifiersState { alt: true, ctrl: true, ..Default::default() };
        assert!(matches!(action_for(&config, &alt, Keysym::Tab), Some(Action::CycleTab(1))));
        assert!(matches!(action_for(&config, &alt_shift, Keysym::Tab), Some(Action::CycleTab(-1))));
        assert!(matches!(action_for(&config, &alt_shift, Keysym::ISO_Left_Tab), Some(Action::CycleTab(-1))));
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::Left), Some(Action::MoveTab(Direction::Left))));
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::j), Some(Action::MoveTab(Direction::Down))));
        assert!(matches!(action_for(&config, &alt, Keysym::t), Some(Action::StackToTabs)));
        assert!(matches!(action_for(&config, &alt_shift, Keysym::t), Some(Action::EmptyTab)));
        assert!(matches!(action_for(&config, &alt, Keysym::slash), Some(Action::TileOptions)));
        assert!(matches!(action_for(&config, &alt_shift, Keysym::Left), Some(Action::Swap(Direction::Left))));
        // With Ctrl in the modifier, Ctrl+direction stays a focus move.
        let config = test_config("ctrl+alt");
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::Left), Some(Action::Focus(Direction::Left))));
    }

    #[test]
    fn escape_locks() {
        let alt = ModifiersState { alt: true, ..Default::default() };
        assert!(matches!(action_for(&test_config("alt"), &alt, Keysym::Escape), Some(Action::Lock)));
        assert!(action_for(&test_config("alt"), &ModifiersState::default(), Keysym::Escape).is_none());
        let logo = ModifiersState { logo: true, ..Default::default() };
        assert!(matches!(action_for(&test_config("super"), &logo, Keysym::Escape), Some(Action::Lock)));
        // Set off: Mod+Escape goes to the client.
        let config = crate::config::from_text("[keys]\nlock = \"none\"\n");
        assert!(action_for(&config, &alt, Keysym::Escape).is_none());
    }

    #[test]
    fn print_takes_screenshots() {
        let config = test_config("alt");
        let none = ModifiersState::default();
        let shift = ModifiersState { shift: true, ..Default::default() };
        let alt_shift = ModifiersState { alt: true, shift: true, ..Default::default() };
        let ctrl = ModifiersState { ctrl: true, ..Default::default() };
        let alt_ctrl = ModifiersState { alt: true, ctrl: true, ..Default::default() };
        let alt_ctrl_shift = ModifiersState { shift: true, ..alt_ctrl };
        assert!(matches!(action_for(&config, &none, Keysym::Print), Some(Action::Screenshot(Capture::Region))));
        assert!(matches!(action_for(&config, &ctrl, Keysym::Print), Some(Action::Screenshot(Capture::Tile))));
        assert!(matches!(action_for(&config, &shift, Keysym::Print), Some(Action::Screenshot(Capture::Screen))));
        assert!(matches!(action_for(&config, &alt_shift, Keysym::r), Some(Action::Record(Capture::Region))));
        assert!(matches!(action_for(&config, &alt_ctrl, Keysym::r), Some(Action::Record(Capture::Tile))));
        assert!(matches!(action_for(&config, &alt_ctrl_shift, Keysym::r), Some(Action::Record(Capture::Screen))));
    }

    #[test]
    fn digits_pick_desktops() {
        let config = test_config("super");
        let logo = ModifiersState { logo: true, ..Default::default() };
        let shift = ModifiersState { logo: true, shift: true, ..Default::default() };
        assert!(matches!(action_for(&config, &logo, Keysym::_1), Some(Action::Desk(1))));
        assert!(matches!(action_for(&config, &logo, Keysym::_9), Some(Action::Desk(9))));
        assert!(matches!(action_for(&config, &shift, Keysym::_3), Some(Action::SendToDesk(3))));
        assert!(action_for(&config, &logo, Keysym::_0).is_none());
        assert!(action_for(&config, &ModifiersState::default(), Keysym::_2).is_none());
    }
}

/// What an active pointer constraint asks of the pointer.
enum Constraint {
    Locked,
    /// Confined to this region of the surface (all of it without one).
    Confined(Option<RegionAttributes>),
}

/// The active constraint of `surface`, when the pointer at `local` (on the surface) is in its region.
fn active_constraint(surface: &WlSurface, pointer: &PointerHandle<Dune>, local: Point<f64, Logical>) -> Option<Constraint> {
    with_pointer_constraint(surface, pointer, |constraint| {
        let constraint = constraint.filter(|c| c.is_active())?;
        if !constraint.region().is_none_or(|r| r.contains(local.to_i32_round())) {
            return None;
        }
        Some(match &*constraint {
            PointerConstraint::Locked(_) => Constraint::Locked,
            PointerConstraint::Confined(confined) => Constraint::Confined(confined.region().cloned()),
        })
    })
}

/// The pointer reached `surface` at `local`: a constraint waiting there starts.
fn activate_constraint(surface: &WlSurface, pointer: &PointerHandle<Dune>, local: Point<f64, Logical>) {
    with_pointer_constraint(surface, pointer, |constraint| {
        if let Some(constraint) = constraint.filter(|c| !c.is_active())
            && constraint.region().is_none_or(|r| r.contains(local.to_i32_round()))
        {
            tracing::info!("pointer constraint active");
            constraint.activate();
        }
    });
}

impl Dune {
    /// The layout may have moved under a still pointer (a tile mapped, resized, floated, a
    /// strip slide, a desktop switch): the pointer focus goes to what is under it now, and a
    /// constraint waiting there starts. Run on every turn of the event loop.
    pub fn refresh_pointer_focus(&mut self) {
        let pointer = self.seat.get_pointer().unwrap();
        let location = pointer.current_location();
        let under = self.surface_under(location);
        if under.as_ref().map(|(s, _)| s) != pointer.current_focus().as_ref() {
            let event = MotionEvent { location, serial: SERIAL_COUNTER.next_serial(), time: InputTime::now() };
            pointer.motion(self, under.clone(), &event);
            pointer.frame(self);
        }
        if let Some((surface, origin)) = under {
            activate_constraint(&surface, &pointer, location - origin);
        }
    }

    /// The closest point to `pos` that is on some output.
    pub fn clamp_to_outputs(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
        let geos: Vec<_> = self.space.outputs().filter_map(|o| self.space.output_geometry(o)).collect();
        clamp_to(&geos, pos)
    }

    /// Slide the strip of the output under `pos` by `dx` pixels (none: nothing).
    fn slide_strip(&mut self, pos: Point<f64, Logical>, dx: i32) {
        if dx == 0 {
            return;
        }
        let Some(name) = self.space.output_under(pos).next().map(|o| o.name()) else { return };
        let focus = self.focused;
        if let Some(layout) = self.layouts.get_mut(&name) {
            layout.slide_view(dx, focus);
            self.arrange();
        }
    }

    /// A resize key or Alt+scroll: a float grows by `dw`/`dh` around its middle, a tile in
    /// the strip takes `change`.
    fn resize_tile(&mut self, id: TileId, dw: f64, dh: f64, change: impl Fn(&mut Layout, TileId)) {
        if self.is_floating(id) {
            self.scale_float(id, dw, dh);
        } else {
            // Keys and wheel jump a whole step: the tiles slide there instead.
            self.animate_resize();
            self.change_tile(id, change);
        }
    }

    fn resize_focused(&mut self, dw: f64, dh: f64, change: impl Fn(&mut Layout, TileId)) {
        if let Some(id) = self.focused {
            self.resize_tile(id, dw, dh, change);
        }
    }

    /// `focus_follows_mouse`: the tile under the pointer takes focus, unless part of it is
    /// off screen or the focused column is centered (focusing it would slide the view under
    /// the pointer), a button or drag is going on, dune-finder is open or the session is locked.
    fn hover_focus(&mut self, pos: Point<f64, Logical>) {
        let busy = !self.seatop.is_default()
            || self.finder.is_some()
            || self.session_lock.is_locked();
        if !self.config.focus_follows_mouse || busy || self.seat.get_pointer().unwrap().is_grabbed()
        {
            return;
        }
        let Some((id, shown)) = self.shown.iter().find(|(_, r)| r.contains(pos.x, pos.y)).copied() else { return };
        let whole = self.geometries.iter().any(|(t, r)| *t == id && *r == shown);
        let centered = self.focused.is_some_and(|f| self.layouts.values().any(|l| l.ends_centering(f, id)));
        if whole && !centered && self.focused != Some(id) {
            self.focus_tile(id);
        }
    }

    /// Mod held, no drag going on and the pointer near an edge of a tile of the strip: that
    /// tile, the side, and the band along that edge to light (`split_hint_at`).
    pub fn split_hint(&self) -> Option<(TileId, Side, Rect)> {
        if self.seatop.dragging()
            || self.finder.is_some()
            || self.session_lock.is_locked()
        {
            return None;
        }
        let mods = self.seat.get_keyboard()?.modifier_state();
        if !modifier_held(&self.config.modifier, &mods) {
            return None;
        }
        self.split_hint_at(self.seat.get_pointer()?.current_location())
    }

    /// The tile of the strip under `pos`, if `pos` is near one of its edges (`split_edge`).
    pub fn split_hint_at(&self, pos: Point<f64, Logical>) -> Option<(TileId, Side, Rect)> {
        let (id, rect) = self.shown.iter().find(|(_, r)| r.contains(pos.x, pos.y)).copied()?;
        if self.is_floating(id) {
            return None;
        }
        let side = split_edge(&rect, pos.x, pos.y)?;
        Some((id, side, split_band(&rect, side)))
    }

    /// `dune action NAME`: what the key of that name in `[keys]` does, as if it were pressed;
    /// not while the session is locked, like the keys.
    pub fn run_named_action(&mut self, name: &str) -> Result<(), String> {
        let binding = crate::config::binding_named(name).ok_or_else(|| format!("dune has no action `{name}`"))?;
        let action = Action::from(binding);
        if self.session_lock.is_locked() && !allowed_while_locked(&action) {
            return Err("the session is locked".into());
        }
        self.run_action(action);
        Ok(())
    }

    fn run_action(&mut self, action: Action) {
        match action {
            Action::EmptyContainer => self.open_empty_container(),
            Action::EmptyTab => self.open_empty_tab_here(),
            Action::Launcher => self.toggle_finder(FinderMode::Apps),
            Action::Switcher => self.toggle_finder(FinderMode::Windows),
            Action::TileOptions => self.toggle_finder(FinderMode::TileOptions),
            Action::PrivateTile => self.toggle_private_focused(),
            Action::MuteTile => self.toggle_mute_focused(),
            Action::Sticky => {
                if let Some(id) = self.focused {
                    self.toggle_sticky(id);
                }
            }
            Action::Dock => {
                if let Some(id) = self.focused {
                    self.send_to_dock(id);
                }
            }
            Action::DockShow => self.ask_dock(None),
            Action::Minimap => self.ask_minimap(),
            Action::Zoom => self.zoom_key(),
            Action::ZoomOut => self.zoom_out(),
            Action::Focus(dir) => self.focus_direction(dir),
            Action::FocusMonitor(dir) => self.focus_output_direction(dir),
            Action::FocusMonitorNext => self.focus_output_next(),
            Action::MoveMonitorNext => self.move_to_output_next(),
            Action::Swap(dir) => self.swap_direction(dir),
            Action::Width(delta) => self.resize_focused(delta, 0.0, |l, id| l.resize_width(id, delta)),
            Action::Height(delta) => self.resize_focused(0.0, delta, |l, id| l.resize_height(id, delta)),
            Action::RowWidth(delta) => self.resize_focused(delta, 0.0, |l, id| l.resize_row_width(id, delta)),
            Action::CycleWidth => self.change_focused(Layout::cycle_width),
            Action::FullWidth => self.change_focused(Layout::toggle_full_width),
            Action::Fullscreen => {
                if let Some(id) = self.focused {
                    let on = !self.fullscreen.contains(&id);
                    self.set_fullscreen(id, on);
                }
            }
            Action::Center => self.change_focused(Layout::toggle_center),
            Action::ToggleFloat => {
                if let Some(id) = self.focused {
                    self.toggle_floating(id);
                }
            }
            Action::SendToScratchpad => {
                if let Some(id) = self.focused {
                    self.send_to_scratchpad(id);
                }
            }
            Action::Scratchpad => self.toggle_scratchpad(),
            Action::Close => self.close_focused(),
            Action::CloseGroup => self.close_group(),
            Action::Quit => self.loop_signal.stop(),
            Action::CycleTab(delta) => self.cycle_tab(delta),
            Action::MoveTab(dir) => self.move_tab(dir),
            Action::StackToTabs => self.stack_to_tabs(),
            Action::Terminal(here) => self.terminal_here(here),
            Action::Desk(n) => self.show_desk(n),
            Action::SendToDesk(n) => self.send_to_desk(n),
            Action::Lock => crate::autostart(&self.config.lock_command),
            Action::ShortcutsInhibit => self.toggle_shortcuts_inhibit(),
            Action::Screenshot(what) => self.screenshot(what),
            Action::Record(what) => self.toggle_recording(what),
            Action::Spawn(n) => {
                if let Some(command) = self.config.spawn.get(n) {
                    crate::autostart(command);
                }
            }
            Action::SwitchVt(vt) => self.change_vt(vt),
        }
    }

    /// A row chosen in the tile options menu: `action` on `tile`, focused first (the actions
    /// work on the focused tile). Nothing without one, or with one on a locked desktop.
    pub fn run_tile_action(&mut self, tile: Option<TileId>, action: TileAction) {
        let Some(tile) = tile.filter(|t| self.tiles.contains_key(t) && !self.on_locked_desk(*t)) else { return };
        match action {
            TileAction::PrivateTile => return self.toggle_private_tile(tile),
            TileAction::Mute => return self.toggle_mute_tile(tile),
            _ => {}
        }
        if self.focused != Some(tile) {
            self.focus_tile(tile);
        }
        self.run_action(crate::privacy::binding_of(action).into());
    }

    /// A key press or release: compositor keybindings first, then the focused client.
    pub fn handle_key(&mut self, keycode: Keycode, state: KeyState, time: InputTime) {
        self.restore_seat_keymap();
        let serial = SERIAL_COUNTER.next_serial();
        let pressed = state == KeyState::Pressed;
        let config = self.config.clone();
        // While dune-finder is open, Mod+Enter is its own (files: the file manager).
        let finder_open = self.finder.is_some();
        let locked = self.session_lock.is_locked();
        let inhibited = self.shortcuts_inhibited();

        let mut seen = None;
        let action = self.seat.get_keyboard().unwrap().input::<Action, _>(
            self,
            keycode,
            state,
            serial,
            time,
            |_, mods, handle| match handle.raw_latin_sym_or_raw_current_sym().inspect(|sym| seen = Some((*sym, *mods))).and_then(|sym| action_for(&config, mods, sym)) {
                // While dune-finder is open, Mod+Enter and Mod+Tab are its own.
                Some(Action::EmptyContainer | Action::CycleTab(_)) if finder_open => FilterResult::Forward,
                Some(action) if locked && !allowed_while_locked(&action) => FilterResult::Forward,
                Some(action) if inhibited && !allowed_while_inhibited(&action) => FilterResult::Forward,
                Some(action) if pressed => FilterResult::Intercept(action),
                _ => FilterResult::Forward,
            },
        );
        if let Some((sym, mods)) = seen {
            self.track_mod_hold(sym, &mods, pressed);
            self.track_click_tap(sym, &mods, pressed);
        }
        if let Some(action) = action {
            self.run_action(action);
        }
    }

    /// `dune pointer`: the pointer moved, or its buttons pressed, as a mouse would (an eye
    /// tracker, a voice); `ok`, why not, or for `Where` the place (`x y output`). Not while
    /// the session is locked.
    pub fn pointer_request(&mut self, op: PointerOp) -> String {
        if self.session_lock.is_locked() {
            return "the session is locked".into();
        }
        let here = self.seat.get_pointer().unwrap().current_location();
        let code = |b: MouseButton| match b {
            MouseButton::Left => crate::seatop::BTN_LEFT,
            MouseButton::Right => crate::seatop::BTN_RIGHT,
            MouseButton::Middle => BTN_MIDDLE,
        };
        match op {
            PointerOp::Move { x, y, output } => {
                let origin = match output {
                    Some(name) => match self.space.outputs().find(|o| o.name() == name).and_then(|o| self.space.output_geometry(o)) {
                        Some(geo) => geo.loc.to_f64(),
                        None => return format!("no monitor `{name}`"),
                    },
                    None => Point::from((0.0, 0.0)),
                };
                self.pointer_move_to(origin + Point::from((x as f64, y as f64)));
            }
            PointerOp::By { dx, dy } => {
                // A relative move, as a mouse's: pushing on against a left/right screen edge
                // slides the strip there (an eye tracker's head turn).
                let wanted = here + Point::from((dx as f64, dy as f64));
                let pos = self.clamp_to_outputs(wanted);
                let at_edge = (pos.x - here.x).abs() < 0.5;
                let slide = edge_slide(&mut self.edge_push, at_edge, wanted.x - pos.x);
                self.pointer_move_to(wanted);
                self.slide_strip(pos, slide);
            }
            PointerOp::Click(b) => {
                self.idle_notifier_state.notify_activity(&self.seat);
                self.pointer_button(code(b), ButtonState::Pressed, InputTime::now(), false);
                self.pointer_button(code(b), ButtonState::Released, InputTime::now(), false);
            }
            PointerOp::Press(b) | PointerOp::Release(b) => {
                self.idle_notifier_state.notify_activity(&self.seat);
                let state = if matches!(op, PointerOp::Press(_)) { ButtonState::Pressed } else { ButtonState::Released };
                self.pointer_button(code(b), state, InputTime::now(), false);
            }
            PointerOp::Scroll { dy } => {
                // Smooth, in pixels, as a touchpad's (an eye tracker's head tilt), to what is
                // under the pointer now (the layout may have moved under it).
                self.idle_notifier_state.notify_activity(&self.seat);
                let pointer = self.seat.get_pointer().unwrap();
                if !pointer.is_grabbed() {
                    let under = self.surface_under(here);
                    if under.as_ref().map(|(s, _)| s) != pointer.current_focus().as_ref() {
                        let event = MotionEvent { location: here, serial: SERIAL_COUNTER.next_serial(), time: InputTime::now() };
                        pointer.motion(self, under, &event);
                    }
                }
                let frame = AxisFrame::new(InputTime::now()).source(AxisSource::Continuous).value(Axis::Vertical, dy as f64);
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            PointerOp::Where => {
                // From the monitor's corner, as `move X Y OUTPUT` takes it and `dune events
                // pointer` tells it (the layout's when it is on none).
                let on = (self.space.outputs())
                    .find_map(|o| self.space.output_geometry(o).filter(|g| g.to_f64().contains(here)).map(|g| (o.name(), g.loc.to_f64())));
                let (output, at) = match on {
                    Some((name, corner)) => (name, here - corner),
                    None => (String::new(), here),
                };
                return format!("{} {} {output}", at.x.round(), at.y.round());
            }
        }
        "ok".into()
    }

    /// The pointer to `wanted` (kept on the screens), as an absolute motion of a mouse.
    fn pointer_move_to(&mut self, wanted: Point<f64, Logical>) {
        self.idle_notifier_state.notify_activity(&self.seat);
        let pos = self.clamp_to_outputs(wanted);
        self.pointer_at(pos);
        self.seatop_motion(pos);
        let pointer = self.seat.get_pointer().unwrap();
        let under = self.surface_under(pos);
        pointer.motion(self, under, &MotionEvent { location: pos, serial: SERIAL_COUNTER.next_serial(), time: InputTime::now() });
        pointer.frame(self);
        self.hover_focus(pos);
    }

    /// A mouse button going down or up, from a mouse or `dune pointer click`: Mod drags,
    /// the dock click, the tab bar's "+", click to focus, then the client under the pointer.
    /// `gestures`: whether held modifiers make it a Mod drag or the dock click. Only a real
    /// mouse's: a click dune makes (the right Ctrl+Shift tap, `dune pointer click`) is a plain
    /// click, even with Mod still down from the key that asked for it.
    pub fn pointer_button(&mut self, button: u32, state: ButtonState, time: InputTime, gestures: bool) {
        let locked = self.session_lock.is_locked();
        let pointer = self.seat.get_pointer().unwrap();
        let serial = SERIAL_COUNTER.next_serial();
        let button_state = state;
        if button_state == ButtonState::Pressed {
            self.mod_hold_button();
        }

        // Locked: the click only goes to the lock surface.
        if locked {
            let button = ButtonEvent { button, state: button_state, serial, time };
            pointer.button(self, &button);
            return pointer.frame(self);
        }

        // Zoomed, a button is a plain click: Mod held for the zoom key does not make it a
        // Mod drag (no float) or the dock click.
        let mouse = gestures;
        let gestures = gestures && !(button_state == ButtonState::Pressed && self.zoom.is_some());

        // `dune events pointer` (dune-eye-tracker learns from where you click): not while locked.
        if button_state == ButtonState::Pressed {
            // What pressed it: a real button, else the Ctrl+Shift tap (it says so), else a
            // command (`dune pointer click`).
            let by = match std::mem::take(&mut self.click_by) {
                _ if mouse => "mouse",
                "" => "command",
                by => by,
            };
            self.tell_click(button, by, pointer.current_location());
            // A click while zoomed (the mouse, a voice, right Ctrl+Shift) is where it
            // was meant to be: back out. The click goes on, in real coordinates as ever.
            self.zoom_out();
        }

        // A click away from dune-finder closes it, and goes no further.
        if button_state == ButtonState::Pressed
            && self.finder_under(pointer.current_location()) == Some(false)
        {
            return self.close_finder();
        }

        let mods = self.seat.get_keyboard().unwrap().modifier_state();
        let mod_held = gestures && modifier_held(&self.config.modifier, &mods);
        let dock = gestures && self.config.dock_click.is_some_and(|c| c.matches(&mods, button));

        // Mod+left and Mod+right drags (`crate::seatop`) take their buttons;
        // while one goes on, every button is its own.
        let pos = pointer.current_location();
        if self.seatop_button(button, button_state, pos, mod_held, dock) {
            return;
        }

        // `dock_click` puts the tile under the pointer into the dock. On a button Mod
        // drags with, only a click does (on release).
        if dock && button_state == ButtonState::Pressed && !(mod_held && [BTN_LEFT, BTN_RIGHT].contains(&button)) {
            let pos = pointer.current_location();
            if let Some(id) = self.tile_at(pos.x, pos.y).or_else(|| self.tab_at(pos.x, pos.y)) {
                return self.send_to_dock(id);
            }
        }

        // A click on the "+" of a tab bar: a new empty tab there, with dune-finder.
        if button == BTN_LEFT
            && button_state == ButtonState::Pressed
            && !pointer.is_grabbed()
        {
            let pos = pointer.current_location();
            if let Some(target) = self.plus_at(pos.x, pos.y) {
                return self.open_empty_tab(target);
            }
        }

        if button_state == ButtonState::Pressed {
            let pos = pointer.current_location();
            let under = self.surface_under(pos).map(|(s, o)| (smithay::reexports::wayland_server::Resource::id(&s), pos - o));
            let focus = pointer.current_focus().map(|s| smithay::reexports::wayland_server::Resource::id(&s));
            let window = self.space.element_under(pos).map(|(w, at)| (at, w.geometry(), w.bbox()));
            tracing::info!(?pos, ?under, ?focus, ?window, grabbed = pointer.is_grabbed(), "click");
        }

        // Click to focus.
        if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
            let pos = pointer.current_location();
            if let Some(id) = self.tab_at(pos.x, pos.y).or_else(|| self.tile_at(pos.x, pos.y)) {
                if self.focused != Some(id) {
                    self.focus_tile(id);
                } else if !self.geometries.iter().any(|(t, r)| *t == id && self.shown.contains(&(*t, *r))) {
                    // Focused already but partly off its monitor (slid there by hand):
                    // the view brings it back whole.
                    for layout in self.layouts.values_mut() {
                        if layout.contains(id) {
                            layout.stop_sliding();
                        }
                    }
                    self.arrange();
                }
            }
        };

        pointer.button(
            self,
            &ButtonEvent {
                button,
                state: button_state,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }

    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        // Any input but devices coming and going restarts swayidle's timers.
        if !matches!(event, InputEvent::DeviceAdded { .. } | InputEvent::DeviceRemoved { .. }) {
            self.idle_notifier_state.notify_activity(&self.seat);
        }
        let locked = self.session_lock.is_locked();
        match event {
            InputEvent::Keyboard { event, .. } => {
                self.handle_key(event.key_code(), event.state(), Event::time(&event));
            }
            InputEvent::PointerMotion { event, .. } => {
                // Relative motion (mice, touchpads on a real session), kept on the screens.
                let pointer = self.seat.get_pointer().unwrap();
                let current = pointer.current_location();
                let before = self.surface_under(current);
                let constraint = before.as_ref().and_then(|(surface, origin)| active_constraint(surface, &pointer, current - *origin));
                pointer.relative_motion(
                    self,
                    before.clone(),
                    &RelativeMotionEvent { delta: event.delta(), delta_unaccel: event.delta_unaccel(), time: event.time() },
                );
                // A locked pointer stays where it is: the game only gets the raw motion.
                if matches!(constraint, Some(Constraint::Locked)) {
                    return pointer.frame(self);
                }
                let wanted = current + event.delta();
                let mut pos = self.clamp_to_outputs(wanted);
                // Pushing on against a left/right screen edge slides the strip there (not for
                // a pointer a game confines, nor while locked).
                if constraint.is_none() && !locked {
                    let at_edge = (pos.x - current.x).abs() < 0.5;
                    let dx = edge_slide(&mut self.edge_push, at_edge, wanted.x - pos.x);
                    self.slide_strip(pos, dx);
                }
                // A confined one stays on its surface, inside the region.
                if let (Some(Constraint::Confined(region)), Some((surface, origin))) = (&constraint, &before) {
                    let inside = |p: Point<f64, Logical>| region.as_ref().is_none_or(|r| r.contains((p - *origin).to_i32_round()));
                    if !inside(Point::from((pos.x, current.y))) {
                        pos.x = current.x;
                    }
                    if !inside(Point::from((current.x, pos.y))) {
                        pos.y = current.y;
                    }
                    if self.surface_under(pos).is_none_or(|(s, _)| s != *surface) {
                        return pointer.frame(self);
                    }
                }
                self.pointer_at(pos);
                self.seatop_motion(pos);
                if !locked {
                    self.tell_motion(pos);
                }
                let under = self.surface_under(pos);
                pointer.motion(
                    self,
                    under.clone(),
                    &MotionEvent {
                        location: pos,
                        serial: SERIAL_COUNTER.next_serial(),
                        time: event.time(),
                    },
                );
                pointer.frame(self);
                if let Some((surface, origin)) = under {
                    activate_constraint(&surface, &pointer, pos - origin);
                }
                self.hover_focus(pos);
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next().unwrap();
                let output_geo = self.space.output_geometry(output).unwrap();
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                self.pointer_at(pos);
                self.seatop_motion(pos);

                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer().unwrap();
                let under = self.surface_under(pos);

                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time(),
                    },
                );
                pointer.frame(self);
                self.hover_focus(pos);
            }
            InputEvent::PointerButton { event, .. } => {
                self.pointer_button(event.button_code(), event.state(), event.time(), true);
            }
            InputEvent::PointerAxis { event, .. } => {
                // Shift+wheel scrolls sideways, Mod+wheel slides the strip: not a hold.
                self.mod_hold_button();
                let source = event.source();

                let horizontal_amount = event
                    .amount(Axis::Horizontal)
                    .unwrap_or_else(|| event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.);
                let vertical_amount = event
                    .amount(Axis::Vertical)
                    .unwrap_or_else(|| event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.);
                let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                // Always Super and Alt, whatever the bindings modifier; not while locked.
                // Super+Alt+scroll over a tile with neighbours side by side resizes it inside
                // its row; anywhere else the scroll goes to the window.
                let mods = self.seat.get_keyboard().unwrap().modifier_state();
                if !locked && mods.logo && mods.alt && !mods.ctrl && vertical_amount != 0.0 {
                    let pos = self.seat.get_pointer().unwrap().current_location();
                    let tile = self.tile_at(pos.x, pos.y);
                    if let Some(id) = tile.filter(|id| self.layouts.values().any(|l| l.shares_row(*id))) {
                        let delta = scroll_resize_delta(vertical_amount);
                        self.change_tile(id, |l, id| l.resize_row_width(id, delta));
                        return;
                    }
                }

                // Super+scroll (up/down or sideways) slides the strip of the monitor under the
                // pointer, focus staying where it is.
                if !locked && mods.logo && !mods.alt {
                    let pos = self.seat.get_pointer().unwrap().current_location();
                    // A wheel notch jumps 120 px: the tiles glide there instead.
                    if matches!(source, AxisSource::Wheel | AxisSource::WheelTilt) {
                        self.animate_slide();
                    }
                    self.slide_strip(pos, strip_scroll_dx(source, horizontal_amount, vertical_amount));
                    return;
                }

                // Alt+scroll resizes the tile under the pointer: the column's width, or with
                // Shift its height.
                if !locked && mods.alt && !mods.logo && !mods.ctrl && vertical_amount != 0.0 {
                    let pos = self.seat.get_pointer().unwrap().current_location();
                    if let Some(id) = self.tile_at(pos.x, pos.y).or(self.focused) {
                        let delta = scroll_resize_delta(vertical_amount);
                        if mods.shift {
                            self.resize_tile(id, 0.0, delta, |l, id| l.resize_height(id, delta));
                        } else {
                            self.resize_tile(id, delta, 0.0, |l, id| l.resize_width(id, delta));
                        }
                    }
                    return;
                }

                let pointer = self.seat.get_pointer().unwrap();
                // The layout may have moved under a still pointer (a strip slide, a desktop
                // switch, a window closing): the scroll goes to what is under it now.
                if !pointer.is_grabbed() {
                    let location = pointer.current_location();
                    let under = self.surface_under(location);
                    if under.as_ref().map(|(s, _)| s) != pointer.current_focus().as_ref() {
                        let event = MotionEvent { location, serial: SERIAL_COUNTER.next_serial(), time: event.time() };
                        pointer.motion(self, under, &event);
                    }
                }
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
