use std::{collections::{HashMap, HashSet}, ffi::OsString, path::PathBuf, sync::Arc};

use smithay::{
    backend::{renderer::gles::GlesRenderer, winit::WinitGraphicsBackend},
    desktop::{PopupManager, Space, Window, WindowSurfaceType, layer_map_for_output},
    input::{Seat, SeatState, pointer::CursorImageStatus},
    reexports::{
        calloop::{EventLoop, Interest, LoopHandle, LoopSignal, Mode, PostAction, channel, generic::Generic},
        wayland_server::{
            Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        dmabuf::DmabufState,
        viewporter::ViewporterState,
        cursor_shape::CursorShapeManagerState,
        shell::wlr_layer::{Layer, WlrLayerShellState},
        output::OutputManagerState,
        selection::{data_device::DataDeviceState, primary_selection::PrimarySelectionState},
        shell::xdg::{XdgShellState, decoration::XdgDecorationState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::layout::{Layout, Rect, TileId};

/// One slot of the layout: its windows, the first one put in it at the bottom. A program
/// started with `dune-open` in its terminal, or an app launched as a tab of a float, is
/// stacked on top until it closes; the last window is the one shown.
#[derive(Debug, Default)]
pub struct Tile {
    pub stack: Vec<Window>,
    /// An empty container (Mod+Enter): it takes its place before any window is in it.
    pub reserved: bool,
    /// Process of the window put in it last.
    pub pid: Option<u32>,
}

impl Tile {
    pub fn top(&self) -> Option<&Window> {
        self.stack.last()
    }
}

pub struct Dune {
    pub start_time: std::time::Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,
    pub loop_handle: LoopHandle<'static, Dune>,

    pub space: Space<Window>,
    pub loop_signal: LoopSignal,

    // Tiling
    /// One layout per output, by output name.
    pub layouts: HashMap<String, Layout>,
    /// The output new tiles go to when they have no target: the last one the pointer
    /// entered or a tile was focused on.
    pub active_output: Option<String>,
    pub tiles: HashMap<TileId, Tile>,
    pub focused: Option<TileId>,
    pub next_tile: TileId,
    /// Geometries from the last `arrange`, used for borders, focus and hit testing.
    pub geometries: Vec<(TileId, Rect)>,
    /// A resize by keys or wheel sliding into place: tiles are drawn on their way there.
    pub resize_animation: Option<crate::tiling::ResizeAnimation>,
    /// The layout held where it was drawn while windows draw their new sizes.
    pub transaction: Option<crate::transaction::Transaction>,
    /// Tiles whose last window closed, shrinking and fading away (`crate::closing`).
    pub closing: Vec<crate::closing::Closing>,
    /// Sizes sent to toplevels that have not drawn them yet, with when they were sent.
    pub configure_waits: HashMap<
        smithay::reexports::wayland_server::backend::ObjectId,
        (Option<smithay::utils::Size<i32, Logical>>, std::time::Instant),
    >,
    /// The part of each tile that is on its output; tiles scrolled away are missing.
    pub shown: Vec<(TileId, Rect)>,
    /// Bars of the tabbed containers on the outputs (shown ones only), from the last `arrange`.
    pub tab_bars: Vec<crate::layout::TabBar>,
    /// Processes playing sound now, and the tiles they belong to (`crate::sound`): alone
    /// in their process, or sharing it (they play if their title says so), with the titles
    /// the media players play.
    pub sound_pids: HashSet<u32>,
    pub sound_streams: Vec<crate::sound::Stream>,
    pub sounding: HashSet<TileId>,
    pub sounding_shared: HashSet<TileId>,
    pub sound_titles: Vec<String>,
    /// By output: how many tiles of its strip are past its left and its right edge.
    pub edge_counts: HashMap<String, (usize, usize)>,
    /// Tiles that want to be fullscreen (one per monitor), until focus goes to another
    /// tile of their monitor or they ask to stop.
    pub fullscreen: HashSet<TileId>,
    /// Floats in front of every other tile, kept on their monitor's shown desktop whichever
    /// it is (`crate::floating`, the `sticky` tile option).
    pub sticky: HashSet<TileId>,
    /// Where sticky floats were, by app id, for their next window (`crate::sticky_places`).
    pub sticky_places: crate::sticky_places::StickyPlaces,
    /// By output: the fullscreen tile covering it now (laid out on its shown desktop).
    pub covers: HashMap<String, TileId>,
    /// The last-mile zoom, while on (`crate::zoom`): one tile (or monitor) magnified.
    pub zoom: Option<crate::zoom::Zoom>,
    /// The zoom just gone out, still shrinking back on screen (render only).
    pub zoom_leaving: Option<crate::zoom::Zoom>,
    /// The right Ctrl+Shift tapped alone, the keyboard's left click (`crate::zoom`).
    pub click_tap: crate::zoom::ClickTap,
    /// What the next synthetic click is by (`key`: the Ctrl+Shift tap), for its event.
    pub click_by: &'static str,
    /// What the pointer is doing: pointing, or a Mod+drag (`crate::seatop`).
    pub seatop: crate::seatop::Seatop,
    /// Floating tiles of each output, by output name, back to front.
    pub floating: HashMap<String, Vec<(TileId, Rect)>>,
    /// Where floating tiles came from in the strip, with the layout's output, to go back.
    pub float_homes: HashMap<TileId, (String, crate::layout::Home)>,
    /// Where tiles sent to a tab of their own by Mod+t were in their column, with the
    /// layout's output, to go back on the next Mod+t.
    pub tab_homes: HashMap<TileId, (String, crate::layout::Home)>,
    /// The desktop each output shows (1 when missing); `layouts` and `floating` hold it.
    pub desks: HashMap<String, u8>,
    /// The desktop each output showed before the one it shows: where locking that one goes.
    pub previous_desks: HashMap<String, u8>,
    /// Desktops not shown, by output name and number.
    pub hidden: HashMap<(String, u8), crate::desktops::Desk>,
    /// Tiles waiting in the scratchpad, in order, with where they floated (`crate::scratchpad`).
    pub scratchpad: Vec<(TileId, Option<Rect>)>,
    /// Scratchpad tiles shown, floating: the `scratchpad` key puts them back.
    pub scratch_shown: HashSet<TileId>,
    /// `dune scratchpad watch` connections, and the last line they were sent.
    pub scratchpad_watchers: Vec<std::os::unix::net::UnixStream>,
    /// `subscribe` connections (`crate::events`), and what they were told last.
    pub subscribers: Vec<crate::events::Subscriber>,
    pub last_snapshot: crate::events::Snapshot,
    /// Events asked for by keys meant for the bar (`minimap`, `dock_show`), sent after this turn.
    pub asked_events: Vec<crate::events::Event>,
    /// When the mouse's motion was last told to `pointer` subscribers (`tell_motion`).
    pub told_motion: Option<std::time::Instant>,
    /// Mod held alone (`crate::mod_hold`).
    pub mod_hold: crate::mod_hold::ModHold,
    pub scratchpad_status: String,
    /// Tiles put away in the docks of the desktops, oldest first (`crate::dock`).
    pub dock: Vec<crate::dock::Docked>,
    /// Tiles and tabbed columns out of the dock that were pinned there: back in the dock,
    /// they are pinned again.
    pub dock_pins: HashSet<crate::dock::Pin>,
    /// `dune dock watch` connections, with the monitor they follow and the last line sent.
    pub dock_watchers: Vec<(std::os::unix::net::UnixStream, Option<String>, String)>,
    /// Tiles whose next windows are those of a process (`dune-open`, a `swallow` request)
    /// and its children, with its pid, while it lives.
    pub swallows: HashMap<TileId, u32>,
    /// Apps the launcher started on their own, by their `DUNE_LAUNCH` number: where their
    /// first window goes, and when they started.
    pub launches: HashMap<u64, crate::launcher::Launch>,
    pub next_launch: u64,
    /// The latest window nothing claimed (a tile of its own), its process and when: it
    /// takes the place of a window of that process closing right after (see
    /// `REPLACE_WINDOW`).
    pub unclaimed_window: Option<(TileId, u32, std::time::Instant)>,
    /// New tiles of their own whose window rules run at their first commit.
    pub pending_rules: crate::rules::PendingRules,
    /// `dune-shell` link given to terminals as `$SHELL`, and the shell it starts.
    pub shell_wrapper: Option<PathBuf>,
    pub user_shell: PathBuf,

    // Smithay State
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Dune>,
    pub data_device_state: DataDeviceState,
    pub primary_selection_state: PrimarySelectionState,
    /// Clipboard managers and `wl-copy`/`wl-paste` (data control, ext and wlr flavours).
    pub ext_data_control_state: smithay::wayland::selection::ext_data_control::DataControlState,
    pub wlr_data_control_state: smithay::wayland::selection::wlr_data_control::DataControlState,
    pub popups: PopupManager,

    pub seat: Seat<Self>,
    pub config: crate::config::Config,

    pub layer_shell_state: WlrLayerShellState,
    pub viewporter_state: ViewporterState,
    pub shortcuts_inhibit_state: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
    pub fractional_scale_state: smithay::wayland::fractional_scale::FractionalScaleManagerState,
    pub cursor_shape_state: CursorShapeManagerState,
    /// A layer (dune-finder, a launcher) that asked for the keyboard; it has it until it closes.
    pub layer_focus: Option<WlSurface>,

    /// GPU buffers from clients (linux-dmabuf); the global is created with the backend.
    pub dmabuf_state: DmabufState,
    /// Screen capture (see `crate::screencopy`): monitors as capture sources, the
    /// sessions clients hold, and frames waiting for something new on their monitor.
    pub output_capture_source_state: smithay::wayland::image_capture_source::OutputCaptureSourceState,
    pub image_copy_capture_state: smithay::wayland::image_copy_capture::ImageCopyCaptureState,
    /// xdg-activation: windows brought forward by a token (a clicked notification).
    pub activation_state: smithay::wayland::xdg_activation::XdgActivationState,
    pub capture_sessions: Vec<smithay::wayland::image_copy_capture::Session>,
    pub pending_captures: Vec<crate::screencopy::PendingCapture>,
    /// The thumbnails of the tiles kept for shells (`crate::thumbnails`).
    pub thumbnails: crate::thumbnails::Thumbnails,
    /// wlr-screencopy (wf-recorder): its frames waiting for something new, see
    /// `crate::wlr_screencopy`.
    pub wlr_screencopy: crate::wlr_screencopy::WlrScreencopy,
    /// Where we draw: a window in another session, or the real hardware.
    pub backend: Backend,
    pub decorations: crate::render::Decorations,
    pub tab_titles: crate::tab_title::TabTitles,
    /// The shown desktop's number in the corner of each monitor.
    pub watermark: crate::watermark::Watermark,
    /// Bars told about the desktops (ext-workspace-v1).
    pub workspaces: crate::workspaces::WorkspaceState,
    /// Night light: gamma ramps clients set per monitor (wlr-gamma-control).
    pub gamma: crate::gamma::GammaState,
    /// dune-finder (Mod+d, Mod+/, Mod+Enter) while it runs.
    pub finder: Option<crate::launcher::FinderRun>,
    /// The pid of a dune-finder that exited, from the thread waiting on it: one that never
    /// showed its layer (`dune find --pick`) is forgotten by this, not by `layer_destroyed`.
    pub finder_exited: channel::Sender<u32>,
    /// The screen recording (`record` key) while it runs, see `crate::screenshot`.
    pub recording: Option<crate::screenshot::Recording>,
    /// When wf-recorder started, while it records (the recording's shell says).
    pub recording_since: Option<std::time::SystemTime>,
    /// What the recording takes, unless a whole monitor: framed on screen while it runs.
    pub recording_area: Option<crate::layout::Rect>,
    /// Recordings take the microphone too (`[capture] record_audio`, `dune recording audio`).
    pub recording_audio: bool,
    /// `dune recording watch` connections (bars), told when a recording starts and ends.
    pub recording_watchers: Vec<std::os::unix::net::UnixStream>,
    /// Tiles focused, the latest first: dune-finder lists windows in this order.
    pub recent: Vec<TileId>,
    /// How far the pointer has pushed against a screen edge (see `input::edge_slide`).
    pub edge_push: f64,
    /// Where a game that locks the pointer wants it when the lock ends (its cursor position
    /// hint, global).
    pub pointer_hint: Option<Point<f64, Logical>>,
    /// The pointer image: clients choose it; dune draws it on a real session.
    pub cursor_status: CursorImageStatus,
    pub cursor: crate::cursor::Cursor,
    /// swayidle's timers (ext-idle-notify-v1), restarted by any input.
    pub idle_notifier_state: smithay::wayland::idle_notify::IdleNotifierState<Dune>,
    /// Surfaces asking to hold off idle (idle-inhibit: videos, games), while they are alive.
    pub idle_inhibitors: Vec<WlSurface>,
    /// The keymap of a virtual keyboard (wtype, dictation) while it is the seat's, until a
    /// real key brings ours back.
    pub virtual_keymap: Option<std::sync::Arc<str>>,
    /// Idle held off from dune-finder's Stay awake, until turned off again.
    pub stay_awake: bool,
    /// Screen lock (swaylock, ext-session-lock-v1), see `crate::lock`.
    pub session_lock: crate::lock::SessionLock,
    /// Tiles and desktops blocked on screen and in captures, see `crate::privacy`.
    pub privacy: crate::privacy::Privacy,
}

pub enum Backend {
    /// Not set up yet.
    None,
    /// Nested in another compositor (or X11).
    Winit(WinitGraphicsBackend<GlesRenderer>),
    /// A real session on a TTY.
    Udev(crate::udev::UdevData),
}

impl Dune {
    /// Run `f` with the renderer lent out of the backend, so it can build a scene from
    /// the rest of the state. None before there is one.
    pub fn with_renderer<T>(&mut self, f: impl FnOnce(&mut Self, &mut GlesRenderer) -> T) -> Option<T> {
        let mut backend = std::mem::replace(&mut self.backend, Backend::None);
        let result = backend.renderer().map(|renderer| f(self, renderer));
        self.backend = backend;
        result
    }

    /// The GPU's render node, for clients allocating buffers on it (real sessions).
    pub fn render_node(&self) -> Option<smithay::backend::drm::DrmNode> {
        match &self.backend {
            Backend::Udev(udev) => udev.render_node(),
            _ => None,
        }
    }
}

impl Backend {
    /// The renderer of the primary GPU.
    pub fn renderer(&mut self) -> Option<&mut GlesRenderer> {
        match self {
            Backend::None => None,
            Backend::Winit(backend) => Some(backend.renderer()),
            Backend::Udev(udev) => udev.renderer.as_mut(),
        }
    }
}

impl Dune {
    pub fn new(event_loop: &mut EventLoop<'static, Self>, display: Display<Self>) -> Self {
        let start_time = std::time::Instant::now();

        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        // Clients scaling their buffers (xwayland-satellite requires it).
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        // Monitor scales like 1.25 told exactly, so clients draw sharp at them.
        let fractional_scale_state = smithay::wayland::fractional_scale::FractionalScaleManagerState::new::<Self>(&dh);
        // Named cursors (GTK4 otherwise draws its own, sized by its own guess).
        let cursor_shape_state = CursorShapeManagerState::new::<Self>(&dh);
        let workspaces = crate::workspaces::WorkspaceState::new(&dh);
        let gamma = crate::gamma::GammaState::new(&dh);
        let popups = PopupManager::default();
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh);
        // `wl-copy`/`wl-paste` and clipboard managers set and read both selections without
        // a focused window.
        let ext_data_control_state = smithay::wayland::selection::ext_data_control::DataControlState::new::<Self, _>(
            &dh,
            Some(&primary_selection_state),
            |_| true,
        );
        let wlr_data_control_state = smithay::wayland::selection::wlr_data_control::DataControlState::new::<Self, _>(
            &dh,
            Some(&primary_selection_state),
            |_| true,
        );
        // Games (through xwayland-satellite too) read raw mouse motion and lock or confine
        // the pointer while playing.
        smithay::wayland::relative_pointer::RelativePointerManagerState::new::<Self>(&dh);
        smithay::wayland::pointer_constraints::PointerConstraintsState::new::<Self>(&dh);
        // Screen capture: grim, wf-recorder (wlr-screencopy), and screen sharing through
        // xdg-desktop-portal-wlr.
        let output_capture_source_state = smithay::wayland::image_capture_source::OutputCaptureSourceState::new::<Self>(&dh);
        let image_copy_capture_state = smithay::wayland::image_copy_capture::ImageCopyCaptureState::new::<Self>(&dh);
        let wlr_screencopy = crate::wlr_screencopy::WlrScreencopy::new(&dh);
        let activation_state = smithay::wayland::xdg_activation::XdgActivationState::new::<Self>(&dh);
        // swayidle: idle timeouts, held off by videos and games; swaylock: the screen lock.
        let idle_notifier_state = smithay::wayland::idle_notify::IdleNotifierState::new(&dh, event_loop.handle());
        smithay::wayland::idle_inhibit::IdleInhibitManagerState::new::<Self>(&dh);
        // Virtual machines, remote desktops and games that want every key, Mod ones too.
        let shortcuts_inhibit_state =
            smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState::new::<Self>(&dh);
        // Typed keys from wtype and dictation (Hex pastes with it); open to every client, as sway.
        smithay::wayland::virtual_keyboard::VirtualKeyboardManagerState::new::<Self, _>(&dh, |_| true);
        let session_lock = crate::lock::SessionLock::new(&dh);

        let mut seat_state = SeatState::new();
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "seat0");
        // Keys arrive as raw keycodes (also when nested), so the layout must be ours.
        let config = crate::config::load().unwrap_or_else(crate::config::defaults);
        let (delay, rate) = (config.repeat_delay, config.repeat_rate);
        if let Err(err) = seat.add_keyboard(config.xkb(), delay, rate) {
            tracing::error!("invalid keyboard layout in the dune config ({err:?}), falling back to the default");
            seat.add_keyboard(Default::default(), delay, rate).unwrap();
        }
        seat.add_pointer();

        let socket_name = Self::init_wayland_listener(display, event_loop);
        let loop_signal = event_loop.get_signal();
        let (finder_exited, exits) = channel::channel();
        event_loop
            .handle()
            .insert_source(exits, |event, _, state: &mut Dune| {
                if let channel::Event::Msg(pid) = event {
                    state.finder_gone(Some(pid));
                }
            })
            .expect("Failed to init the dune-finder exit source.");

        Self {
            start_time,
            display_handle: dh,
            loop_handle: event_loop.handle(),

            space: Space::default(),
            loop_signal,
            socket_name,

            layouts: HashMap::new(),
            active_output: None,
            tiles: HashMap::new(),
            focused: None,
            next_tile: 1,
            geometries: Vec::new(),
            resize_animation: None,
            transaction: None,
            closing: Vec::new(),
            configure_waits: HashMap::new(),
            shown: Vec::new(),
            tab_bars: Vec::new(),
            sound_pids: HashSet::new(),
            sound_streams: Vec::new(),
            sounding: HashSet::new(),
            sounding_shared: HashSet::new(),
            sound_titles: Vec::new(),
            edge_counts: HashMap::new(),
            fullscreen: HashSet::new(),
            sticky: HashSet::new(),
            sticky_places: crate::sticky_places::StickyPlaces::load(),
            covers: HashMap::new(),
            seatop: Default::default(),
            floating: HashMap::new(),
            float_homes: HashMap::new(),
            tab_homes: HashMap::new(),
            desks: HashMap::new(),
            previous_desks: HashMap::new(),
            hidden: HashMap::new(),
            scratchpad: Vec::new(),
            scratch_shown: HashSet::new(),
            swallows: HashMap::new(),
            scratchpad_watchers: Vec::new(),
            subscribers: Vec::new(),
            last_snapshot: Default::default(),
            asked_events: Vec::new(),
            zoom: None,
            zoom_leaving: None,
            click_tap: Default::default(),
            click_by: "",
            told_motion: None,
            mod_hold: Default::default(),
            scratchpad_status: String::new(),
            dock: Vec::new(),
            dock_pins: HashSet::new(),
            dock_watchers: Vec::new(),
            launches: HashMap::new(),
            next_launch: 1,
            unclaimed_window: None,
            pending_rules: Default::default(),
            shell_wrapper: None,
            user_shell: PathBuf::from("/bin/sh"),

            compositor_state,
            xdg_shell_state,
            xdg_decoration_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            primary_selection_state,
            ext_data_control_state,
            wlr_data_control_state,
            popups,
            seat,
            tab_titles: crate::tab_title::TabTitles::new(&config.font_family, config.tab_widths, config.tab_style),
            watermark: crate::watermark::Watermark::new(&config.font_family, config.watermark),
            finder: None,
            finder_exited,
            recording: None,
            recording_since: None,
            recording_area: None,
            recording_audio: config.record_audio,
            recording_watchers: Vec::new(),
            recent: Vec::new(),
            edge_push: 0.0,
            pointer_hint: None,
            workspaces,
            gamma,
            config,
            layer_shell_state,
            viewporter_state,
            shortcuts_inhibit_state,
            fractional_scale_state,
            cursor_shape_state,
            layer_focus: None,
            dmabuf_state: DmabufState::new(),
            output_capture_source_state,
            image_copy_capture_state,
            activation_state,
            capture_sessions: Vec::new(),
            pending_captures: Vec::new(),
            thumbnails: Default::default(),
            wlr_screencopy,
            backend: Backend::None,
            decorations: Default::default(),
            cursor_status: CursorImageStatus::default_named(),
            cursor: crate::cursor::Cursor::load(),
            idle_notifier_state,
            idle_inhibitors: Vec::new(),
            virtual_keymap: None,
            stay_awake: false,
            session_lock,
            privacy: Default::default(),
        }
    }

    fn init_wayland_listener(display: Display<Dune>, event_loop: &mut EventLoop<Self>) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();

        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: we don't drop the display
                    unsafe {
                        display.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    /// The surface under the pointer (while locked, see `lock_surface_under`): overlay/top layers, then tiles, then bottom/background layers.
    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        // Locked: only the lock surfaces take the pointer.
        if self.session_lock.is_locked() {
            return self.lock_surface_under(pos);
        }
        let output = self.space.outputs().find(|o| self.space.output_geometry(o).is_some_and(|g| g.to_f64().contains(pos)))?;
        let output_loc = self.space.output_geometry(output)?.loc;
        let layers = layer_map_for_output(output);
        let layer_surface_under = |layer: Layer, kind: WindowSurfaceType| {
            let local = pos - output_loc.to_f64();
            layers.layer_under(layer, local).and_then(|l| {
                let geo = layers.layer_geometry(l)?;
                l.surface_under(local - geo.loc.to_f64(), kind)
                    .map(|(s, p)| (s, (p + geo.loc + output_loc).to_f64()))
            })
        };
        let window_under = || {
            self.space.element_under(pos).and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
        };
        // A fullscreen tile is under Overlay layers only (see `output_elements`).
        if self.covers.contains_key(&output.name()) {
            return layer_surface_under(Layer::Overlay, WindowSurfaceType::ALL).or_else(window_under);
        }
        // The popups of a bar under the windows are drawn over them (see `output_elements`).
        layer_surface_under(Layer::Overlay, WindowSurfaceType::ALL)
            .or_else(|| layer_surface_under(Layer::Top, WindowSurfaceType::ALL))
            .or_else(|| layer_surface_under(Layer::Bottom, WindowSurfaceType::POPUP))
            .or_else(|| layer_surface_under(Layer::Background, WindowSurfaceType::POPUP))
            .or_else(window_under)
            .or_else(|| layer_surface_under(Layer::Bottom, WindowSurfaceType::ALL))
            .or_else(|| layer_surface_under(Layer::Background, WindowSurfaceType::ALL))
    }
}

/// Data associated with a wayland client that connects to Dune.
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
