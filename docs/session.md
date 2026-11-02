# Session and bar

Lock, idle, night light, X11, Waybar and the `dune` command line.

- [Session: lock, idle, night light, X11](#session-lock-idle-night-light-x11)
- [Waybar](#waybar)
- [Command line](#command-line)

## Session: lock, idle, night light, X11

- **Lock**: `Mod+Escape` runs `lock_command` (`swaylock -f`), through `ext-session-lock`.
  While locked only the lock surface is shown and compositor keys are off.
- **Every key to one window**: a virtual machine, a remote desktop or a game can ask
  for every key, dune's own too (keyboard-shortcuts-inhibit). It gets them while it has
  the keyboard. `Mod+Shift+Escape` (`shortcuts_inhibit`) takes dune's keys back, and
  pressing it again gives them to the window again. That key, `lock` and the VT keys
  always work.
- **Accessibility**: with [dune-a11y](../../dune-a11y) installed, `dune-session` runs
  `dune-a11y enable` before dune, so every app shows its accessibility tree (for
  dune-voice's `click save` and dune-eye-tracker landing on buttons). An app started
  before it was on shows nothing until it is restarted.
- **Adaptive sync** (VRR): `adaptive_sync` in `[output.NAME]`. `"fullscreen"` turns it on
  only while a tile is fullscreen on that monitor (games), so the desktop never flickers.
  A monitor that would need a modeset to switch it is left as it is (logged).
- **Idle**: `ext-idle-notify` for swayidle's timers; idle-inhibit lets videos and games hold it off, and so does the finder's Stay awake.
- **Night light**: `wlr-gamma-control` for wlsunset (real session only).
- **X11 apps**: through `xwayland-satellite`, started when the first X11 client connects
  and restarted if it crashes (`xwayland = "off"` disables it).
- **Games**: pointer constraints (locked and confined pointers) and relative motion.
- **Layers**: wlr-layer-shell for bars, notifications, launchers and wallpapers.

## Waybar

| Module | Setup |
|---|---|
| desktops | `ext/workspaces`, with `"on-click-right": "deactivate"` to lock / unlock |
| dock | `custom`, `exec: dune dock watch`, `return-type: json`, `on-click: dune dock show`; first in `modules-left` (each bar follows the dock of its own monitor, from `$WAYBAR_OUTPUT_NAME`; the text is the count, class `empty` or `filled`; the tooltip lists the scratchpad's windows too, the dock showing them; a click asks dune-shell to show the dock) |
| scratchpad | optional, the dock has it: `custom`, `exec: dune scratchpad watch`, `return-type: json`, `on-click: dune scratchpad show` |
| recording | `custom`, `exec: dune recording watch`, `return-type: json`, `on-click: dune recording menu`, `on-click-middle: dune recording tile`, `on-click-right: dune recording screen`, `on-scroll-up` / `on-scroll-down: dune recording audio` (a record button while idle, the time recorded while one runs; a click shows the recording panel, or stops the one running, as any of them again does; scrolling turns the microphone on or off, 🎙 while on) |
| night light | waybar's own module, which runs wlsunset |

## Command line

```sh
dune open [--split l|r|u|d] [--from TILE] [--cwd DIR] [--] COMMAND...   # a terminal tile
dune-open COMMAND...                     # in a terminal: the program takes its tile
dune scratchpad show | watch
dune dock show [OUTPUT] | watch          # for OUTPUT, else the monitor of $WAYBAR_OUTPUT_NAME, else the active one
dune dock take TILE                      # the dock entry holding TILE out, that tab shown (or out of the scratchpad)
dune dock pin TILE [OUTPUT]              # pin that entry, or unpin it into OUTPUT's dock (else the active monitor's)
dune recording menu                      # the recording panel, or stop the recording running
dune recording region | tile | screen     # record that, or stop the recording running
dune recording audio                     # turn recording the microphone on or off
dune recording stop | watch
dune events [window,desktop,output,strip,minimap,dock,mod,pointer]  # a JSON line per change, until dune goes away
dune goto TILE                           # show that window and focus it
dune action NAME                         # what the key of that name in [keys] does
dune pointer move X Y [OUTPUT]           # the pointer there (from OUTPUT's corner, else the layout's)
dune pointer by DX DY                    # the pointer moved by that much (on against a side edge: the strip slides, as a mouse pushing it)
dune pointer scroll DY                   # the window under the pointer scrolled DY px (down when positive), as a touchpad
dune pointer click|press|release [left|right|middle]  # a button, as a mouse would (left by default)
dune pointer where                       # where the pointer is: x y output (from its corner, as move takes it)
dune find [--pick] [apps|windows|files] TEXT  # dune-finder with TEXT typed in (--pick: its first row)
```

The control socket is `$DUNE_SOCKET`; the protocol lives in `dune-common/src/protocol.rs`.

`dune events` is for scripts and bars that follow dune, like sway's `subscribe`. It
starts with the current state, written as changes, then writes one JSON line for each
change:

```text
{"event":"window","change":"new","tile":3,"app_id":"foot","title":"~","place":"desk 1"}
{"event":"window","change":"focus","tile":3,"app_id":"foot","title":"~","place":"desk 1"}
{"event":"desktop","change":"focus","output":"DP-1","desk":2}
{"event":"output","change":"added","output":"HDMI-A-1"}
{"event":"strip","change":"layout","output":"HDMI-A-1","view":0,"width":2560,"height":1080,"tiles":[{"tile":3,"x":8,"y":8,"w":1268,"h":1064}]}
{"event":"minimap","change":"toggle","output":"HDMI-A-1"}
{"event":"dock","change":"entries","output":"HDMI-A-1","entries":[{"tile":3,"app_id":"foot","title":"vim","pinned":false,"tabs":[{"tile":3,"app_id":"foot","title":"vim","sound":false},{"tile":5,"app_id":"foot","title":"htop","sound":false}],"sound":false}],"scratchpad":[{"tile":7,"app_id":"foot","title":"~","sound":false}]}
{"event":"dock","change":"toggle","output":"HDMI-A-1"}
{"event":"mod","change":"start","keys":"shift"}
```

`minimap` is not a change but a key (`minimap` in `[keys]`, `Mod+a`): dune-shell shows
or hides the minimap of that monitor (the active one, under the pointer).

A `dock` `entries` line tells the dock a monitor shows whenever it changes (see
[The dock](dock.md)): its entries, pinned ones first, each by its shown tile (`tile`, the
shown tab of a column) with `tabs`, the first tile of each tab of a tabbed column (empty
for a tile), and `sound` while one of its windows plays; then `scratchpad`, the windows
waiting there, the same on every monitor. Titles are plain, without the `📌` or `(+2)`
that `dune dock watch` adds. With `dune dock take TILE` and `dune dock pin TILE` it is what
a dock needs. A `dock` `toggle` line is the `dock_show` key (`Mod+Shift+e`, the active
monitor) or `dune dock show`: dune-shell shows or hides that monitor's dock.

A `mod` line is Mod (`modifier`) or Shift held alone, `keys` saying which (`mod`,
`shift`): `start` once it has been down 300 ms with no other key or button, then `end`
when it is let go, or `cancel` when another key or a button goes down first (it was a
shortcut, or a capital letter). A tap or a shortcut tells nothing; Mod+Shift is neither.
Mod only for a `modifier` of one key (`super`, `alt`); neither while the session is locked
or a window holds the keys. dune-voice listens for a phrase while Mod is held, and
dictates with Hex into the focused app while Shift is (let go: it is pasted).

The window changes are `new`, `title`, `move` (another desktop, the dock, the
scratchpad), `focus` and `close`. Windows on locked desktops are left out, and private
tiles go by their masked title. For example, `dune events window | jq -r 'select(.change
== "focus") | .title'` prints the title of every window that takes focus.

A `pointer` `click` line is a mouse button (left, right, middle) going down, `x` `y` from
the monitor's corner. On a tile (a float too) it also says which (`tile`, `app_id`), where
in it (`tile_x`, `tile_y`, from its corner) and its size (`tile_w`, `tile_h`): a button
keeps its place in its window while the strip slides, and dune-eye-tracker learns from
that. Tiles of locked desktops say none. A `move` line is the mouse moving the pointer
(not `dune pointer`), a few times a second at most.

A `strip` line tells the strip of tiles a monitor shows whenever it changes (a tile
opens, closes or resizes, the view slides): each tile's rectangle along the strip and
the part on screen, from `view` to `view + width`. With `dune goto TILE` it is what a
minimap needs.

dune also keeps a thumbnail of every tile for it: `$DUNE_THUMBNAILS/TILE.png` (in
`$XDG_RUNTIME_DIR`), at most 360 px on its longer side, redrawn every
`[capture] thumbnail_interval` seconds (default 2, 0 turns them off) for the windows
that changed. Like the events, private windows and those of locked desktops get none,
and a tile's file is removed when the tile goes. Files are renamed into place, so a
reader never sees half of one.
