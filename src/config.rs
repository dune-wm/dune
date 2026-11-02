//! Settings from `~/.config/dune/dune.toml` (or `$XDG_CONFIG_HOME/dune/dune.toml`), in
//! TOML; dune-finder's are in `finder.toml` next to it (see dune-finder). Every setting is
//! optional:
//!
//! ```toml
//! terminal = "wezterm start --always-new-process"  # or foot, alacritty, ...
//! modifier = "alt"            # alt, super or ctrl+alt: `mod` in the keys below, and Mod+mouse
//! lock_command = "swaylock -f"  # what the `lock` key runs; default swaylock -f
//! follow = true               # go to the desktop an app is launched on (`firefox :d3`); default false
//! focus_follows_mouse = true  # the tile under the pointer takes focus (when all of it is on screen); default false
//! xwayland = "satellite"      # X11 apps through xwayland-satellite, or "off"
//! font = "JetBrains Mono"     # tab titles; default monospace
//! border_color = "#ffffff"    # border of tiles without focus: #RRGGBB or #RRGGBBAA, or "none"; default white
//! focus_color = "#5c9ef2"     # border of the focused tile (and Mod+click edges), the same way; default #5c9ef2
//! background_color = "#14141a"  # behind the tiles, #RRGGBB; default #14141a
//! border_width = 1            # px of the tile borders (0-16), also between tiles of a column; default 1
//! gap = 6                     # px around and between columns (0-64); default 6
//! corner_radius = 8           # px the corners of tiles are rounded by (0-64); default 0, square
//! exec = ["waybar", "mako"]   # run when dune starts, in order
//!
//! [keyboard]
//! layout = "us"
//! variant = "dvorak"
//! options = "caps:escape"     # also `model` and `rules`
//! repeat_delay = 500          # ms a held key waits before repeating (100-2000)
//! repeat_rate = 25            # repeats per second while held (1-100)
//!
//! [tabs]
//! min_width = 200             # px a tab shrinks to before its bar scrolls; default 200
//! max_width = 220             # px a tab grows to while there is room; default 220
//! height = 24                 # px of a tab bar (16-64); default 24
//! font_size = 14              # px of the tab titles (8-32); default 14
//! background = "#1e2029"      # behind the tabs, #RRGGBB
//! active = "#4c566a"          # the shown tab
//! text = "#d8dce6"            # the shown tab's title
//! inactive_text = "#9096a4"   # the other titles and the "+"
//!
//! [animations]
//! enabled = true              # false: resizes and closes are instant; default true
//! resize_ms = 150             # ms a resize slides for (0-1000, 0 none); default 150
//! slide_ms = 280              # ms the strip slides sideways for (a focus key, a head turn, Super+wheel) (0-1000, 0 none); default 280
//! close_ms = 150              # ms a closing tile shrinks and fades for (0-1000, 0 none); default 150
//! close_scale = 0.85          # how big a closing tile ends, as a share of its size (0.5-1.0); default 0.85
//!
//! [capture]
//! screenshot_dir = "~/shots"  # where the `screenshot` keys save; default $XDG_PICTURES_DIR/Screenshots, else ~/Pictures/Screenshots
//! recording_dir = "~/videos"  # where `record` saves; default $XDG_VIDEOS_DIR, else ~/Videos
//! record_audio = false        # recordings take the microphone too (`dune recording audio` flips it)
//! thumbnail_interval = 2      # s between the thumbnails of the tiles kept for shells (`$DUNE_THUMBNAILS`, 0-60, 0 off); default 2
//!
//! [mouse]
//! dock_click = "mod+right"    # the tile under the pointer (with its tabs) into the dock of its desktop; default none
//!
//! # Mice and touchpads (on the real session, not nested)
//! [input]
//! left_handed = true          # swap the left and right buttons
//! natural_scroll = false      # content follows the fingers
//! tap = true                  # tap to click on touchpads
//! accel_speed = 0.0           # pointer acceleration, -1.0 (slow) to 1.0 (fast)
//!
//! # Monitors, by connector name (dune logs them as `monitor HDMI-A-1: ...`)
//! [output.HDMI-A-1]
//! mode = "2560x1080@60"       # WIDTHxHEIGHT[@HZ]; default: the monitor's preferred mode
//! position = [0, 0]           # top-left corner, in logical pixels; unset ones go to the right of the rest
//! scale = 1.5                 # 0.5-4.0, fractional too; default 1
//! adaptive_sync = "fullscreen"  # VRR: on, off (default) or fullscreen (only while a tile is)
//! [output.DP-1]
//! off = true                  # leave this monitor dark
//!
//! # What new windows get, by app id and title (see `rules.rs`)
//! [[rules]]
//! app_id = "^pavucontrol$"    # a regex; `title = "..."` too, both must match
//! actions = ["float"]         # tile actions by their key names: float, center, full_width,
//!                             # send_to_desk_N, send_to_scratchpad, dock, private_tile, mute_tile, sticky, ...
//! ```
//!
//! A setting dune does not know, or a value that is not one (a key that is not a key, a
//! number out of range), is warned about in the log and the default stays. A file that is
//! not TOML, or a value of the wrong type (`follow = "yes"`), is reported with its line;
//! dune then starts with the defaults, or keeps the settings it has when the file is saved.
//!
//! Keys, in `[keys]`: every compositor key has a name, set like `close = ["super+q",
//! "super+'"]` (or one key, `close = "super+q"`). A value replaces that key's defaults;
//! `"none"` (or `[]`) turns it off. `mod` stands for `modifier`, the rest are `super`, `alt`,
//! `ctrl` and `shift`, then a key name as xkb writes it (`Return`, `slash`, `comma`,
//! `Escape`) or one character (`q`, `'`). Keys are the unshifted ones, so Mod+Shift+1 is
//! `mod+shift+1`, and `mod+E` is `mod+e`. A key two names share goes to the one listed
//! first here (a warning tells), so with `modifier = "ctrl+alt"` the Ctrl keys (`move_tab_*`,
//! `row_*`) give way. Ctrl+Alt+F1..F12 switch VTs on a real session, always. The defaults:
//!
//! ```toml
//! [keys]
//! focus_left = ["mod+h", "mod+Left"]  # focus the tile that way
//! focus_down = ["mod+j", "mod+Down"]
//! focus_up = ["mod+k", "mod+Up"]
//! focus_right = ["mod+l", "mod+Right"]
//! swap_left = ["mod+shift+h", "mod+shift+Left"]  # swap the tile with the one that way
//! swap_down = ["mod+shift+j", "mod+shift+Down"]
//! swap_up = ["mod+shift+k", "mod+shift+Up"]
//! swap_right = ["mod+shift+l", "mod+shift+Right"]
//! move_tab_left = ["mod+ctrl+h", "mod+ctrl+Left"]  # join the column that way as a tab, or leave a tabbed one
//! move_tab_down = ["mod+ctrl+j", "mod+ctrl+Down"]
//! move_tab_up = ["mod+ctrl+k", "mod+ctrl+Up"]
//! move_tab_right = ["mod+ctrl+l", "mod+ctrl+Right"]
//! focus_monitor_left = "none"         # focus the monitor that way (monitors stacked: mod+Up / mod+Down,
//! focus_monitor_down = "none"         # with those keys taken out of focus_up / focus_down)
//! focus_monitor_up = "none"
//! focus_monitor_right = "none"
//! focus_monitor_next = "none"         # the next monitor, top to bottom and left to right, then the first
//! move_monitor_next = "none"          # the focused tile to that next monitor
//! desk_1 = "mod+1"                    # show that desktop (desk_1 .. desk_9: mod+1 .. mod+9)
//! send_to_desk_1 = "mod+shift+1"      # send the tile there (send_to_desk_1 .. _9: mod+shift+1 .. 9)
//! empty_container = "mod+Return"      # an empty container, dune-finder to fill it
//! empty_tab = "mod+shift+t"           # one as the next tab of the focused tile's container
//! next_tab = "mod+Tab"
//! prev_tab = ["mod+shift+Tab", "mod+shift+ISO_Left_Tab"]
//! wider = ["mod+equal", "mod+plus"]   # the column's width
//! narrower = "mod+minus"
//! taller = ["mod+shift+equal", "mod+shift+plus"]  # the tile's height
//! shorter = ["mod+shift+minus", "mod+shift+underscore"]
//! row_wider = ["mod+ctrl+equal", "mod+ctrl+plus"]  # the tile's width inside its row
//! row_narrower = "mod+ctrl+minus"
//! cycle_width = "mod+r"
//! full_width = "mod+f"
//! fullscreen = "mod+shift+f"          # the tile over its whole monitor, bar too (again: back)
//! center = "mod+space"                # center the column (again: stop)
//! float = "mod+shift+space"           # float the tile (again: back into the strip)
//! send_to_scratchpad = "mod+shift+grave"  # the tile off every desktop, into the scratchpad
//! scratchpad = "mod+grave"            # the next scratchpad window floats here (on it again: back)
//! close = "mod+q"
//! close_group = "mod+shift+q"         # close every tile of the focused tile's column, tabs and all
//! launcher = "mod+d"                  # dune-finder on the apps
//! tile_options = "mod+slash"          # dune-finder on the focused tile's options (private, float, ...)
//! switcher = "none"                   # dune-finder on the windows (`/` in it gets there too)
//! private_tile = "none"               # block the focused tile on screen and in captures (again: show it)
//! mute_tile = "none"                  # mute the sound of the focused tile's app (again: unmute it)
//! sticky = "none"                     # the focused tile floats in front of all, on every desktop (again: off)
//! dock = "mod+shift+d"                # the focused tile (with its tabs) into the dock of its desktop
//! minimap = "mod+a"                  # dune-shell's minimap of the active monitor (again: hide it)
//! dock_show = "mod+shift+e"           # dune-shell shows the dock of the active monitor (again: hides it)
//! terminal_vertical = "mod+v"         # a terminal right of the tile, in its folder
//! terminal_horizontal = "mod+s"       # one below it
//! terminal_tab = "none"               # one as a new tab of its container
//! terminal_left = "none"              # one left of it
//! terminal_above = "none"             # one above it
//! stack_to_tabs = "mod+t"
//! quit = "none"                       # end the session at once (Log out is in dune-finder's System mode)
//! lock = "mod+Escape"                 # runs `lock_command`
//! shortcuts_inhibit = "mod+shift+Escape"  # a window that took every key (a VM) gives dune's back (again: lets it)
//! screenshot = "Print"                # of a region chosen with the mouse (Esc: none)
//! screenshot_tile = "ctrl+Print"      # of the focused tile
//! screenshot_screen = "shift+Print"   # of the active monitor
//! record = "mod+shift+r"              # record a region chosen with the mouse (any record key again: stop)
//! record_tile = "mod+ctrl+r"          # record the focused tile
//! record_screen = "mod+ctrl+shift+r"  # record the active monitor
//! ```
//!
//! Keys that run a command, like niri's `spawn`, go in `[spawn]`: the key, then the command
//! (through the shell, from the home folder). A key `[keys]` already has stays there.
//!
//! ```toml
//! [spawn]
//! "mod+shift+Return" = "vivaldi-stable"
//! ```
//!
//! The pointer pushing on against the left or right edge of the screens slides the strip
//! that way; it stays there until focus moves. A mark at the middle of an edge (`‹3`, `2›`)
//! tells how many tiles are past it, in white.
//!
//! dune-finder (a program of its own, installed next to dune) is the launcher and the
//! window list: Mod+d shows the apps, `/` typed first in them turns it into every window
//! to go to, Backspace with nothing typed turns it back (`:l` typed first lists the
//! windows of locked desktops instead; choosing one unlocks its desktop). Mod+/ shows the
//! options of the focused tile: private tile, float, center, full width, to tab, send to
//! a desktop, close, each with its key; typing narrows them, Enter does one. A click
//! chooses a row, the wheel moves through them, a click away closes it. Mod+Enter opens an
//! empty container with dune-finder for it: Enter right away puts a terminal (`terminal`)
//! there, a program chosen or typed goes there instead. `#` typed first shows the files of
//! the home folder: Enter opens a folder or a file (`xdg-open`), Mod+Enter the
//! `file_manager` (finder.toml) at it. `>` typed first jumps, like zoxide's `z`: the folders terminals
//! worked in (and the finder opened), the most used and recent first; Enter opens a
//! terminal there, Mod+Enter the `file_manager`.
//!
//! With Mod held, the pointer near an edge of a tile lights a band along that edge;
//! a Mod+click there opens an empty container on that side with a compact dune-finder (no
//! list until something is typed). A Mod+click elsewhere floats the tile.
//!
//! Tabs belong to columns (each tab with its own splits) and have keys (their only settings
//! are `[tabs]` `min_width` and `max_width`):
//! Mod+Tab / Mod+Shift+Tab show the next / previous tab, Mod+Ctrl+direction joins the
//! column that way as a tab (or leaves a tabbed one), Mod+t sends a tile out of its split
//! into a tab of its own (a lone tile: a bar with it as the one tab), a click on a tab
//! shows it, a Mod+drag let go on a tab bar adds the tile there as a tab, a tab Mod+dragged
//! along its own bar moves to the place it is let go, and dune-finder opens an app as a tab with
//! Ctrl+T or `:t`. The tab bar sits at the top left of its column (see
//! `layout::split_tab_bar`; its placement may become a setting).
//!
//! A right click on a desktop in the bar (waybar `ext/workspaces` with `"on-click-right":
//! "deactivate"`) locks it, "🔒 n": it is not shown (Mod+N, a click on it and the window
//! list leave it alone; locking the one shown switches to another) until another right
//! click, or a window chosen with `:l`, unlocks it. Tiles can still be sent there.
//!
//! Screenshots and recordings (see `screenshot`) need grim, slurp, wf-recorder,
//! wl-clipboard and libnotify (`notify-send`). A screenshot is saved as
//! `dune-YYYY-MM-DD_HH-MM-SS.png`, put on the clipboard and announced with its path; a
//! recording (`.mp4`, no sound) is announced when it starts and when it is saved. Bars
//! show a recording running with `dune recording watch` (waybar `custom` module,
//! `return-type: json`; `on-click: dune recording stop`).
//!
//! Night light has no settings here: programs like wlsunset (the waybar night-light
//! module runs it) set the monitors' gamma through wlr-gamma-control (see `gamma`), on
//! the real session only.
//!
//! Saving the file applies it live (see `reload`), except monitor modes, `off`, `xwayland`
//! and `exec` (programs start once, with dune).
//!
//! Environment variables win over the file: `XKB_DEFAULT_LAYOUT`, `XKB_DEFAULT_VARIANT`,
//! `XKB_DEFAULT_OPTIONS`, `XKB_DEFAULT_MODEL`, `XKB_DEFAULT_RULES`, `DUNE_TERMINAL`, `DUNE_MOD`.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use serde::Deserialize;
use smithay::input::keyboard::{Keysym, ModifiersState, XkbConfig, xkb};

