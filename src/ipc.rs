//! The compositor's end of the control socket (protocol in `dune_common::protocol`),
//! and the `dune open`, `dune scratchpad`, `dune dock` and `dune recording` commands.

use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    time::Duration,
};

use dune_common::protocol::{Request, Side, ask, send, watch};
use smithay::reexports::calloop::{EventLoop, Interest, Mode, PostAction, generic::Generic};

use crate::Dune;

pub use dune_common::protocol::socket_path;

pub fn listen(event_loop: &mut EventLoop<'static, Dune>, path: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    listener.set_nonblocking(true)?;

    event_loop
        .handle()
        .insert_source(Generic::new(listener, Interest::READ, Mode::Level), |_, listener, state| {
            while let Ok((mut stream, _)) = listener.accept() {
                match read_request(&mut stream) {
                    Some(Request::WatchScratchpad) => state.watch_scratchpad(stream),
                    Some(Request::WatchRecording) => state.watch_recording(stream),
                    Some(Request::WatchDock { output }) => state.watch_dock(stream, output),
                    Some(Request::Subscribe { kinds }) => state.subscribe(stream, &kinds),
                    Some(req) => {
                        if let Some(answer) = state.handle_request(req) {
                            let _ = stream.write_all(answer.as_bytes());
                        }
                    }
                    None => tracing::warn!("ignoring malformed IPC request"),
                }
            }
            Ok(PostAction::Continue)
        })
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}

fn read_request(stream: &mut UnixStream) -> Option<Request> {
    stream.set_nonblocking(false).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(200))).ok()?;
    let mut msg = String::new();
    stream.read_to_string(&mut msg).ok()?;
    Request::decode(&msg)
}

/// `dune scratchpad show`: what the `scratchpad` key does. `dune scratchpad watch`: a line
/// for waybar (`return-type: json`) now and on every change of the scratchpad, until dune
/// goes away.
pub fn scratchpad_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("show") => ask(&Request::ScratchpadShow, Duration::from_secs(1)).map(|_| ()),
        Some("watch") => {
            let stream = watch(&Request::WatchScratchpad)?;
            let mut out = std::io::stdout().lock();
            for line in BufReader::new(stream).lines() {
                let line = line.map_err(|e| e.to_string())?;
                writeln!(out, "{line}").and_then(|_| out.flush()).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Err("usage: dune scratchpad show|watch".into()),
    }
}

