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
| dock | `custom`, `exec: dune dock watch`, `return-type: json`, `on-click: dune dock show`; first in `modules-left` (each bar follows the dock of its own monitor, from `$WAYBAR_OUTPUT_NAME`; the text is the count, class `empty` or `filled`; the tooltip lists the scratchpad's windows too, it being the dock's first cell) |
| scratchpad | optional, the dock has it: `custom`, `exec: dune scratchpad watch`, `return-type: json`, `on-click: dune scratchpad show` |
| recording | `custom`, `exec: dune recording watch`, `return-type: json`, `on-click: dune recording menu`, `on-click-middle: dune recording tile`, `on-click-right: dune recording screen`, `on-scroll-up` / `on-scroll-down: dune recording audio` (a record button while idle, the time recorded while one runs; a click shows the recording panel, or stops the one running, as any of them again does; scrolling turns the microphone on or off, 🎙 while on) |
| night light | waybar's own module, which runs wlsunset |

## Command line

```sh
dune open [--split l|r|u|d] [--from TILE] [--cwd DIR] [--] COMMAND...   # a terminal tile
dune-open COMMAND...                     # in a terminal: the program takes its tile
dune scratchpad show | watch
dune dock show | watch                   # for the monitor of $WAYBAR_OUTPUT_NAME, else the active one
dune recording menu                      # the recording panel, or stop the recording running
dune recording region | tile | screen     # record that, or stop the recording running
dune recording audio                     # turn recording the microphone on or off
dune recording stop | watch
dune events [window,desktop,output]      # a JSON line per change, until dune goes away
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
```

The window changes are `new`, `title`, `move` (another desktop, the dock, the
scratchpad), `focus` and `close`. Windows on locked desktops are left out, and private
tiles go by their masked title. For example, `dune events window | jq -r 'select(.change
== "focus") | .title'` prints the title of every window that takes focus.