use crate::layout::{Direction, TabWidths};


/// A key with its modifiers, written like `super+Right` or `ctrl+alt+h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySpec {
    pub logo: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub key: Keysym,
}

impl KeySpec {
    /// `mod` in `spec` stands for `modifier` (`alt`, `super` or `ctrl+alt`).
    pub fn parse(spec: &str, modifier: &str) -> Option<KeySpec> {
        let mut parts: Vec<&str> = spec.split('+').map(str::trim).collect();
        let name = parts.pop().filter(|n| !n.is_empty())?;
        let mut key = xkb::keysym_from_name(name, xkb::KEYSYM_CASE_INSENSITIVE);
        // A single character stands for its own key, like `'` for apostrophe.
        let mut chars = name.chars();
        if let (Some(c), None, Keysym::NoSymbol) = (chars.next(), chars.next(), key) {
            key = xkb::utf32_to_keysym(c as u32);
        }
        if key == Keysym::NoSymbol {
            return None;
        }
        // Keys come unshifted (Mod+Shift+e is `e` with shift held), so `E` is the `e` key.
        if let Some(c) = key.key_char() {
            let mut lower = c.to_lowercase();
            if let (Some(l), None) = (lower.next(), lower.next())
                && l != c
            {
                key = Keysym::from_char(l);
            }
        }
        let (logo, alt, ctrl, shift) = modifiers(&parts, modifier)?;
        Some(KeySpec { logo, alt, ctrl, shift, key })
    }

    /// Exactly these modifiers (so `super+Right` is not taken by `super+shift+Right`).
    pub fn matches(&self, mods: &ModifiersState, key: Keysym) -> bool {
        key == self.key && mods.logo == self.logo && mods.alt == self.alt && mods.ctrl == self.ctrl && mods.shift == self.shift
    }
}

/// The modifiers of a key or click (`super`, `alt`, `ctrl`, `shift`, `mod` for
/// `modifier`), as (logo, alt, ctrl, shift); `None` if one is not a modifier.
fn modifiers(parts: &[&str], modifier: &str) -> Option<(bool, bool, bool, bool)> {
    let (mut logo, mut alt, mut ctrl, mut shift) = (false, false, false, false);
    for m in parts {
        match m.to_lowercase().as_str() {
            "mod" => match modifier {
                "super" | "logo" => logo = true,
                "ctrl+alt" => (ctrl, alt) = (true, true),
                _ => alt = true,
            },
            "super" | "logo" | "mod4" => logo = true,
            "alt" | "mod1" => alt = true,
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            _ => return None,
        }
    }
    Some((logo, alt, ctrl, shift))
}

/// A mouse button with its modifiers, written like `mod+right` or `ctrl+middle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClickSpec {
    pub logo: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    /// The evdev code of the button (`BTN_LEFT`, ...).
    pub button: u32,
}

