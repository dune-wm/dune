# The dock

Every desktop of every monitor has a dock: tiles put out of the way, off the strip.
`Mod+Shift+d` (the `dock` key), `[mouse] dock_click` (for example `"mod+right"`) on the
tile under the pointer, **Send to dock** in the tile options, or the window's own minimize
button (`-`) puts a tile there. A tile
of a tabbed column takes its whole column, tabs and all.

dune keeps the dock; dune-shell draws it, unfolding upward from the bottom left corner of
the monitor, with the icon of each entry's app and the scratchpad's windows next to it.
`Mod+Shift+e` (the `dock_show` key) asks dune-shell to show the dock of the active monitor
(the one desktops switch on), or to hide it; `dune dock show [OUTPUT]` does the same for
that monitor (a bar's click: the bar's monitor, from `$WAYBAR_OUTPUT_NAME`). See
dune-shell's README for its mouse and keys.

- **Taking an entry out** (`dune dock take TILE`): it goes back where it was in the strip
  (or where it floated), focused. For a tabbed column, the tab holding `TILE` shows. A
  `TILE` waiting in the scratchpad floats out of it on the desktop shown.
- **Pinning** (`dune dock pin TILE [OUTPUT]`): pin the entry holding `TILE`, or unpin it.
  A pinned entry is in the dock of every desktop, and comes out on the desktop shown: a
  tile floating, a tabbed column right of the focused one. Unpinned, it belongs to the
  dock of the desktop `OUTPUT` shows (the active monitor without one).
- **What dune-shell is told**: `dune events dock` writes each monitor's dock whenever it
  changes, the pinned entries first: each entry's shown tile, app and title, whether it is
  pinned or plays sound (followed through `pactl`; for browsers, matched by the MPRIS
  title), and a tabbed column's tabs; then the scratchpad's windows. Private tiles go by
  their masked title. See [`dune events`](session.md#command-line).
- dune-finder's windows mode lists docked windows (`dock`, `dock 📌`); choosing one takes
  it out of the dock, going to its desktop.
- `dune dock watch` writes a waybar line with the count of the monitor's dock, the titles
  (`📌` before pinned ones, `(+2)` after a column's other tabs) in the tooltip.

Tiles in a column or row never get smaller than a tenth of it: resizing stops there.