/// `dune dock show`: show the dock of the monitor of the bar it is run from (waybar says
/// which in `$WAYBAR_OUTPUT_NAME`; else the active one), or hide it. `dune dock watch`: a
/// line for waybar (`return-type: json`) with the windows in that dock, now and on every
/// change, until dune goes away.
pub fn dock_command(args: &[String]) -> Result<(), String> {
    let output = std::env::var("WAYBAR_OUTPUT_NAME").ok().filter(|o| !o.is_empty());
    match args.first().map(String::as_str) {
        Some("show") => ask(&Request::DockShow { output }, Duration::from_secs(1)).map(|_| ()),
        Some("watch") => {
            let stream = watch(&Request::WatchDock { output })?;
            let mut out = std::io::stdout().lock();
            for line in BufReader::new(stream).lines() {
                let line = line.map_err(|e| e.to_string())?;
                writeln!(out, "{line}").and_then(|_| out.flush()).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Err("usage: dune dock show|watch".into()),
    }
}

/// `dune goto TILE`: show that window and focus it (a minimap's click).
pub fn goto_command(args: &[String]) -> Result<(), String> {
    match args {
        [tile] => {
            let tile = tile.parse().map_err(|_| format!("not a tile: `{tile}`"))?;
            ask(&Request::GoTo { tile }, Duration::from_secs(1)).map(|_| ())
        }
        _ => Err("usage: dune goto TILE".into()),
    }
}

/// `dune recording stop`: stop the recording running. `dune recording region|tile|screen`:
/// record that, or stop the recording running (a bar's button). `dune recording audio`: turn
/// recording the microphone on or off (scrolling on the button). `dune recording menu`: stop the
/// recording running, else show dune-finder's recording panel (a click on the button). `dune recording watch`: a waybar
/// JSON line (`return-type: json`) now and on every change, the time recorded going on each
/// second while one runs, until dune goes away. `started` / `ended` are what the
/// recording's shell tells dune (`crate::screenshot`).
pub fn recording_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("stop") => ask(&Request::RecordingStop, Duration::from_secs(1)).map(|_| ()),
        Some(what @ ("region" | "tile" | "screen")) => {
            ask(&Request::RecordingToggle { what: what.into() }, Duration::from_secs(1)).map(|_| ())
        }
        Some("audio") => ask(&Request::RecordingAudio, Duration::from_secs(1)).map(|_| ()),
        Some("menu") => ask(&Request::RecordingMenu, Duration::from_secs(1)).map(|_| ()),
        Some(state @ ("started" | "ended")) => {
            let area = args.get(1).and_then(|g| dune_common::protocol::parse_geometry(g));
            send(&Request::Recording { started: state == "started", area })
        }
        Some("watch") => {
            let stream = watch(&Request::WatchRecording)?;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines() {
                    if line.ok().and_then(|l| tx.send(l).ok()).is_none() {
                        return;
                    }
                }
            });
            let mut out = std::io::stdout().lock();
            let mut since: Option<u64> = None;
            let mut mic = false;
            let mut last = String::new();
            loop {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(line) => {
                        let mut words = line.split_whitespace();
                        since = (words.next() == Some("on")).then(|| words.next().and_then(|s| s.parse().ok())).flatten();
                        mic = line.ends_with(" mic");
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                }
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
                let line = recording_line(since.map(|s| now.saturating_sub(s)), mic);
                if line != last {
                    writeln!(out, "{line}").and_then(|_| out.flush()).map_err(|e| e.to_string())?;
                    last = line;
                }
            }
        }
        _ => Err("usage: dune recording menu|region|tile|screen|audio|stop|watch".into()),
    }
}

/// The waybar line for a recording going on for `elapsed` seconds, or the record button
/// while none runs, after a camera (Nerd Font); 🎙 after it when recordings take the microphone (`mic`).
fn recording_line(elapsed: Option<u64>, mic: bool) -> String {
    let (icon, state) = if mic { (" 🎙", "on") } else { ("", "off") };
    match elapsed {
        Some(s) => format!(
            r#"{{"text":" ● {:02}:{:02}{icon}","tooltip":"Recording · click to stop","class":"recording"}}"#,
            s / 60,
            s % 60
        ),
        None => format!(
            r#"{{"text":" ⏺{icon}","tooltip":"Record · click for the options, middle click the tile, right click the monitor\nMicrophone {state} · scroll to turn it on or off","class":"idle"}}"#
        ),
    }
}

/// `dune open [--split l|r|u|d] [--from TILE] [--cwd DIR] [--] COMMAND...`
/// (`v` and `h` are accepted for `r` and `d`).
pub fn open_command(args: &[String]) -> Result<(), String> {
    let mut side = None;
    let mut from = std::env::var("DUNE_TILE").ok().and_then(|t| t.parse().ok());
    let mut cwd = std::env::current_dir().ok();
    let mut args = args.iter();
    let mut command = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--split" => {
                let v = args.next().ok_or("--split needs a value")?;
                side = Some(Side::parse(v).ok_or("--split must be one of l, r, u, d (or v, h)")?);
            }
            "--from" => from = args.next().and_then(|t| t.parse().ok()),
            "--cwd" => cwd = args.next().map(PathBuf::from),
            "--" => command.extend(args.by_ref().cloned()),
            _ => command.push(arg.clone()),
        }
    }
    send(&Request::Open {
        side,
        from,
        cwd,
        command: command.join(" "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recording_line_counts_the_time() {
        assert!(recording_line(None, false).contains(r#""text":" ⏺","#));
        assert!(recording_line(None, true).contains(r#""text":" ⏺ 🎙","#));
        assert_eq!(
            recording_line(Some(83), false),
            r#"{"text":" ● 01:23","tooltip":"Recording · click to stop","class":"recording"}"#
        );
        assert!(recording_line(Some(83), true).contains(r#""text":" ● 01:23 🎙","#));
    }
}