impl ClickSpec {
    /// The button is `left`, `right` or `middle`; `mod` stands for `modifier`.
    pub fn parse(spec: &str, modifier: &str) -> Option<ClickSpec> {
        let mut parts: Vec<&str> = spec.split('+').map(str::trim).collect();
        let button = match parts.pop()?.to_lowercase().as_str() {
            "left" => 0x110,
            "right" => 0x111,
            "middle" => 0x112,
            _ => return None,
        };
        let (logo, alt, ctrl, shift) = modifiers(&parts, modifier)?;
        Some(ClickSpec { logo, alt, ctrl, shift, button })
    }

    /// This button with exactly these modifiers.
    pub fn matches(&self, mods: &ModifiersState, button: u32) -> bool {
        button == self.button && mods.logo == self.logo && mods.alt == self.alt && mods.ctrl == self.ctrl && mods.shift == self.shift
    }
}


impl std::fmt::Display for KeySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (held, name) in [(self.logo, "super+"), (self.ctrl, "ctrl+"), (self.alt, "alt+"), (self.shift, "shift+")] {
            if held {
                f.write_str(name)?;
            }
        }
        f.write_str(&xkb::keysym_get_name(self.key))
    }
}

/// What a compositor key does; each one has a name in the file (see the table up top).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Focus(Direction),
    Swap(Direction),
    MoveTab(Direction),
    FocusMonitor(Direction),
    FocusMonitorNext,
    MoveMonitorNext,
    Desk(u8),
    SendToDesk(u8),
    EmptyContainer,
    EmptyTab,
    NextTab,
    PrevTab,
    Wider,
    Narrower,
    Taller,
    Shorter,
    RowWider,
    RowNarrower,
    CycleWidth,
    FullWidth,
    Fullscreen,
    Center,
    Float,
    SendToScratchpad,
    Scratchpad,
    Close,
    CloseGroup,
    Launcher,
    TileOptions,
    Switcher,
    PrivateTile,
    MuteTile,
    Sticky,
    Dock,
    DockShow,
    /// Ask the bar (dune-shell) to show or hide the minimap of the active monitor.
    Minimap,
    Terminal(TerminalHere),
    StackToTabs,
    Quit,
    Lock,
    /// Let the focused window have every key, compositor keys too, or take them back.
    ShortcutsInhibit,
    Screenshot,
    ScreenshotTile,
    ScreenshotScreen,
    Record,
    RecordTile,
    RecordScreen,
    /// Run the command at this place in `Config::spawn`.
    Spawn(usize),
}

/// Every binding: its name in the file, what it does and its default keys. A key two of
/// them share goes to the earlier one.
fn binding_table() -> Vec<(String, Binding, String)> {
    use Binding::*;
    let sides = [
        ("left", Direction::Left, "h", "Left"),
        ("down", Direction::Down, "j", "Down"),
        ("up", Direction::Up, "k", "Up"),
        ("right", Direction::Right, "l", "Right"),
    ];
    let mut table = Vec::new();
    for (prefix, mods, binding) in [
        ("focus", "mod", Focus as fn(Direction) -> Binding),
        ("swap", "mod+shift", Swap),
        ("move_tab", "mod+ctrl", MoveTab),
    ] {
        for (side, dir, letter, arrow) in sides {
            table.push((format!("{prefix}_{side}"), binding(dir), format!("{mods}+{letter}, {mods}+{arrow}")));
        }
    }
    table.extend(sides.map(|(side, dir, _, _)| (format!("focus_monitor_{side}"), FocusMonitor(dir), "none".to_string())));
    table.push(("focus_monitor_next".to_string(), FocusMonitorNext, "none".to_string()));
    table.push(("move_monitor_next".to_string(), MoveMonitorNext, "none".to_string()));
    table.extend((1..=9).map(|n| (format!("desk_{n}"), Desk(n), format!("mod+{n}"))));
    table.extend((1..=9).map(|n| (format!("send_to_desk_{n}"), SendToDesk(n), format!("mod+shift+{n}"))));
    let rest = [
        ("empty_container", EmptyContainer, "mod+Return"),
        ("empty_tab", EmptyTab, "mod+shift+t"),
        ("next_tab", NextTab, "mod+Tab"),
        ("prev_tab", PrevTab, "mod+shift+Tab, mod+shift+ISO_Left_Tab"),
        ("wider", Wider, "mod+equal, mod+plus"),
        ("narrower", Narrower, "mod+minus"),
        ("taller", Taller, "mod+shift+equal, mod+shift+plus"),
        ("shorter", Shorter, "mod+shift+minus, mod+shift+underscore"),
        ("row_wider", RowWider, "mod+ctrl+equal, mod+ctrl+plus"),
        ("row_narrower", RowNarrower, "mod+ctrl+minus"),
        ("cycle_width", CycleWidth, "mod+r"),
        ("full_width", FullWidth, "mod+f"),
        ("fullscreen", Fullscreen, "mod+shift+f"),
        ("center", Center, "mod+space"),
        ("float", Float, "mod+shift+space"),
        ("send_to_scratchpad", SendToScratchpad, "mod+shift+grave"),
        ("scratchpad", Scratchpad, "mod+grave"),
        ("close", Close, "mod+q"),
        ("close_group", CloseGroup, "mod+shift+q"),
        ("launcher", Launcher, "mod+d"),
        ("tile_options", TileOptions, "mod+slash"),
        ("switcher", Switcher, "none"),
        ("private_tile", PrivateTile, "none"),
        ("mute_tile", MuteTile, "none"),
        ("sticky", Sticky, "none"),
        ("dock", Dock, "mod+shift+d"),
        ("dock_show", DockShow, "mod+shift+e"),
        ("minimap", Minimap, "mod+a"),
        ("terminal_vertical", Terminal(TerminalHere::Right), "mod+v"),
        ("terminal_horizontal", Terminal(TerminalHere::Below), "mod+s"),
        ("terminal_tab", Terminal(TerminalHere::Tab), "none"),
        ("terminal_left", Terminal(TerminalHere::Left), "none"),
        ("terminal_above", Terminal(TerminalHere::Above), "none"),
        ("stack_to_tabs", StackToTabs, "mod+t"),
        ("quit", Quit, "none"),
        ("lock", Lock, "mod+Escape"),
        ("shortcuts_inhibit", ShortcutsInhibit, "mod+shift+Escape"),
        ("screenshot", Screenshot, "Print"),
        ("screenshot_tile", ScreenshotTile, "ctrl+Print"),
        ("screenshot_screen", ScreenshotScreen, "shift+Print"),
        ("record", Record, "mod+shift+r"),
        ("record_tile", RecordTile, "mod+ctrl+r"),
        ("record_screen", RecordScreen, "mod+ctrl+shift+r"),
    ];
    table.extend(rest.map(|(name, binding, keys)| (name.to_string(), binding, keys.to_string())));
    table
}

/// The keys of a name in `[keys]`: one, or a list.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Keys {
    One(String),
    Many(Vec<String>),
}

/// Keys separated by commas, or `none`; `None` if one of them is not a key.
fn key_list(value: &str, modifier: &str) -> Option<Vec<KeySpec>> {
    keys_of_specs(&value.split(',').collect::<Vec<_>>(), modifier)
}

/// Keys written one by one; `["none"]` or none at all: no key. `None` if one is not a key.
fn keys_of_specs(specs: &[&str], modifier: &str) -> Option<Vec<KeySpec>> {
    if specs.is_empty() || matches!(specs, [one] if one.trim().eq_ignore_ascii_case("none")) {
        return Some(Vec::new());
    }
    specs.iter().map(|spec| KeySpec::parse(spec, modifier)).collect()
}

/// The binding of that name in `[keys]` (`focus_left`, `desk_3`...), for `dune action`.
pub fn binding_named(name: &str) -> Option<Binding> {
    binding_table().into_iter().find(|(n, ..)| n == name).map(|(_, binding, _)| binding)
}

