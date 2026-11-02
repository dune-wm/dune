//! Which tiles play sound, so dune-shell's dock can show it on a docked one (`sound` in
//! `dune events dock`). A thread
//! follows `pactl subscribe` and, on every change of the playback streams, lists them
//! (`pactl list sink-inputs`): the processes of the streams playing (not paused, not
//! muted) are sent to the event loop, which marks the tiles they belong to. A browser
//! plays through one process for all its windows: when several tiles share it, only those
//! whose title holds the title of what plays (MPRIS, followed with `playerctl`) count.
//! The same streams let the tile options mute a tile (`toggle_mute_tile`): every stream
//! of its process goes silent, so a browser's windows go silent together. A Steam game
//! through xwayland-satellite has satellite's pid: its streams are the ones whose process
//! runs with the `SteamAppId` of its `steam_app_<id>` app id.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    time::Duration,
};

use smithay::reexports::calloop::{LoopHandle, channel};

use crate::{Dune, layout::TileId};

/// Wait before following `pactl subscribe` again when it ends (the sound server restarted).
const RETRY: Duration = Duration::from_secs(5);

/// A playback stream (`pactl list sink-inputs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    /// Its sink input number, for `pactl set-sink-input-mute`.
    pub index: u32,
    pub pid: Option<u32>,
    /// Paused.
    pub corked: bool,
    pub muted: bool,
}

impl Stream {
    fn plays(&self) -> bool {
        !self.corked && !self.muted
    }
}

enum Change {
    /// The playback streams now.
    Streams(Vec<Stream>),
    /// Titles of what the media players play now.
    Titles(Vec<String>),
}

pub fn watch(loop_handle: &LoopHandle<'static, Dune>) {
    let (tx, rx) = channel::channel();
    let watched = loop_handle.insert_source(rx, |event, _, state| match event {
        channel::Event::Msg(Change::Streams(streams)) => {
            state.sound_pids = streams.iter().filter(|s| s.plays()).filter_map(|s| s.pid).collect();
            state.sound_streams = streams;
            state.update_sounding();
        }
        channel::Event::Msg(Change::Titles(titles)) => state.sound_titles = titles,
        channel::Event::Closed => {}
    });
    if let Err(err) = watched {
        return tracing::warn!("docked tiles will not show sound: {err}");
    }
    let players = tx.clone();
    let spawned = std::thread::Builder::new().name("dune-sound".into()).spawn(move || follow(&tx));
    if let Err(err) = spawned {
        tracing::warn!("docked tiles will not show sound: {err}");
    }
    let spawned = std::thread::Builder::new().name("dune-players".into()).spawn(move || follow_players(&players));
    if let Err(err) = spawned {
        tracing::warn!("browser tiles will not tell which one plays: {err}");
    }
}

/// Send the streams now and after every change of them, until dune exits.
fn follow(tx: &channel::Sender<Change>) {
    loop {
        let child = Command::new("pactl").arg("subscribe").env("LC_ALL", "C").stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(err) => return tracing::warn!("docked tiles will not show sound: no pactl ({err})"),
        };
        if tx.send(Change::Streams(streams())).is_err() {
            let _ = child.kill();
            return;
        }
        let lines = BufReader::new(child.stdout.take().unwrap()).lines();
        for line in lines.map_while(Result::ok) {
            if line.contains("sink-input") && tx.send(Change::Streams(streams())).is_err() {
                let _ = child.kill();
                return;
            }
        }
        let _ = child.wait();
        std::thread::sleep(RETRY);
    }
}

/// `playerctl` lists every player's status and title on one line.
const PLAYER_FORMAT: &str = "{{status}}\t{{xesam:title}}";

/// Send the titles of what plays now and after every change of a player, until dune exits.
fn follow_players(tx: &channel::Sender<Change>) {
    loop {
        let child = Command::new("playerctl")
            .args(["--all-players", "--follow", "metadata", "--format", PLAYER_FORMAT])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(err) => return tracing::warn!("browser tiles will not tell which one plays: no playerctl ({err})"),
        };
        if tx.send(Change::Titles(playing_titles())).is_err() {
            let _ = child.kill();
            return;
        }
        // `--follow` prints only the player that changed: list them all again.
        let lines = BufReader::new(child.stdout.take().unwrap()).lines();
        for _ in lines.map_while(Result::ok) {
            if tx.send(Change::Titles(playing_titles())).is_err() {
                let _ = child.kill();
                return;
            }
        }
        let _ = child.wait();
        std::thread::sleep(RETRY);
    }
}

