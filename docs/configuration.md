# Configuration

`dune.toml`, `finder.toml` and every default key.

- [The files](#the-files)
- [Window rules](#window-rules)
- [Default keys](#default-keys)

## The files

Two TOML files in `~/.config/dune/` (or `$XDG_CONFIG_HOME/dune/`): `dune.toml` for the
window manager and `finder.toml` for dune-finder. Every setting is optional. Saving
`dune.toml` applies it live, except monitor modes, `off`, `xwayland` and `exec`;
dune-finder reads `finder.toml` each time it opens.

A monitor's `scale` makes everything on it bigger (1.25, 1.5, 2) and applies live. Apps
that know fractional scaling (GTK4, Qt6, Firefox, Chromium with Wayland) draw at the
monitor's real pixels and stay sharp. Others are scaled up and look a little soft, and so
do X11 apps and dune's own tab titles for now.

```toml
# ~/.config/dune/dune.toml
terminal = "wezterm start --always-new-process"
modifier = "alt"               # alt (default), super or ctrl+alt: `mod` below
lock_command = "swaylock -f"
follow = true                  # go to the desktop an app is launched on
focus_follows_mouse = true
xwayland = "satellite"         # or "off"
font = "JetBrains Mono"        # tab titles
border_color = "none"          # "#RRGGBB", "#RRGGBBAA" or "none"
corner_radius = 8              # px the corners of tiles round by, 0-64 (default 0: square)
exec = ["waybar", "mako"]      # programs started with dune, in order

[keyboard]
layout = "us"
variant = "dvorak"
repeat_delay = 500             # ms before a held key repeats
repeat_rate = 25               # repeats per second

[tabs]
min_width = 200
max_width = 220

[capture]
screenshot_dir = "~/shots"
recording_dir = "~/videos"

[mouse]
dock_click = "mod+right"

[input]                        # mice and touchpads (real session)
left_handed = true
natural_scroll = false
tap = true
accel_speed = 0.0              # -1.0 .. 1.0

[output.HDMI-A-1]              # monitors, by connector name
mode = "2560x1080@60"
position = [0, 0]              # in logical pixels (the monitor's divided by its scale)
scale = 1.25                   # 0.5-4.0, fractional too; default 1
adaptive_sync = "fullscreen"   # VRR: "on", "off" (default), "fullscreen" (only while a tile is)

[output.DP-1]
off = true

[keys]
close = ["super+q", "super+'"]
launcher = "super+e"
switcher = "none"

[spawn]
"mod+shift+Return" = "vivaldi"

[[rules]]                      # see Window rules below
app_id = "^pavucontrol$"
actions = ["float"]
```

```toml
# ~/.config/dune/finder.toml
font = "JetBrains Mono"
icon_theme = "Papirus-Dark"
file_manager = "yazi"
file_manager_terminal = "foot"
```

Every key can be rebound in `[keys]`, with one key or a list; a value replaces that key's
defaults and `"none"` (or `[]`) turns it off. The names and defaults are listed at the top
of `src/config.rs`. Keys that run a command, like niri's `spawn`, go in `[spawn]`: the
key, then the command (run through the shell, from the home folder); a key `[keys]` already
has stays with it. A setting dune does not know, or a value it cannot use, is warned about in
the log; a file that is not TOML is reported with its line, and the settings in use stay.
Environment variables win over the file: `XKB_DEFAULT_*`, `DUNE_TERMINAL`, `DUNE_MOD`,
`DUNE_XWAYLAND`.

## Window rules

A `[[rules]]` table tells dune what to do with a new window, by its app id (an X11
app: its class) and title. Both are regexes, and when a rule has both, both must match.
`actions` lists tile options (Mod+/) by their key names: `float`, `center`,
`full_width`, `stack_to_tabs`, `send_to_desk_1` .. `send_to_desk_9`, `dock`,
`send_to_scratchpad`, `private_tile`, `mute_tile`, `close`.

```toml
[[rules]]
app_id = "^pavucontrol$"
actions = ["float"]

[[rules]]
title = "Picture-in-Picture"
actions = ["float", "private_tile"]

[[rules]]
app_id = "(?i)^steam$"         # (?i): any case
actions = ["send_to_desk_3"]
```

Rules run once, when the window first shows. They only apply to a window that gets its
own tile on the desktop you are looking at. A window that opens as a tab, in a
terminal's tile or on another desktop stays where it was put. Every matching rule runs,
in the order of the file. `float` leaves a window that already floats (a dialog)
floating. An action that sends the tile away (to a desktop, the dock or the
scratchpad) ends the list. A rule with a bad regex, or with neither `app_id` nor
`title`, is skipped with a warning, and so is an unknown action. The log names each
window's app id (`focus ... app=...`), which helps when writing rules.

## Default keys

`Mod` is `modifier` (Alt by default). The first column is the name to rebind it with in
`[keys]`.

| Name | Default | Does |
|---|---|---|
| `focus_left` / `_down` / `_up` / `_right` | `Mod+h/j/k/l`, arrows | focus |
| `swap_left` / `_down` / `_up` / `_right` | `Mod+Shift+h/j/k/l`, arrows | swap |
| `move_tab_left` / `_down` / `_up` / `_right` | `Mod+Ctrl+h/j/k/l`, arrows | join / leave tabs |
| `focus_monitor_left` / `_down` / `_up` / `_right` | none | focus the monitor that way |
| `desk_1` .. `desk_9` | `Mod+1..9` | show desktop |
| `send_to_desk_1` .. `send_to_desk_9` | `Mod+Shift+1..9` | send to desktop |
| `empty_container` | `Mod+Return` | empty container |
| `empty_tab` | `Mod+Shift+t` | empty tab |
| `next_tab` / `prev_tab` | `Mod+Tab` / `Mod+Shift+Tab` | next / previous tab |
| `wider` / `narrower` | `Mod+=` / `Mod+-` | column width |
| `taller` / `shorter` | `Mod+Shift+=` / `Mod+Shift+-` | tile height |
| `row_wider` / `row_narrower` | `Mod+Ctrl+=` / `Mod+Ctrl+-` | width in row |
| `cycle_width` | `Mod+r` | cycle width |
| `full_width` | `Mod+f` | full width |
| `fullscreen` | `Mod+Shift+f` | fullscreen |
| `center` | `Mod+Space` | center |
| `float` | `Mod+Shift+Space` | float |
| `send_to_scratchpad` / `scratchpad` | `` Mod+Shift+` `` / `` Mod+` `` | to / from the scratchpad |
| `close` | `Mod+q` | close |
| `close_group` | `Mod+Shift+q` | close the column: every tile, tabs and all |
| `launcher` | `Mod+d` | dune-finder: apps |
| `tile_options` | `Mod+/` | dune-finder: tile options |
| `switcher` | none | dune-finder: windows |
| `private_tile` | none | private tile |
| `mute_tile` | none | mute the focused tile's app (again: unmute) |
| `dock` | `Mod+Shift+d` | the tile (with its tabs) into the dock |
| `terminal_vertical` / `terminal_horizontal` | `Mod+v` / `Mod+s` | terminal right / below |
| `terminal_tab` | none | terminal as a tab |
| `stack_to_tabs` | `Mod+t` | stack to tabs |
| `lock` | `Mod+Escape` | lock |
| `dock_show` | `Mod+Shift+e` | the dock of the active monitor, by keys |
| `quit` | none | end the session at once (Log out: dune-finder's System mode) |
| `screenshot` / `_tile` / `_screen` | `Print` / `Ctrl+Print` / `Shift+Print` | screenshot region / tile / monitor |
| `record` / `_tile` / `_screen` | `Mod+Shift+r` / `Mod+Ctrl+r` / `Mod+Ctrl+Shift+r` | record region / tile / monitor |

`Ctrl+Alt+F1..F12` always switch VTs on a real session.