/// The keys of every binding, in table order: the file's in place of the defaults.
fn bindings(file: &BTreeMap<String, Keys>, spawn: &BTreeMap<String, String>, modifier: &str) -> Vec<(Binding, KeySpec)> {
    let table = binding_table();
    for name in file.keys().filter(|name| !table.iter().any(|(n, ..)| n == *name)) {
        tracing::warn!("ignoring `keys.{name}`: dune has no key by that name");
    }
    // With the name and whether the file set it, for the warnings.
    let mut taken: Vec<(String, Binding, KeySpec, bool)> = Vec::new();
    for (name, binding, defaults) in table {
        let set = file.get(&name).and_then(|value| {
            let specs: Vec<&str> = match value {
                Keys::One(key) => vec![key],
                Keys::Many(keys) => keys.iter().map(String::as_str).collect(),
            };
            let keys = keys_of_specs(&specs, modifier);
            if keys.is_none() {
                tracing::warn!("ignoring `keys.{name} = {specs:?}`: not keys like mod+q, super+shift+t or none");
            }
            keys
        });
        let from_file = set.is_some();
        let keys = set.unwrap_or_else(|| key_list(&defaults, modifier).unwrap_or_default());
        for key in keys {
            if let Some((other, other_binding, _, other_from_file)) = taken.iter().find(|(_, _, k, _)| *k == key) {
                // Defaults collapsing under `modifier = ctrl+alt` are expected, not news.
                if *other_binding != binding && (from_file || *other_from_file) {
                    tracing::warn!("`keys.{name}`: {key} already does `{other}`, it stays there");
                }
                continue;
            }
            taken.push((name.clone(), binding, key, from_file));
        }
    }
    // `[spawn]` after `[keys]`, in the order of `Config::spawn`.
    for (n, spec) in spawn.keys().enumerate() {
        let Some(key) = KeySpec::parse(spec, modifier) else {
            tracing::warn!("ignoring `spawn.\"{spec}\"`: not a key like mod+q or mod+shift+Return");
            continue;
        };
        if let Some((other, ..)) = taken.iter().find(|(_, _, k, _)| *k == key) {
            tracing::warn!("`spawn.\"{spec}\"`: {key} already does `{other}`, it stays there");
            continue;
        }
        taken.push((format!("spawn.\"{spec}\""), Binding::Spawn(n), key, true));
    }
    taken.into_iter().map(|(_, binding, key, _)| (binding, key)).collect()
}

/// `mouse.dock_click`: a click, or none (the default); a value that is not one is warned about.
fn dock_click(value: Option<&str>, modifier: &str) -> Option<ClickSpec> {
    let value = value.filter(|v| !v.eq_ignore_ascii_case("none"))?;
    let click = ClickSpec::parse(value, modifier);
    if click.is_none() {
        tracing::warn!("ignoring `mouse.dock_click = {value}`: not modifiers and left, right or middle, nor none");
    }
    click
}

/// Where the terminal keys (`terminal_vertical`, `_horizontal`, `_tab`) open a terminal,
/// next to the focused tile and in the folder its shell is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalHere {
    Right,
    Below,
    Left,
    Above,
    Tab,
}

/// A monitor mode written like `2560x1080@60` (the refresh rate is optional).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputMode {
    pub width: u16,
    pub height: u16,
    /// Hz, rounded.
    pub refresh: Option<u32>,
}

impl OutputMode {
    pub fn parse(text: &str) -> Option<OutputMode> {
        let (size, refresh) = match text.split_once('@') {
            Some((size, hz)) => (size, Some(hz.trim().parse::<f64>().ok()?.round() as u32)),
            None => (text, None),
        };
        let (w, h) = size.split_once('x')?;
        Some(OutputMode { width: w.trim().parse().ok()?, height: h.trim().parse().ok()?, refresh })
    }
}

/// Mice and touchpads (`[input]`); unset ones keep libinput's defaults.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    pub left_handed: Option<bool>,
    pub natural_scroll: Option<bool>,
    pub tap: Option<bool>,
    /// -1.0 to 1.0.
    pub accel_speed: Option<f64>,
}

/// Settings of one monitor (`[output.NAME]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OutputConfig {
    pub mode: Option<OutputMode>,
    pub position: Option<(i32, i32)>,
    /// Logical pixels are this many of the monitor's (1.25, 2); `None`: 1.
    pub scale: Option<f64>,
    pub adaptive_sync: AdaptiveSync,
    pub off: bool,
}

/// Variable refresh rate (FreeSync, G-Sync compatible) of a monitor that has it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdaptiveSync {
    #[default]
    Off,
    On,
    /// Only while a tile is fullscreen on it (a game), so the desktop never flickers.
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub keyboard_rules: String,
    pub keyboard_model: String,
    pub keyboard_layout: String,
    pub keyboard_variant: String,
    pub keyboard_options: String,
    /// Milliseconds a held key waits before it starts repeating.
    pub repeat_delay: i32,
    /// Repeats per second while a key stays held.
    pub repeat_rate: i32,
    pub terminal: String,
    pub modifier: String,
    /// Every compositor key and what it does, `mod` already read as `modifier`; a key is
    /// in it once.
    pub bindings: Vec<(Binding, KeySpec)>,
    /// What the `lock` key runs, through the shell.
    pub lock_command: String,
    /// The commands of `[spawn]`, by `Binding::Spawn`.
    pub spawn: Vec<String>,
    /// Show the desktop the launcher opens an app on, instead of staying here.
    pub follow: bool,
    /// The tile under the pointer takes focus as the pointer moves.
    pub focus_follows_mouse: bool,
    /// The click that puts the tile under the pointer into the dock.
    pub dock_click: Option<ClickSpec>,
    /// `satellite` (default) or `off`.
    pub xwayland: String,
    /// Monitors by connector name, like `HDMI-A-1`.
    pub outputs: BTreeMap<String, OutputConfig>,
    pub input: InputConfig,
    /// As fontconfig names it; tab titles are written with it.
    pub font_family: String,
    /// How narrow and how wide tabs get in their bars.
    pub tab_widths: TabWidths,
    /// How tall tab bars are, their text size and colors.
    pub tab_style: crate::tab_title::TabStyle,
    /// How long resizes and closes animate for, and how a closing tile shrinks.
    pub animations: Animations,
    /// The border of tiles without focus (premultiplied RGBA); `None`: not drawn.
    pub border_color: Option<[f32; 4]>,
    /// The border of the focused tile, the same way.
    pub focus_color: Option<[f32; 4]>,
    /// What the screens are cleared to behind the tiles (opaque RGBA).
    pub background_color: [f32; 4],
    /// Pixels of the tile borders, and between the tiles of a column.
    pub border_width: i32,
    /// Pixels around and between columns; the borders are drawn inside it.
    pub gap: i32,
    /// How many pixels the corners of tiles are rounded by; 0: square.
    pub corner_radius: i32,
    /// Shell commands run when dune starts (`exec`, in order).
    pub exec: Vec<String>,
    /// Where screenshots go (`~/` for the home folder); empty: `$XDG_PICTURES_DIR/Screenshots`.
    pub screenshot_dir: String,
    /// Where recordings go; empty: `$XDG_VIDEOS_DIR`.
    pub recording_dir: String,
    /// Whether recordings start taking the microphone too.
    pub record_audio: bool,
    /// Seconds between the thumbnails of the tiles (`crate::thumbnails`); 0: none.
    pub thumbnail_interval: u64,
    /// What new windows get by their app id and title (`[[rules]]`).
    pub rules: Vec<crate::rules::Rule>,
}

impl Config {
    pub fn xkb(&self) -> XkbConfig<'_> {
        XkbConfig {
            rules: &self.keyboard_rules,
            model: &self.keyboard_model,
            layout: &self.keyboard_layout,
            variant: &self.keyboard_variant,
            options: Some(self.keyboard_options.clone()).filter(|o| !o.is_empty()),
        }
    }

    /// The keys of `binding`, as the file writes them (`super+f, super+F11`); empty: none.
    pub fn key_label(&self, binding: Binding) -> String {
        let keys: Vec<String> = self.bindings.iter().filter(|(b, _)| *b == binding).map(|(_, k)| k.to_string()).collect();
        keys.join(", ")
    }

    /// Settings that differ from `old` but only take effect when dune starts again.
    pub fn restart_needed(&self, old: &Config) -> Vec<String> {
        let mut changed = Vec::new();
        if self.xwayland != old.xwayland {
            changed.push("xwayland".to_string());
        }
        if self.exec != old.exec {
            changed.push("exec".to_string());
        }
        let names = self.outputs.keys().chain(old.outputs.keys()).collect::<BTreeSet<_>>();
        for name in names {
            let new = self.outputs.get(name).cloned().unwrap_or_default();
            let old = old.outputs.get(name).cloned().unwrap_or_default();
            if new.mode != old.mode {
                changed.push(format!("output.{name}.mode"));
            }
            if new.off != old.off {
                changed.push(format!("output.{name}.off"));
            }
        }
        changed
    }
}

/// `dune.toml` as written: what is missing is `None` (or empty), for `resolve` to fill in.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct File {
    terminal: Option<String>,
    modifier: Option<String>,
    lock_command: Option<String>,
    follow: bool,
    focus_follows_mouse: bool,
    xwayland: Option<String>,
    font: Option<String>,
    border_color: Option<String>,
    focus_color: Option<String>,
    background_color: Option<String>,
    border_width: Option<i64>,
    gap: Option<i64>,
    corner_radius: Option<i64>,
    exec: Vec<String>,
    keyboard: KeyboardFile,
    tabs: TabsFile,
    animations: AnimationsFile,
    capture: CaptureFile,
    mouse: MouseFile,
    input: InputConfig,
    output: BTreeMap<String, OutputFile>,
    keys: BTreeMap<String, Keys>,
    spawn: BTreeMap<String, String>,
    rules: Vec<crate::rules::RuleFile>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct KeyboardFile {
    rules: Option<String>,
    model: Option<String>,
    layout: Option<String>,
    variant: Option<String>,
    options: Option<String>,
    repeat_delay: Option<i64>,
    repeat_rate: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AnimationsFile {
    enabled: Option<bool>,
    resize_ms: Option<i64>,
    slide_ms: Option<i64>,
    close_ms: Option<i64>,
    close_scale: Option<f64>,
}

/// `[animations]`: a zero duration is no animation (`enabled = false` makes both zero).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Animations {
    pub resize: std::time::Duration,
    /// The strip sliding sideways to another column.
    pub slide: std::time::Duration,
    pub close: std::time::Duration,
    /// How big a closing tile is by the end, as a share of its size.
    pub close_scale: f64,
}