/// Titles of what the media players play now.
fn playing_titles() -> Vec<String> {
    Command::new("playerctl")
        .args(["--all-players", "metadata", "--format", PLAYER_FORMAT])
        .stderr(Stdio::null())
        .output()
        .map(|out| titles_playing(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

/// The titles of the `Playing` lines of `playerctl` output in `PLAYER_FORMAT`, lowercase.
fn titles_playing(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter_map(|line| line.strip_prefix("Playing\t"))
        .map(|title| title.trim().to_lowercase())
        .filter(|title| !title.is_empty())
        .collect()
}

/// The playback streams now.
fn streams() -> Vec<Stream> {
    Command::new("pactl")
        .args(["list", "sink-inputs"])
        .env("LC_ALL", "C")
        .stderr(Stdio::null())
        .output()
        .map(|out| parse_streams(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

/// The streams of `pactl list sink-inputs` output.
fn parse_streams(listing: &str) -> Vec<Stream> {
    let mut streams = Vec::new();
    for block in listing.split("Sink Input #").skip(1) {
        let mut lines = block.lines();
        let Some(index) = lines.next().and_then(|n| n.trim().parse().ok()) else { continue };
        let mut stream = Stream { index, pid: None, corked: false, muted: false };
        for line in lines.map(str::trim) {
            if line == "Corked: yes" {
                stream.corked = true;
            } else if line == "Mute: yes" {
                stream.muted = true;
            } else if let Some(value) = line.strip_prefix("application.process.id = ") {
                stream.pid = value.trim_matches('"').parse().ok();
            }
        }
        streams.push(stream);
    }
    streams
}

/// Whether to mute `streams` (the ones of a tile): unless they all are already.
fn mute_next(streams: &[&Stream]) -> bool {
    !streams.iter().all(|s| s.muted)
}

/// The Steam game id of a `steam_app_<id>` app id.
fn steam_app(app_id: &str) -> Option<&str> {
    app_id.strip_prefix("steam_app_").filter(|id| !id.is_empty())
}

/// The `SteamAppId` the process runs with (a Steam game's processes inherit it).
fn process_steam_app(pid: u32) -> Option<String> {
    let environ = std::fs::read(format!("/proc/{pid}/environ")).ok()?;
    env_var(&environ, "SteamAppId").map(str::to_string)
}

/// A variable of a `/proc/<pid>/environ` listing.
fn env_var<'a>(environ: &'a [u8], name: &str) -> Option<&'a str> {
    environ.split(|b| *b == 0).find_map(|entry| {
        let value = entry.strip_prefix(name.as_bytes())?.strip_prefix(b"=")?;
        std::str::from_utf8(value).ok()
    })
}

impl Dune {
    /// Mark the tiles whose window's process, or one it started, plays sound. Tiles that
    /// share that process with others (a browser's windows) only might: see `plays_sound`.
    pub fn update_sounding(&mut self) {
        let mut owners: HashMap<u32, Vec<TileId>> = HashMap::new();
        if !self.sound_pids.is_empty() {
            let ids: Vec<TileId> = self.tiles.keys().copied().collect();
            for id in ids {
                let Some(owner) = self.tile_pid(id) else { continue };
                if self.sound_pids.iter().any(|pid| crate::tiling::is_descendant(*pid, owner)) {
                    owners.entry(owner).or_default().push(id);
                }
            }
        }
        self.sounding.clear();
        self.sounding_shared.clear();
        for tiles in owners.into_values() {
            match tiles[..] {
                [one] => {
                    self.sounding.insert(one);
                }
                _ => self.sounding_shared.extend(tiles),
            }
        }
    }

    /// The streams of the tile's window's process, or of one it started; for a Steam
    /// game, the ones of its processes.
    fn tile_streams(&self, id: TileId) -> Vec<&Stream> {
        let owner = self.tile_pid(id);
        let steam_app = self.tile_app_id(id).as_deref().and_then(steam_app).map(str::to_string);
        self.sound_streams
            .iter()
            .filter(|s| {
                s.pid.is_some_and(|pid| {
                    owner.is_some_and(|owner| crate::tiling::is_descendant(pid, owner))
                        || steam_app.as_deref().is_some_and(|app| process_steam_app(pid).as_deref() == Some(app))
                })
            })
            .collect()
    }

    /// Whether the tile is muted: it has streams, all muted.
    pub fn tile_muted(&self, id: TileId) -> bool {
        let streams = self.tile_streams(id);
        !streams.is_empty() && !mute_next(&streams)
    }

    /// Mute the tile's streams, or unmute them when they all are. `pactl` runs off the
    /// event loop; its change comes back through `pactl subscribe`.
    pub fn toggle_mute_tile(&mut self, id: TileId) {
        let streams = self.tile_streams(id);
        if streams.is_empty() {
            return tracing::info!(tile = id, "nothing to mute: the tile plays no stream");
        }
        let mute = mute_next(&streams);
        let indexes: Vec<u32> = streams.iter().map(|s| s.index).collect();
        tracing::info!(tile = id, mute, ?indexes, "mute tile");
        let spawned = std::thread::Builder::new().name("dune-mute".into()).spawn(move || {
            for index in indexes {
                let set = Command::new("pactl")
                    .args(["set-sink-input-mute", &index.to_string(), if mute { "1" } else { "0" }])
                    .stderr(Stdio::null())
                    .status();
                if let Err(err) = set {
                    tracing::warn!("cannot mute stream {index}: {err}");
                }
            }
        });
        if let Err(err) = spawned {
            tracing::warn!("cannot mute the tile: {err}");
        }
    }

    /// The `mute_tile` key: the focused tile.
    pub fn toggle_mute_focused(&mut self) {
        if let Some(id) = self.focused {
            self.toggle_mute_tile(id);
        }
    }

    /// Whether the tile plays sound: its process does, alone, or its window's title holds
    /// the title of what a media player plays (the title changes with the browser's tab).
    pub fn plays_sound(&self, id: TileId) -> bool {
        self.sounding.contains(&id)
            || self.sounding_shared.contains(&id) && {
                let title = self.tile_title(id).to_lowercase();
                self.sound_titles.iter().any(|t| title.contains(t.as_str()))
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn streams_are_parsed() {
        let listing = "Sink Input #10\n\tDriver: PipeWire\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tapplication.name = \"Firefox\"\n\t\tapplication.process.id = \"1234\"\n\n\
                       Sink Input #11\n\tCorked: yes\n\tMute: no\n\tProperties:\n\t\tapplication.process.id = \"55\"\n\n\
                       Sink Input #12\n\tCorked: no\n\tMute: yes\n\tProperties:\n\t\tapplication.process.id = \"66\"\n\n\
                       Sink Input #13\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tapplication.name = \"mpv\"\n";
        let streams = parse_streams(listing);
        let stream = |index, pid, corked, muted| Stream { index, pid, corked, muted };
        assert_eq!(
            streams,
            [stream(10, Some(1234), false, false), stream(11, Some(55), true, false), stream(12, Some(66), false, true), stream(13, None, false, false)]
        );
        let playing: HashSet<u32> = streams.iter().filter(|s| s.plays()).filter_map(|s| s.pid).collect();
        assert_eq!(playing, HashSet::from([1234]));
        assert!(parse_streams("").is_empty());
    }

    #[test]
    fn steam_games_are_told() {
        assert_eq!(steam_app("steam_app_1062090"), Some("1062090"));
        assert_eq!(steam_app("steam_app_"), None);
        assert_eq!(steam_app("steam"), None);
        let environ = b"HOME=/home/u\0SteamAppIdX=1\0SteamAppId=1062090\0SteamGameId=1062090\0";
        assert_eq!(env_var(environ, "SteamAppId"), Some("1062090"));
        assert_eq!(env_var(environ, "PATH"), None);
    }

    #[test]
    fn mute_unless_all_muted() {
        let on = Stream { index: 1, pid: Some(1), corked: false, muted: true };
        let off = Stream { muted: false, ..on.clone() };
        assert!(mute_next(&[&on, &off]));
        assert!(!mute_next(&[&on, &on]));
        assert!(mute_next(&[&off]));
    }

    #[test]
    fn only_titles_that_play_count() {
        let listing = "Playing\tEL LADO OSCURO\nPaused\tOther song\nPlaying\t\nStopped\t\n";
        assert_eq!(titles_playing(listing), vec!["el lado oscuro".to_string()]);
        assert!(titles_playing("").is_empty());
    }
}
