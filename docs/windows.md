# Windows

How tiles are laid out and moved: the strip, columns, tabs, fullscreen, floats, desktops and the mouse.

- [The strip](#the-strip)
- [Columns, rows and tiles](#columns-rows-and-tiles)
- [Tabs](#tabs)
- [Fullscreen](#fullscreen)
- [Floating tiles and the scratchpad](#floating-tiles-and-the-scratchpad)
- [Desktops and monitors](#desktops-and-monitors)
- [Mouse](#mouse)

## The strip

Every monitor shows a window onto a strip of columns that never ends. Columns open to
the right of the focused one and the view slides to show whatever takes focus, whole.

```text
              ┌───────────────── monitor ─────────────────┐
   ┌────────┐ │┌──────────────┐┌─────────────┐┌──────────┐│┌──────────┐ ┌────────┐
   │        │ ││              ││             ││          │││          │ │        │
   │   A    │ ││      B       ││   C (focus) ││    D     │││    E     │ │   F    │
   │        │ ││              ││             ││          │││          │ │        │
   └────────┘ │└──────────────┘└─────────────┘└──────────┘│└──────────┘ └────────┘
              └───────────────────────────────────────────┘
        ‹1                                                   2›
     ◄── off the view                         off the view ──►
```

- **Focus** `Mod+h/j/k/l` (or the arrows) moves focus; the view follows.
- **Swap** `Mod+Shift+direction` swaps the tile with its neighbour. With nothing that way
  on this monitor it moves to the next monitor.
- **Edge marks** `‹1` and `2›` at the middle of the screen edges count the tiles past them, in white.
- **Edge push**: pushing the pointer against the left or right screen edge slides the
  strip that way, past a little resistance. The view stays there until focus moves, or
  until you click the focused tile that was slid off. When the whole strip fits on the
  monitor, pushing does nothing.

```text
  pointer pushes ►|            the strip slides left
  ┌───────────────┤          ┌───────────────┐
  │ B  │ C  │ D  ░│ E   ──►  │ C  │ D  │ E   │
  └───────────────┤          └───────────────┘
```

- **Width** `Mod+r` cycles the column through ⅓, ½, ⅔ and full width; `Mod+=` / `Mod+-`
  resize it by hand, `Mod+f` toggles full width. When a change gives windows new sizes,
  the layout stays as it was until they have drawn them (at most 200 ms), then every tile
  moves at once, so a slow app never leaves its neighbours overlapping or a gap.
- **Center** `Mod+Space` keeps the focused column in the middle of the monitor, even when
  that leaves empty space at an end, until focus goes to another column.

```text
   without centering             Mod+Space
  ┌─────────────────────┐      ┌─────────────────────┐
  │┌────────┐           │      │     ┌────────┐      │
  ││   A    │           │      │     │   A    │      │
  │└────────┘           │      │     └────────┘      │
  └─────────────────────┘      └─────────────────────┘
```

## Columns, rows and tiles

A column holds rows stacked on top of each other; a row holds tiles side by side.
Rows share the height of the monitor, tiles in a row share the width of the column.

```text
   column 1           column 2
  ┌──────────────┐ ┌──────────────────────┐
  │              │ │  row 1: one tile     │
  │              │ ├───────────┬──────────┤
  │   one tile   │ │  row 2:   │  two     │
  │              │ │  tile     │  tiles   │
  │              │ ├───────────┴──────────┤
  │              │ │  row 3               │
  └──────────────┘ └──────────────────────┘
```

| Keys | Does |
|---|---|
| `Mod+Shift+=` / `Mod+Shift+-` | the tile's height in its column |
| `Mod+Ctrl+=` / `Mod+Ctrl+-` | the tile's width inside its row |
| `Mod+Return` | an empty container, with dune-finder to fill it |
| `Mod+v` / `Mod+s` | a terminal right of / below the tile, in its folder |
| `Mod+q` | close the tile |
| `Mod+Shift+q` | close the column, every tab of it |

When the last window of a tile closes, the tile shrinks towards its middle and fades out
(150 ms) while the tiles left slide into its place.

**Edge splits**: with Mod held, the pointer near the edge of a tile lights a band along that
edge. A Mod+click there opens an empty container on that side, with a compact dune-finder
that shows no list until you type.

```text
  Mod held, pointer near the right edge     Mod+click
  ┌──────────────────┐                       ┌─────────┬────────┐
  │                 ▐│                       │         │ ▸ _    │
  │      tile       ▐│  ──►                  │  tile   │ (empty │
  │                 ▐│                       │         │  slot) │
  └──────────────────┘                       └─────────┴────────┘
```

## Tabs

Tabs belong to columns, and each tab keeps its own rows and splits. The bar sits at the
top left of the column; a tab grows up to `[tabs] max_width` and shrinks to `min_width`,
then the bar scrolls.

```text
  ┌─[ vim ]─[ logs ]─[ htop ]─ + ────────────┐
  │                                          │
  │          the shown tab ("vim")           │
  │          with its own splits             │
  │                                          │
  └──────────────────────────────────────────┘
```

| Keys / mouse | Does |
|---|---|
| `Mod+Tab` / `Mod+Shift+Tab` | next / previous tab |
| `Mod+Ctrl+direction` | join the column that way as a tab, or leave a tabbed one |
| `Mod+t` | the tile out of its split into a tab of its own |
| `Mod+Shift+t` | an empty tab next to the focused one |
| click on a tab | show it |
| click on `+` | a new empty tab, with dune-finder |
| Mod+drag onto a tab bar | the tile joins that container as a tab |
| Mod+drag a tab along its bar | reorder it |
| `Ctrl+T` or `:t` in dune-finder | open the app as a tab |

## Fullscreen

A tile goes fullscreen when its app asks (a video's `f`, a page's fullscreen button, a
game), with its maximize button (`[]`), or with `Mod+Shift+f`. It covers its whole monitor, bar included, with no gaps or
borders; only Overlay layers (dune-finder, notifications set to `layer=overlay` in mako)
and the pointer show over it.

```text
   Mod+Shift+f (or the app's own key)
  ┌─ waybar ────────────────────┐      ┌─────────────────────────────┐
  │┌──────┐┌────────┐┌──────┐   │      │                             │
  ││  A   ││ video  ││  C   │   │ ──►  │           video             │
  │└──────┘└────────┘└──────┘   │      │                             │
  └─────────────────────────────┘      └─────────────────────────────┘
```

It keeps its place in the strip (or as a float): leaving fullscreen puts it back. It
ends with the same key, the app's own key (Esc in a browser), closing it, or focus
going to another tile of that monitor (`Mod+h/l`, a new window, a notification). Going
to another desktop and back keeps it.

## Floating tiles and the scratchpad

`Mod+Shift+Space` (or a Mod+click without moving) floats the tile above the strip; the
same again puts it back. Mod+left-drag moves a float, Mod+right-drag resizes it.

A **sticky** float (`On top, every desktop` in the tile options, the `sticky` key or rule
action, none bound by default) stays in front of every other tile, a fullscreen one too,
and on its monitor whichever desktop it shows, at the same place: a camera, a video, a
call. A tiled tile turned sticky floats small in the bottom right corner (16:9); move and
resize it as any float. The same again makes it an ordinary float.

`dune-camera` (installed by `make install`; mpv on the first video device, mirrored, run
again to close) is meant for it. Its picture fills the float whatever its shape, cropping
what does not fit: resize it to a square or a strip to frame just a face
(`DUNE_CAMERA_CROP=0` letterboxes instead).

```toml
[spawn]
"mod+c" = "dune-camera"

[[rules]]
app_id = "^dune-camera$"
actions = ["sticky"]
```

The **scratchpad** works like sway's: `` Mod+Shift+` `` puts the tile away, off every desktop.
`` Mod+` `` calls the next one back as a float on the active monitor; on a shown, focused one
it puts it away again at the end of the queue, so repeated presses cycle through them.
dune-shell's dock shows its windows too, next to the dock's entries (see
[The dock](dock.md)).

```text
   desktop                          scratchpad queue
  ┌──────────────────────┐         ┌───┬───┬───┐
  │  ┌──────────┐        │ ◄─Mod+`─│ 1 │ 2 │ 3 │
  │  │ float 1  │        │         └───┴───┴───┘
  │  └──────────┘        │ ─Mod+`─►  (back to the end)
  └──────────────────────┘
```

## Desktops and monitors

Every monitor has nine desktops (1-9) and shows one of them.

| Keys | Does |
|---|---|
| `Mod+1` .. `Mod+9` | show that desktop |
| `Mod+Shift+1` .. `Mod+Shift+9` | send the focused tile there |
| `[keys] focus_monitor_*` | focus the monitor that way, or `_next` the next one (unbound by default) |

```text
   HDMI-A-1                      DP-1
  ┌──────────────────┐          ┌──────────────────┐
  │  desktop 2 of 9  │          │  desktop 5 of 9  │
  │  [1] 2 [3] ...   │          │  ... [5] ...     │
  └──────────────────┘          └──────────────────┘
```

Apps can be launched onto a desktop (`firefox :d3`); with `follow = true` dune goes there too.
Clicking a notification (mako, xdg-activation) goes to the window, showing its desktop and
its tab.

## Mouse

| Mouse | Does |
|---|---|
| click | focus the tile (a focused tile slid half off comes back whole) |
| hover | focus, with `focus_follows_mouse = true` (only tiles fully on screen) |
| Mod+left drag | move the tile; drop it on another column, monitor, or a tab bar |
| Mod+left click | float / unfloat; near an edge: an empty split there |
| Mod+right drag | resize the tile (or a float) |
| `[mouse] dock_click` | the tile under the pointer into the dock |
| push a screen edge | slide the strip |

One drag at a time: while a Mod+drag goes on, the other buttons belong to it, and it ends
when its button is let go, the session locks or its tile closes.

```text
   Mod+left drag of B onto D's column
  ┌──────┐┌──────┐┌──────┐          ┌──────┐┌──────┐
  │  A   ││  B ✋ ││  D   │   ──►    │  A   ││  D   │
  │      ││      ││      │          │      │├──────┤
  └──────┘└──────┘└──────┘          └──────┘│  B   │
                                            └──────┘
```