impl Default for Animations {
    fn default() -> Self {
        let ms = std::time::Duration::from_millis;
        Animations { resize: ms(150), slide: ms(280), close: ms(150), close_scale: 0.85 }
    }
}

fn animations(file: &AnimationsFile) -> Animations {
    let default = Animations::default();
    let ms = |key: &str, value: Option<i64>, fallback: std::time::Duration| {
        let n = number(key, value, fallback.as_millis() as i32, 0..=1000);
        std::time::Duration::from_millis(if file.enabled == Some(false) { 0 } else { n as u64 })
    };
    let close_scale = match file.close_scale {
        Some(v) if (0.5..=1.0).contains(&v) => v,
        Some(v) => {
            tracing::warn!("ignoring `animations.close_scale = {v}`: not from 0.5 to 1.0");
            default.close_scale
        }
        None => default.close_scale,
    };
    Animations {
        resize: ms("animations.resize_ms", file.resize_ms, default.resize),
        slide: ms("animations.slide_ms", file.slide_ms, default.slide),
        close: ms("animations.close_ms", file.close_ms, default.close),
        close_scale,
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TabsFile {
    min_width: Option<i64>,
    max_width: Option<i64>,
    height: Option<i64>,
    font_size: Option<i64>,
    background: Option<String>,
    active: Option<String>,
    text: Option<String>,
    inactive_text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CaptureFile {
    screenshot_dir: String,
    recording_dir: String,
    record_audio: bool,
    thumbnail_interval: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct MouseFile {
    dock_click: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct OutputFile {
    mode: Option<String>,
    position: Option<[i32; 2]>,
    scale: Option<f64>,
    adaptive_sync: Option<String>,
    off: bool,
}

/// The config a file with `text` gives, without the environment.
#[cfg(test)]
pub fn from_text(text: &str) -> Config {
    resolve(&parse(text).unwrap(), &|_| None)
}

/// `~/.config/dune/dune.toml`.
pub fn path() -> Option<PathBuf> {
    Some(dune_common::config_dir()?.join("dune.toml"))
}

/// Every setting at its default (the environment still counts): no file, or one that is
/// not TOML when dune starts.
pub fn defaults() -> Config {
    resolve(&File::default(), &|key| std::env::var(key).ok())
}

/// The settings of `dune.toml`; `None` (reported) when it is not TOML or a value has the
/// wrong type. No file: the defaults.
pub fn load() -> Option<Config> {
    let path = path()?;
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    match parse(&text) {
        Ok(file) => Some(resolve(&file, &|key| std::env::var(key).ok())),
        Err(err) => {
            tracing::error!("{}: {err}", path.display());
            None
        }
    }
}

/// The file as TOML; a setting dune does not know is warned about.
fn parse(text: &str) -> Result<File, toml::de::Error> {
    let deserializer = toml::Deserializer::parse(text)?;
    serde_ignored::deserialize(deserializer, |setting| {
        tracing::warn!("ignoring `{setting}`: dune has no setting by that name");
    })
}

fn resolve(file: &File, env: &dyn Fn(&str) -> Option<String>) -> Config {
    let get = |env_key: &str, value: &Option<String>, default: &str| {
        env(env_key)
            .filter(|v| !v.is_empty())
            .or_else(|| value.clone())
            .unwrap_or_else(|| default.to_string())
    };
    let keyboard = &file.keyboard;
    // Nested inside another compositor Super is usually taken by the host.
    let modifier = get("DUNE_MOD", &file.modifier, "alt");
    Config {
        keyboard_rules: get("XKB_DEFAULT_RULES", &keyboard.rules, ""),
        keyboard_model: get("XKB_DEFAULT_MODEL", &keyboard.model, ""),
        keyboard_layout: get("XKB_DEFAULT_LAYOUT", &keyboard.layout, ""),
        keyboard_variant: get("XKB_DEFAULT_VARIANT", &keyboard.variant, ""),
        keyboard_options: get("XKB_DEFAULT_OPTIONS", &keyboard.options, ""),
        repeat_delay: number("keyboard.repeat_delay", keyboard.repeat_delay, 500, 100..=2000),
        repeat_rate: number("keyboard.repeat_rate", keyboard.repeat_rate, 25, 1..=100),
        // A new process per window: wezterm's single instance would open every window
        // from the first one, whose `DUNE_TILE` and pid no longer find the new tile.
        terminal: get("DUNE_TERMINAL", &file.terminal, "wezterm start --always-new-process"),
        modifier: modifier.clone(),
        xwayland: get("DUNE_XWAYLAND", &file.xwayland, "satellite"),
        bindings: bindings(&file.keys, &file.spawn, &modifier),
        spawn: file.spawn.values().cloned().collect(),
        lock_command: file.lock_command.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| "swaylock -f".into()),
        follow: file.follow,
        focus_follows_mouse: file.focus_follows_mouse,
        dock_click: dock_click(file.mouse.dock_click.as_deref(), &modifier),
        outputs: outputs(&file.output),
        input: input(&file.input),
        font_family: file.font.clone().filter(|f| !f.is_empty()).unwrap_or_else(|| "monospace".into()),
        tab_widths: tab_widths(&file.tabs),
        tab_style: tab_style(&file.tabs),
        animations: animations(&file.animations),
        border_color: color("border_color", file.border_color.as_deref(), WHITE),
        focus_color: color("focus_color", file.focus_color.as_deref(), FOCUS),
        background_color: color("background_color", file.background_color.as_deref(), BACKGROUND)
            .map_or(BACKGROUND, |[r, g, b, _]| [r, g, b, 1.0]),
        border_width: number("border_width", file.border_width, 1, 0..=16),
        gap: number("gap", file.gap, 6, 0..=64),
        corner_radius: number("corner_radius", file.corner_radius, 0, 0..=64),
        exec: file.exec.iter().filter(|c| !c.trim().is_empty()).cloned().collect(),
        screenshot_dir: file.capture.screenshot_dir.clone(),
        recording_dir: file.capture.recording_dir.clone(),
        record_audio: file.capture.record_audio,
        thumbnail_interval: number("capture.thumbnail_interval", file.capture.thumbnail_interval, 2, 0..=60) as u64,
        rules: crate::rules::rules(&file.rules),
    }
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// `focus_color` by default: #5c9ef2.
pub const FOCUS: [f32; 4] = [0.36, 0.62, 0.95, 1.0];
/// `background_color` by default: #14141a.
const BACKGROUND: [f32; 4] = [0.08, 0.08, 0.1, 1.0];

/// A color setting (`key`): `#RRGGBB`, `#RRGGBBAA` (the `#` optional) or `none`;
/// `default` when missing or not one. Premultiplied, as solid colour buffers take it.
fn color(key: &str, value: Option<&str>, default: [f32; 4]) -> Option<[f32; 4]> {
    let Some(value) = value else { return Some(default) };
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    let hex = value.trim_start_matches('#').trim_start_matches("0x");
    let channel = |i: usize| hex.get(i..i + 2).and_then(|c| u8::from_str_radix(c, 16).ok()).map(|c| c as f32 / 255.0);
    let rgba = match hex.len() {
        6 => (0..3).map(|i| channel(i * 2)).chain([Some(1.0)]).collect::<Option<Vec<_>>>(),
        8 => (0..4).map(|i| channel(i * 2)).collect(),
        _ => None,
    };
    let Some([r, g, b, a]) = rgba.and_then(|v| <[f32; 4]>::try_from(v).ok()) else {
        tracing::warn!("ignoring `{key} = {value}`: not #RRGGBB, #RRGGBBAA or none");
        return Some(default);
    };
    // Solid colour buffers take premultiplied alpha.
    Some([r * a, g * a, b * a, a])
}

/// `tabs.min_width` and `tabs.max_width`; a maximum under the minimum is raised to it.
fn tab_widths(tabs: &TabsFile) -> TabWidths {
    let default = TabWidths::default();
    let min = number("tabs.min_width", tabs.min_width, default.min, 40..=2000);
    let max = number("tabs.max_width", tabs.max_width, default.max, 40..=2000);
    if max < min {
        tracing::warn!("`tabs.max_width = {max}` is under `tabs.min_width = {min}`, using {min} for both");
    }
    TabWidths { min, max: max.max(min) }
}

/// The `[tabs]` look; what is missing or not one keeps its default.
fn tab_style(tabs: &TabsFile) -> crate::tab_title::TabStyle {
    let default = crate::tab_title::TabStyle::default();
    let rgb = |key: &str, value: &Option<String>, fallback: [u8; 3]| {
        let fallback_f = fallback.map(|c| c as f32 / 255.0);
        color(key, value.as_deref(), [fallback_f[0], fallback_f[1], fallback_f[2], 1.0])
            .map_or(fallback, |[r, g, b, _]| [r, g, b].map(|c| (c * 255.0).round() as u8))
    };
    crate::tab_title::TabStyle {
        height: number("tabs.height", tabs.height, default.height, 16..=64),
        font_size: number("tabs.font_size", tabs.font_size, default.font_size, 8..=32),
        background: rgb("tabs.background", &tabs.background, default.background),
        active: rgb("tabs.active", &tabs.active, default.active),
        text: rgb("tabs.text", &tabs.text, default.text),
        inactive_text: rgb("tabs.inactive_text", &tabs.inactive_text, default.inactive_text),
    }
}

fn number(key: &str, value: Option<i64>, default: i32, range: std::ops::RangeInclusive<i32>) -> i32 {
    let Some(value) = value else { return default };
    i32::try_from(value).ok().filter(|v| range.contains(v)).unwrap_or_else(|| {
        tracing::warn!("ignoring `{key} = {value}`: not a number from {} to {}", range.start(), range.end());
        default
    })
}

fn input(input: &InputConfig) -> InputConfig {
    let mut input = input.clone();
    if let Some(speed) = input.accel_speed.filter(|v| !(-1.0..=1.0).contains(v)) {
        tracing::warn!("ignoring `input.accel_speed = {speed}`: not from -1.0 to 1.0");
        input.accel_speed = None;
    }
    input
}

fn outputs(file: &BTreeMap<String, OutputFile>) -> BTreeMap<String, OutputConfig> {
    file.iter()
        .map(|(name, output)| {
            let mode = output.mode.as_deref().and_then(|text| {
                let mode = OutputMode::parse(text);
                if mode.is_none() {
                    tracing::warn!("ignoring `output.{name}.mode = {text}`: not WIDTHxHEIGHT[@HZ]");
                }
                mode
            });
            let scale = output.scale.and_then(|value| {
                let scale = crate::scale::valid_scale(value);
                if scale.is_none() {
                    let (min, max) = (crate::scale::MIN_SCALE, crate::scale::MAX_SCALE);
                    tracing::warn!("ignoring `output.{name}.scale = {value}`: not from {min} to {max}");
                }
                scale
            });
            let adaptive_sync = match output.adaptive_sync.as_deref().map(str::to_lowercase).as_deref() {
                None | Some("off") => AdaptiveSync::Off,
                Some("on") => AdaptiveSync::On,
                Some("fullscreen") => AdaptiveSync::Fullscreen,
                Some(other) => {
                    tracing::warn!("ignoring `output.{name}.adaptive_sync = {other}`: not on, off or fullscreen");
                    AdaptiveSync::Off
                }
            };
            let position = output.position.map(|[x, y]| (x, y));
            let config = OutputConfig { mode, position, scale, adaptive_sync, off: output.off };
            (name.clone(), config)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(text: &str) -> Config {
        from_text(text)
    }

    #[test]
    fn file_values_and_defaults() {
        let c = config("# my setup\n[keyboard]\nlayout = \"us\"\nvariant = \"dvorak\"  # comment\n");
        assert_eq!(c.keyboard_layout, "us");
        assert_eq!(c.keyboard_variant, "dvorak");
        assert_eq!(c.keyboard_options, "");
        assert_eq!(c.terminal, "wezterm start --always-new-process");
        assert_eq!(c.modifier, "alt");
    }

    #[test]
    fn output_scale_and_adaptive_sync() {
        let c = config("[output.DP-1]\nscale = 1.25\nadaptive_sync = \"Fullscreen\"\n[output.HDMI-A-1]\nscale = 9.0\nadaptive_sync = \"maybe\"\n");
        assert_eq!((c.outputs["DP-1"].scale, c.outputs["DP-1"].adaptive_sync), (Some(1.25), AdaptiveSync::Fullscreen));
        // Out of range or unknown: the defaults.
        assert_eq!((c.outputs["HDMI-A-1"].scale, c.outputs["HDMI-A-1"].adaptive_sync), (None, AdaptiveSync::Off));
    }

    #[test]
    fn shortcuts_inhibit_has_a_key() {
        assert_eq!(keys_of(&config(""), Binding::ShortcutsInhibit), [spec("alt+shift+Escape")]);
    }

    #[test]
    fn not_toml_or_a_wrong_type_is_an_error_with_its_line() {
        let err = parse("terminal = foot\n").unwrap_err();
        assert!(err.to_string().contains("line 1"), "{err}");
        let err = parse("modifier = \"super\"\nfollow = \"yes\"\n").unwrap_err();
        assert!(err.to_string().contains("line 2"), "{err}");
        // Unknown settings are not errors (they are warned about).
        assert!(parse("nonsense = 1\n[keyboard]\nlayot = \"us\"\n").is_ok());
    }

    #[test]
    fn key_specs() {
        let spec = KeySpec::parse("super+Right", "alt").unwrap();
        assert!(spec.logo && !spec.alt && spec.key == Keysym::Right);
        assert_eq!(KeySpec::parse("Ctrl + Alt + h", "alt").unwrap().key, Keysym::h);
        assert!(KeySpec::parse("hyper+Right", "alt").is_none());
        assert!(KeySpec::parse("super+NotAKey", "alt").is_none());
        assert_eq!(KeySpec::parse("super+'", "alt").unwrap().key, Keysym::apostrophe);
        let mods = |logo, shift| ModifiersState { logo, shift, ..Default::default() };
        assert!(spec.matches(&mods(true, false), Keysym::Right));
        assert!(!spec.matches(&mods(true, true), Keysym::Right));
        assert!(!spec.matches(&mods(false, false), Keysym::Right));
    }

    /// The keys `config` gives `binding`.
    fn keys_of(config: &Config, binding: Binding) -> Vec<KeySpec> {
        config.bindings.iter().filter(|(b, _)| *b == binding).map(|(_, k)| *k).collect()
    }

    fn spec(text: &str) -> KeySpec {
        KeySpec::parse(text, "alt").unwrap()
    }

    #[test]
    fn default_keys_follow_the_modifier() {
        for (modifier, held) in [("alt", "alt"), ("super", "super"), ("logo", "super"), ("ctrl+alt", "ctrl+alt")] {
            let c = config(&format!("modifier = \"{modifier}\"\n"));
            assert_eq!(keys_of(&c, Binding::Close), [spec(&format!("{held}+q"))], "{modifier}");
            assert_eq!(keys_of(&c, Binding::Focus(Direction::Left)), [spec(&format!("{held}+h")), spec(&format!("{held}+Left"))]);
            assert_eq!(keys_of(&c, Binding::SendToDesk(9)), [spec(&format!("{held}+shift+9"))]);
            assert_eq!(keys_of(&c, Binding::TileOptions), [spec(&format!("{held}+slash"))]);
            assert_eq!(keys_of(&c, Binding::Fullscreen), [spec(&format!("{held}+shift+f"))]);
        }
        // `DUNE_MOD` wins over the file, for the keys too.
        let env = |k: &str| (k == "DUNE_MOD").then(|| "super".to_string());
        let c = resolve(&parse("modifier = \"alt\"\n").unwrap(), &env);
        assert_eq!(keys_of(&c, Binding::Close), [spec("super+q")]);
        // Every binding has a key by default, but the windows (`/` in dune-finder), the
        // private switches (in the tile options), a terminal tab (Mod+Shift+t opens
        // an empty one) and the monitors (their keys depend on how they are placed).
        let c = config("");
        let unbound = [Binding::Switcher, Binding::PrivateTile, Binding::MuteTile, Binding::Sticky, Binding::Terminal(TerminalHere::Tab), Binding::Terminal(TerminalHere::Left), Binding::Terminal(TerminalHere::Above), Binding::Quit];
        for (name, binding, defaults) in binding_table() {
            let none = unbound.contains(&binding) || matches!(binding, Binding::FocusMonitor(_) | Binding::FocusMonitorNext | Binding::MoveMonitorNext);
            assert_eq!(keys_of(&c, binding).is_empty(), none, "{name}");
            assert!(key_list(&defaults, "alt").is_some(), "{name} = {defaults}");
        }
    }

    #[test]
    fn keys_spawn_commands() {
        let c = config("modifier = \"super\"\n[spawn]\n\"mod+shift+Return\" = \"vivaldi\"\n\"mod+q\" = \"foot\"\n\"nope+x\" = \"foot\"\n\"mod+b\" = \"vivaldi-stable --new-window 'https://example.com'\"\n");
        let Binding::Spawn(n) = c.bindings.iter().find(|(_, k)| *k == spec("super+shift+Return")).unwrap().0 else { panic!() };
        assert_eq!(c.spawn[n], "vivaldi");
        // The command goes to the shell whole, arguments and quotes included.
        let Binding::Spawn(n) = c.bindings.iter().find(|(_, k)| *k == spec("super+b")).unwrap().0 else { panic!() };
        assert_eq!(c.spawn[n], "vivaldi-stable --new-window 'https://example.com'");
        // A key `[keys]` has stays there; one that is not a key is left out.
        assert_eq!(keys_of(&c, Binding::Close), [spec("super+q")]);
        assert_eq!(c.bindings.iter().filter(|(b, _)| matches!(b, Binding::Spawn(_))).count(), 2);
    }

    #[test]
    fn dock_click_is_a_button_with_modifiers() {
        let c = config("modifier = \"super\"\n[mouse]\ndock_click = \"mod+right\"\n");
        let click = c.dock_click.unwrap();
        assert_eq!((click.logo, click.alt, click.ctrl, click.shift, click.button), (true, false, false, false, 0x111));
        assert_eq!(ClickSpec::parse("ctrl+shift+Middle", "alt").map(|c| (c.ctrl, c.shift, c.button)), Some((true, true, 0x112)));
        // None by default; `none`, a key or an unknown modifier: none.
        for value in ["none", "mod+q", "hyper+left"] {
            assert_eq!(config(&format!("[mouse]\ndock_click = \"{value}\"\n")).dock_click, None, "{value}");
        }
        assert_eq!(config("").dock_click, None);
    }

    #[test]
    fn monitor_focus_takes_keys_given_up_by_tile_focus() {
        let c = config("[keys]\nfocus_up = \"mod+k\"\nfocus_monitor_up = \"mod+Up\"\n");
        assert_eq!(keys_of(&c, Binding::Focus(Direction::Up)), [spec("alt+k")]);
        assert_eq!(keys_of(&c, Binding::FocusMonitor(Direction::Up)), [spec("alt+Up")]);
    }

    #[test]
    fn a_value_replaces_the_defaults() {
        let c = config("modifier = \"super\"\n[keys]\nclose = [\"super+q\", \"super+'\"]\nlauncher = \"mod+e\"\n");
        assert_eq!(keys_of(&c, Binding::Close), [spec("super+q"), spec("super+apostrophe")]);
        assert_eq!(keys_of(&c, Binding::Launcher), [spec("super+e")]);
        // The others keep theirs.
        assert_eq!(keys_of(&c, Binding::TileOptions), [spec("super+slash")]);
        let c = config("[keys]\nterminal_vertical = \"super+ctrl+v\"\n");
        assert_eq!(keys_of(&c, Binding::Terminal(TerminalHere::Right)), [spec("super+ctrl+v")]);
        assert_eq!(keys_of(&c, Binding::Terminal(TerminalHere::Below)), [spec("alt+s")]);
    }

    #[test]
    fn none_turns_a_key_off() {
        let c = config("[keys]\nquit = \"none\"\nlock = \"None\"\nclose = []\n");
        assert!(keys_of(&c, Binding::Quit).is_empty());
        assert!(keys_of(&c, Binding::Lock).is_empty());
        assert!(keys_of(&c, Binding::Close).is_empty());
        assert!(!c.bindings.iter().any(|(_, k)| *k == spec("alt+q")));
    }

    #[test]
    fn keys_are_unshifted_and_lowercase() {
        let c = config("[keys]\nlock = \"mod+E\"\nquit = \"super+shift+Q\"\n");
        assert_eq!(keys_of(&c, Binding::Lock), [spec("alt+e")]);
        assert_eq!(keys_of(&c, Binding::Quit), [spec("super+shift+q")]);
        assert_eq!(spec("alt+e").key, Keysym::e);
    }

    #[test]
    fn a_bad_value_keeps_the_defaults() {
        for value in ["\"hyper+q\"", "[\"mod+q\", \"mod+NotAKey\"]", "\"\"", "\"mod+\""] {
            let c = config(&format!("[keys]\nclose = {value}\n"));
            assert_eq!(keys_of(&c, Binding::Close), [spec("alt+q")], "{value:?}");
        }
    }

    #[test]
    fn ctrl_keys_give_way_under_ctrl_alt() {
        let c = config("modifier = \"ctrl+alt\"\n");
        let ctrl_alt_left = spec("ctrl+alt+Left");
        assert_eq!(c.bindings.iter().find(|(_, k)| *k == ctrl_alt_left).map(|(b, _)| *b), Some(Binding::Focus(Direction::Left)));
        assert!(keys_of(&c, Binding::MoveTab(Direction::Left)).is_empty());
        assert!(keys_of(&c, Binding::RowWider).is_empty());
        assert_eq!(keys_of(&c, Binding::Wider), [spec("ctrl+alt+equal"), spec("ctrl+alt+plus")]);
    }

    #[test]
    fn scratchpad_keys() {
        let c = config("");
        assert_eq!(keys_of(&c, Binding::Scratchpad), [spec("alt+grave")]);
        assert_eq!(keys_of(&c, Binding::SendToScratchpad), [spec("alt+shift+grave")]);
        let c = config("[keys]\nscratchpad = \"none\"\n");
        assert!(keys_of(&c, Binding::Scratchpad).is_empty());
    }

    #[test]
    fn tile_options_and_private_keys() {
        let c = config("");
        assert_eq!(keys_of(&c, Binding::TileOptions), [spec("alt+slash")]);
        assert!(keys_of(&c, Binding::Switcher).is_empty());
        assert!(keys_of(&c, Binding::PrivateTile).is_empty());
        // Still settable; a `switcher` set to Mod+/ gives way to `tile_options`, listed first.
        let c = config("[keys]\nswitcher = \"mod+w\"\nprivate_tile = \"mod+p\"\n");
        assert_eq!(keys_of(&c, Binding::Switcher), [spec("alt+w")]);
        assert_eq!(keys_of(&c, Binding::PrivateTile), [spec("alt+p")]);
        // `private_desktop` is gone (desktops lock from the bar): an unknown key, ignored.
        assert!(!binding_table().iter().any(|(name, ..)| name == "private_desktop"));
        let c = config("[keys]\nprivate_desktop = \"mod+shift+p\"\n");
        assert!(!c.bindings.iter().any(|(_, k)| *k == spec("alt+shift+p")));
        let c = config("[keys]\nswitcher = \"mod+slash\"\n");
        assert_eq!(keys_of(&c, Binding::TileOptions), [spec("alt+slash")]);
        assert!(keys_of(&c, Binding::Switcher).is_empty());
    }

    #[test]
    fn bindings_by_name() {
        assert_eq!(binding_named("desk_3"), Some(Binding::Desk(3)));
        assert_eq!(binding_named("focus_left"), Some(Binding::Focus(Direction::Left)));
        assert_eq!(binding_named("close"), Some(Binding::Close));
        assert_eq!(binding_named("desk_10"), None);
        assert_eq!(binding_named(""), None);
    }

    #[test]
    fn key_labels() {
        let c = config("modifier = \"super\"\n[keys]\nclose = [\"mod+q\", \"super+'\"]\n");
        assert_eq!(c.key_label(Binding::Close), "super+q, super+apostrophe");
        assert_eq!(c.key_label(Binding::PrivateTile), "");
    }

    #[test]
    fn lock_key_and_command() {
        let c = config("");
        assert_eq!(keys_of(&c, Binding::Lock), [spec("alt+Escape")]);
        assert_eq!(keys_of(&c, Binding::Lock)[0].key, Keysym::Escape);
        assert_eq!(c.lock_command, "swaylock -f");
        let c = config("lock_command = \"swaylock -f -c 000000\"\n");
        assert_eq!(c.lock_command, "swaylock -f -c 000000");
    }

    #[test]
    fn capture_keys_and_folders() {
        let c = config("");
        assert_eq!(keys_of(&c, Binding::Screenshot), [spec("Print")]);
        assert_eq!(keys_of(&c, Binding::ScreenshotScreen), [spec("shift+Print")]);
        assert_eq!(keys_of(&c, Binding::Record), [spec("alt+shift+r")]);
        assert_eq!(keys_of(&c, Binding::ScreenshotTile), [spec("ctrl+Print")]);
        assert_eq!(keys_of(&c, Binding::RecordTile), [spec("alt+ctrl+r")]);
        assert_eq!(keys_of(&c, Binding::RecordScreen), [spec("alt+ctrl+shift+r")]);
        assert_eq!(keys_of(&c, Binding::CycleWidth), [spec("alt+r")]);
        assert_eq!((c.screenshot_dir.as_str(), c.recording_dir.as_str()), ("", ""));
        let c = config("modifier = \"super\"\n[keys]\nscreenshot = \"super+p\"\nrecord = \"none\"\n");
        assert_eq!(keys_of(&c, Binding::Screenshot), [spec("super+p")]);
        assert!(keys_of(&c, Binding::Record).is_empty());
        assert_eq!(keys_of(&c, Binding::ScreenshotScreen), [spec("shift+Print")]);
        let c = config("[capture]\nscreenshot_dir = \"~/shots\"\nrecording_dir = \"/data/videos\"\n");
        assert_eq!(c.screenshot_dir, "~/shots");
        assert_eq!(c.recording_dir, "/data/videos");
        assert_eq!(c.thumbnail_interval, 2);
        assert_eq!(config("[capture]\nthumbnail_interval = 0\n").thumbnail_interval, 0);
        assert_eq!(config("[capture]\nthumbnail_interval = 600\n").thumbnail_interval, 2);
    }

    #[test]
    fn a_shared_key_stays_with_the_first() {
        // `close` comes before `lock`: alt+q stays a close key, lock is left with none.
        let c = config("[keys]\nlock = \"mod+q\"\n");
        assert_eq!(keys_of(&c, Binding::Close), [spec("alt+q")]);
        assert!(keys_of(&c, Binding::Lock).is_empty());
        // Set on the earlier one, it takes the key from the later one's defaults.
        let c = config("[keys]\nfocus_left = \"mod+t\"\n");
        assert_eq!(keys_of(&c, Binding::Focus(Direction::Left)), [spec("alt+t")]);
        assert!(keys_of(&c, Binding::StackToTabs).is_empty());
        // Each key once.
        let keys: Vec<_> = c.bindings.iter().map(|(_, k)| *k).collect();
        assert!(keys.iter().enumerate().all(|(i, k)| !keys[..i].contains(k)));
    }

    #[test]
    fn exec_is_a_list() {
        let c = config("exec = [\"waybar -c ~/.config/waybar/dune.jsonc\", \"mako\", \"\"]\n");
        assert_eq!(c.exec, ["waybar -c ~/.config/waybar/dune.jsonc", "mako"]);
        assert!(config("").exec.is_empty());
    }

    #[test]
    fn key_repeat() {
        let c = config("");
        assert_eq!((c.repeat_delay, c.repeat_rate), (500, 25));
        let c = config("[keyboard]\nrepeat_delay = 600\nrepeat_rate = 30\n");
        assert_eq!((c.repeat_delay, c.repeat_rate), (600, 30));
        let c = config("[keyboard]\nrepeat_delay = 5\nrepeat_rate = 99999999999\n");
        assert_eq!((c.repeat_delay, c.repeat_rate), (500, 25));
    }

    #[test]
    fn follow() {
        assert!(!config("").follow);
        assert!(config("follow = true\n").follow);
        assert!(!config("").focus_follows_mouse);
        assert!(config("focus_follows_mouse = true\n").focus_follows_mouse);
    }

    #[test]
    fn font_and_tab_widths() {
        let c = config("");
        assert_eq!((c.font_family.as_str(), c.tab_widths), ("monospace", TabWidths::default()));
        let c = config("font = \"JetBrains Mono\"\n[tabs]\nmin_width = 120\nmax_width = 300\n");
        assert_eq!((c.font_family.as_str(), c.tab_widths), ("JetBrains Mono", TabWidths { min: 120, max: 300 }));
        // Out of range: the default; a maximum under the minimum: the minimum.
        let c = config("[tabs]\nmin_width = 5\nmax_width = 100\n");
        assert_eq!(c.tab_widths, TabWidths { min: TabWidths::default().min, max: TabWidths::default().min });
    }

    #[test]
    fn corner_radius() {
        assert_eq!(config("").corner_radius, 0);
        assert_eq!(config("corner_radius = 8\n").corner_radius, 8);
        assert_eq!(config("corner_radius = 100\n").corner_radius, 0);
        assert_eq!(config("corner_radius = -1\n").corner_radius, 0);
    }

    #[test]
    fn border_color() {
        assert_eq!(config("").border_color, Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(config("border_color = \"none\"\n").border_color, None);
        assert_eq!(config("border_color = \"#ff0000\"\n").border_color, Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(config("border_color = \"ff0000\"\n").border_color, Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(config("border_color = \"#ffffff00\"\n").border_color, Some([0.0, 0.0, 0.0, 0.0]));
        assert_eq!(config("border_color = \"blue\"\n").border_color, Some([1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn animations() {
        use std::time::Duration;
        assert_eq!(config("").animations, Animations::default());
        let a = config("[animations]\nresize_ms = 300\nclose_ms = 0\nclose_scale = 0.6\n").animations;
        assert_eq!((a.resize, a.close, a.close_scale), (Duration::from_millis(300), Duration::ZERO, 0.6));
        let a = config("[animations]\nslide_ms = 400\n").animations;
        assert_eq!((a.resize, a.slide), (Duration::from_millis(150), Duration::from_millis(400)));
        let a = config("[animations]\nenabled = false\nresize_ms = 300\n").animations;
        assert_eq!((a.resize, a.slide, a.close), (Duration::ZERO, Duration::ZERO, Duration::ZERO));
        let a = config("[animations]\nresize_ms = 5000\nclose_scale = 2.0\n").animations;
        assert_eq!((a.resize, a.close_scale), (Duration::from_millis(150), 0.85));
    }

    #[test]
    fn tab_style() {
        use crate::tab_title::TabStyle;
        assert_eq!(config("").tab_style, TabStyle::default());
        let c = config("[tabs]\nheight = 30\nfont_size = 16\nbackground = \"#000000\"\nactive = \"#ff0000\"\ntext = \"blue\"\n");
        let s = c.tab_style;
        assert_eq!((s.height, s.font_size, s.background, s.active), (30, 16, [0, 0, 0], [255, 0, 0]));
        assert_eq!(s.text, TabStyle::default().text);
        assert_eq!(config("[tabs]\nheight = 99\n").tab_style.height, TabStyle::default().height);
    }

    #[test]
    fn look() {
        let c = config("");
        assert_eq!((c.focus_color, c.background_color, c.border_width, c.gap), (Some(FOCUS), BACKGROUND, 1, 6));
        let c = config("focus_color = \"none\"\nbackground_color = \"#ff000080\"\nborder_width = 3\ngap = 0\n");
        assert_eq!((c.focus_color, c.background_color, c.border_width, c.gap), (None, [0.5019608, 0.0, 0.0, 1.0], 3, 0));
        let c = config("background_color = \"none\"\nborder_width = 17\ngap = -1\n");
        assert_eq!((c.background_color, c.border_width, c.gap), (BACKGROUND, 1, 6));
    }

    #[test]
    fn outputs() {
        let c = config(
            "[output.HDMI-A-1]\nmode = \"2560x1080@59.94\"\nposition = [0, 0]\n\
             [output.HDMI-A-2]\nmode = \"1920x1080\"\nposition = [311, 1080]\n\
             [output.DP-1]\noff = true\n[output.DP-2]\nmode = \"big\"\n",
        );
        let a = &c.outputs["HDMI-A-1"];
        assert_eq!(a.mode, Some(OutputMode { width: 2560, height: 1080, refresh: Some(60) }));
        assert_eq!(a.position, Some((0, 0)));
        let b = &c.outputs["HDMI-A-2"];
        assert_eq!(b.mode, Some(OutputMode { width: 1920, height: 1080, refresh: None }));
        assert_eq!(b.position, Some((311, 1080)));
        assert!(c.outputs["DP-1"].off && !a.off);
        assert_eq!(c.outputs["DP-2"].mode, None); // invalid: the monitor's preferred mode
    }

    #[test]
    fn input_settings() {
        let c = config("[input]\nleft_handed = true\nnatural_scroll = false\naccel_speed = 2.0\n");
        assert_eq!(c.input.left_handed, Some(true));
        assert_eq!(c.input.natural_scroll, Some(false));
        assert_eq!(c.input.accel_speed, None); // out of range: libinput's default
        assert_eq!(c.input.tap, None);
        assert_eq!(config("").input, InputConfig::default());
    }

    #[test]
    fn restart_needed() {
        let old = config("xwayland = \"off\"\n[output.A]\nmode = \"1920x1080\"\nposition = [0, 0]\n");
        let new = config(
            "xwayland = \"off\"\nmodifier = \"super\"\n[output.A]\nmode = \"2560x1080\"\nposition = [10, 0]\n[output.B]\noff = true\n",
        );
        assert_eq!(new.restart_needed(&old), ["output.A.mode", "output.B.off"]);
        assert!(old.restart_needed(&old).is_empty());
    }

    #[test]
    fn environment_wins() {
        let file = parse("terminal = \"alacritty\"\n[keyboard]\nvariant = \"dvorak\"\n").unwrap();
        let env = |k: &str| match k {
            "XKB_DEFAULT_VARIANT" => Some("colemak".to_string()),
            "DUNE_TERMINAL" => Some(String::new()), // empty means unset
            _ => None,
        };
        let c = resolve(&file, &env);
        assert_eq!(c.keyboard_variant, "colemak");
        assert_eq!(c.terminal, "alacritty");
    }
}
