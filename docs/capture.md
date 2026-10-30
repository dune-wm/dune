# Screenshots and recordings

| Keys | Does |
|---|---|
| `Print` | screenshot of a region chosen with the mouse (Esc cancels) |
| `Ctrl+Print` | screenshot of the focused tile |
| `Shift+Print` | screenshot of the active monitor |
| `Mod+Shift+r` | record a region (any record key again stops) |
| `Mod+Ctrl+r` | record the focused tile |
| `Mod+Ctrl+Shift+r` | record the active monitor |

Screenshots are saved as `dune-YYYY-MM-DD_HH-MM-SS.png` in `[capture] screenshot_dir`, copied to the
clipboard and announced by notification. Recordings are `.mp4` in
`[capture] recording_dir`, silent unless the microphone is on: `[capture] record_audio = true`,
`dune recording audio`, or scrolling on waybar's recording module (the default PipeWire/Pulse
source, so pick the mic in your sound settings). Private tiles never show up in either. Waybar's recording module
starts and stops them too (see [Session and bar](session.md#waybar)): a click shows dune-finder's recording
panel (record a region, the focused tile or the screen; the microphone on or off, flipped in place),
or stops the recording running.

dune implements `ext-image-copy-capture` and `wlr-screencopy`, so grim, wf-recorder and
xdg-desktop-portal-wlr (screen sharing in browsers and calls) work. It also implements
ext/wlr data control, so wl-copy and wl-paste work.
