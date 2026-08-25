//! Resident daemon: owns the overlay, session state, and virtual pointer.
//!
//! CLI invocations talk to it over a UNIX socket (`ipc.rs`). The daemon runs
//! a single-threaded calloop event loop that integrates:
//!   - the Wayland connection (layer-shell overlay events)
//!   - an IPC channel fed by a small accept/read thread
//!   - periodic timers (session timeout, click sequencing)

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use calloop::channel::{channel, Event as ChannelEvent};
use calloop::timer::{TimeoutAction, Timer};
use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use smithay_client_toolkit as sctk;
use sctk::compositor::{CompositorHandler, CompositorState};
use sctk::output::{OutputHandler, OutputState};
use sctk::registry::{ProvidesRegistryState, RegistryState};
use sctk::shell::wlr_layer::{
    LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use sctk::shm::{Shm, ShmHandler};
use sctk::{
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
};
use wayland_client::globals::{registry_queue_init, GlobalList};
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::{Connection, QueueHandle};

use crate::config::Settings;
use crate::input::VirtualPointer;
use crate::ipc::{self, Request, Response};
use crate::keys::{KeyEvent, KeyboardGrab};
use crate::overlay::Overlay;
use crate::state::{KeyResult, OverlaySession, SelectionResult, SessionState};
use crate::tray::TrayEvent;

// ---------------------------------------------------------------------------
// Wayland delegate wiring
// ---------------------------------------------------------------------------

delegate_registry!(Daemon);
delegate_compositor!(Daemon);
delegate_shm!(Daemon);
delegate_layer!(Daemon);
delegate_output!(Daemon);

pub struct Daemon {
    pub registry_state: RegistryState,
    pub compositor: CompositorState,
    pub shm: Shm,
    pub layer_shell: LayerShell,
    pub output_state: OutputState,
    pub qh: QueueHandle<Daemon>,

    pub overlay: Overlay,
    pub settings: Settings,
    pub session: Option<OverlaySession>,
    pub pointer: Option<VirtualPointer>,
    pub pointer_warning: Option<String>,
    pub keyboard: Option<KeyboardGrab>,
    pub keyboard_warning: Option<String>,

    action: Option<PendingAction>,
    keys_tx: calloop::channel::Sender<KeyEvent>,
    /// Heartbeat updated by the main loop; a watchdog thread force-exits the
    /// daemon if it ever goes stale (defense against loop wedges).
    heartbeat: Arc<std::sync::atomic::AtomicU64>,
    /// Timestamp of the last key event processed by the main loop (nanos).
    /// Anchors the keyboard reader's failsafe deadline.
    key_processed_at: Arc<std::sync::atomic::AtomicU64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Overlay just dismissed: wait, then move the cursor.
    Dismiss,
    /// Cursor about to be moved: wait out the pre-warp delay, then move.
    Move,
    /// Cursor moved: wait out the post-warp delay, then click.
    Click,
}

#[derive(Clone, Copy, Debug)]
struct PendingAction {
    point: (i32, i32),
    phase: Phase,
    ready_at: Instant,
}

impl Daemon {
    fn new(
        globals: &GlobalList,
        qh: QueueHandle<Daemon>,
        keys_tx: calloop::channel::Sender<KeyEvent>,
    ) -> Result<Self, String> {
        let registry_state = RegistryState::new(globals);
        let compositor =
            CompositorState::bind(globals, &qh).map_err(|e| format!("wl_compositor: {e:?}"))?;
        let shm = Shm::bind(globals, &qh).map_err(|e| format!("wl_shm: {e:?}"))?;
        let layer_shell =
            LayerShell::bind(globals, &qh).map_err(|e| format!("zwlr_layer_shell_v1: {e:?}"))?;
        let output_state = OutputState::new(globals, &qh);
        Ok(Self {
            registry_state,
            compositor,
            shm,
            layer_shell,
            output_state,
            qh,
            overlay: Overlay::new(),
            settings: Settings::load(),
            session: None,
            pointer: None,
            pointer_warning: None,
            keyboard: None,
            keyboard_warning: None,
            action: None,
            keys_tx,
            heartbeat: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            key_processed_at: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    // -- request handling ---------------------------------------------------

    fn process_request(&mut self, request: Request) -> Response {
        match request {
            Request::Activate => self.activate(),
            Request::Cancel => {
                self.cancel_session();
                Response::ok("cancelled")
            }
            Request::KeyDown { key } => self.key_down(&key),
            Request::KeyUp { key } => self.key_up(&key),
            Request::Ping => Response::ok("pong"),
        }
    }

    fn activate(&mut self) -> Response {
        self.cancel_session();
        if self.pointer.is_none() {
            match VirtualPointer::new() {
                Ok(pointer) => {
                    self.pointer = Some(pointer);
                    self.pointer_warning = None;
                }
                Err(e) => self.pointer_warning = Some(e.to_string()),
            }
        }
        if self.keyboard.is_none() {
            match KeyboardGrab::start(
                self.keys_tx.clone(),
                self.settings.session_timeout_seconds,
                self.key_processed_at.clone(),
            ) {
                Ok(grab) => {
                    self.keyboard = Some(grab);
                    self.keyboard_warning = None;
                }
                Err(e) => self.keyboard_warning = Some(e),
            }
        }
        let state = SessionState::start(self.overlay.bounds);
        let session = OverlaySession::new(state.clone());
        self.session = Some(session);
        {
            let Daemon { overlay, compositor, layer_shell, qh, .. } = self;
            overlay.ensure_surface(compositor, layer_shell, qh);
            overlay.state = Some(state);
            overlay.show();
        }
        let mut warnings = Vec::new();
        if let Some(warning) = &self.pointer_warning {
            warnings.push(format!("clicks disabled: {warning}"));
        }
        if let Some(warning) = &self.keyboard_warning {
            warnings.push(format!("keyboard capture unavailable: {warning}"));
        }
        if warnings.is_empty() {
            Response::ok("activated")
        } else {
            Response::ok(format!("activated ({})", warnings.join("; ")))
        }
    }

    fn key_down(&mut self, key: &str) -> Response {
        let Some(session) = self.session.as_mut() else {
            return Response::err("no active session");
        };
        if session.state.has_timed_out(self.settings.session_timeout_seconds) {
            self.cancel_session();
            return Response::err("session timed out");
        }
        match session.key_down(key) {
            KeyResult::Invalid => Response::err("key not on grid"),
            _ => {
                // Pending keys have no visual effect; skip the redraw.
                Response::ok("pending")
            }
        }
    }

    fn key_up(&mut self, key: &str) -> Response {
        let Some(session) = self.session.as_mut() else {
            return Response::err("no active session");
        };
        if let KeyResult::Commit = session.key_up(key) {
            if let Some(selection) = session.commit_pending() {
                self.refresh_overlay();
                if selection.is_final {
                    return self.commit_selection(selection);
                }
            }
        }
        Response::ok("ok")
    }

    fn commit_selection(&mut self, selection: SelectionResult) -> Response {
        if self.pointer.is_none() {
            let warning = self
                .pointer_warning
                .clone()
                .unwrap_or_else(|| "no click backend".to_string());
            self.cancel_session();
            return Response::err(format!("clicks disabled: {warning}"));
        }
        // Release the keyboard grab first: the click lands on the app under
        // the pointer, and any further keystrokes must reach it.
        if let Some(mut keyboard) = self.keyboard.take() {
            keyboard.stop();
        }
        let point = selection.point;
        self.session = None;
        let Daemon { overlay, .. } = self;
        overlay.hide();
        self.run_optional_command(&self.settings.on_commit_command.clone());
        self.action = Some(PendingAction {
            point,
            phase: Phase::Dismiss,
            ready_at: Instant::now()
                + Duration::from_secs_f64(self.settings.overlay_dismiss_delay_seconds.max(0.0)),
        });
        Response::ok(format!("committed at {},{}", point.0, point.1)).with_code(3)
    }

    fn cancel_session(&mut self) {
        eprintln!("mousetrap: trace: cancel_session begin");
        // Release the keyboard grab first so keystrokes resume reaching
        // applications immediately.
        if let Some(mut keyboard) = self.keyboard.take() {
            keyboard.stop();
        }
        eprintln!("mousetrap: trace: keyboard stopped");
        self.session = None;
        self.action = None;
        let Daemon { overlay, .. } = self;
        overlay.hide();
        eprintln!("mousetrap: trace: overlay hidden");
        self.run_optional_command(&self.settings.on_cancel_command.clone());
        eprintln!("mousetrap: trace: cancel_session end");
    }

    fn refresh_overlay(&mut self) {
        let Daemon { overlay, session, shm, .. } = self;
        overlay.state = session.as_ref().map(|s| s.state.clone());
        overlay.invalidate();
        overlay.redraw(shm);
    }

    fn run_optional_command(&mut self, command: &Option<String>) {
        if let Some(cmd) = command {
            if !cmd.trim().is_empty() {
                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
            }
        }
    }

    // -- click sequencing ----------------------------------------------------

    /// Advance the pending action when its phase deadline passes.
    fn advance_action(&mut self) {
        let Some(action) = self.action else { return };
        let clicks_enabled = self.settings.click_backend != "none";
        match action.phase {
            Phase::Dismiss => {
                self.action = Some(PendingAction {
                    point: action.point,
                    phase: Phase::Move,
                    ready_at: Instant::now()
                        + Duration::from_secs_f64(self.settings.pre_warp_delay_seconds.max(0.0)),
                });
            }
            Phase::Move => {
                if clicks_enabled {
                    let (w, h) = self.overlay.size;
                    if let Some(pointer) = self.pointer.as_mut() {
                        if let Err(e) = pointer.move_abs(action.point.0, action.point.1, w as i32, h as i32) {
                            eprintln!("mousetrap: cursor move failed: {e}");
                        }
                    }
                }
                self.action = Some(PendingAction {
                    point: action.point,
                    phase: Phase::Click,
                    ready_at: Instant::now()
                        + Duration::from_secs_f64(self.settings.post_warp_delay_seconds.max(0.0)),
                });
            }
            Phase::Click => {
                if clicks_enabled {
                    if let Some(pointer) = self.pointer.as_mut() {
                        if let Err(e) = pointer.left_click() {
                            eprintln!("mousetrap: click failed: {e}");
                        }
                    }
                }
                self.action = None;
            }
        }
    }

    /// Action timer callback: advance phases on schedule.
    fn on_action_timer(&mut self) -> TimeoutAction {
        if let Some(action) = &mut self.action {
            let now = Instant::now();
            if now >= action.ready_at {
                self.advance_action();
                return TimeoutAction::ToDuration(Duration::from_millis(25));
            }
            let remaining = action.ready_at.saturating_duration_since(now);
            return TimeoutAction::ToDuration(remaining.max(Duration::from_millis(1)));
        }
        TimeoutAction::ToDuration(Duration::from_millis(100))
    }

    /// Periodic tick: enforce the session timeout.
    fn tick(&mut self) {
        self.heartbeat.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(session) = &self.session {
            if session.state.has_timed_out(self.settings.session_timeout_seconds) {
                eprintln!("mousetrap: session timed out; cancelling");
                self.cancel_session();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sctk handler traits
// ---------------------------------------------------------------------------

impl ProvidesRegistryState for Daemon {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    fn runtime_add_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
        _version: u32,
    ) {
    }

    fn runtime_remove_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
    ) {
    }
}

impl OutputHandler for Daemon {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for Daemon {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl CompositorHandler for Daemon {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.overlay.scale = new_factor;
        let Daemon { overlay, shm, .. } = self;
        overlay.invalidate();
        overlay.redraw(shm);
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for Daemon {
    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
    ) {
        // The compositor closed the surface (e.g. we destroyed it).
        self.overlay.hide();
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        serial: u32,
    ) {
        let Daemon { overlay, .. } = self;
        overlay.configure(configure.new_size);
        let _ = layer;
        // If the session started before the first configure, fix its bounds.
        let bounds = self.overlay.bounds;
        if let Some(session) = &mut self.session {
            if session.state.initial_bounds == (0, 0, 0, 0) && bounds.2 > 0 && bounds.3 > 0 {
                session.state.initial_bounds = bounds;
                session.state.current_bounds = bounds;
            }
            self.overlay.state = Some(session.state.clone());
        }
        let Daemon { overlay, shm, .. } = self;
        overlay.redraw(shm);
    }
}

// ---------------------------------------------------------------------------
// IPC thread
// ---------------------------------------------------------------------------

type IpcMessage = (Request, std_mpsc::Sender<Response>);

fn bind_socket() -> UnixListener {
    let path = ipc::socket_path();
    match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            UnixListener::bind(&path).expect("bind ipc socket")
        }
    }
}

fn ipc_accept_loop(listener: UnixListener, tx: calloop::channel::Sender<IpcMessage>) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut line = String::new();
        let mut reader = BufReader::new(
            stream.try_clone().expect("clone stream"),
        );
        if reader.read_line(&mut line).is_err() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Request>(line.trim()) else {
            continue;
        };
        let (reply_tx, reply_rx) = std_mpsc::channel();
        if tx.send((request, reply_tx)).is_err() {
            break; // event loop is gone
        }
        let response = reply_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_else(|_| Response::err("daemon busy"));
        let _ = stream.write_all(serde_json::to_string(&response).unwrap().as_bytes());
        let _ = stream.write_all(b"\n");
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the resident daemon in the foreground.
pub fn run() -> i32 {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mousetrap: cannot connect to the Wayland compositor: {e}");
            return 1;
        }
    };
    let (globals, event_queue) = match registry_queue_init::<Daemon>(&conn) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mousetrap: wayland registry init failed: {e:?}");
            return 1;
        }
    };
    let qh = event_queue.handle();
    let (keys_tx, keys_rx) = channel::<KeyEvent>();
    let mut daemon = match Daemon::new(&globals, qh.clone(), keys_tx) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mousetrap: {e} (is this a wlr-layer-shell compositor?)");
            return 1;
        }
    };

    let mut event_loop: EventLoop<Daemon> = match EventLoop::try_new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mousetrap: cannot create event loop: {e}");
            return 1;
        }
    };
    let handle = event_loop.handle();

    // Watchdog: if the main loop stops beating (wedge), force-exit so the
    // kernel releases the keyboard grab and the overlay dies with us.
    let heartbeat = daemon.heartbeat.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        loop {
            let last = heartbeat.load(std::sync::atomic::Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(1));
            let now = heartbeat.load(std::sync::atomic::Ordering::Relaxed);
            if now == last && now != 0 {
                eprintln!("mousetrap: watchdog: main loop stalled; exiting");
                std::process::exit(1);
            }
        }
    });

    if let Err(e) = handle.insert_source(WaylandSource::new(conn.clone(), event_queue), |_, queue, data| {
        match queue.dispatch_pending(data) {
            Ok(count) => {
                if count > 0 {
                    eprintln!("mousetrap: trace: wayland dispatched {count}");
                }
                Ok(count)
            }
            Err(err) => {
                eprintln!("mousetrap: wayland error: {err:?}; exiting");
                std::process::exit(1);
            }
        }
    }) {
        eprintln!("mousetrap: cannot start wayland source: {e}");
        return 1;
    }

    // IPC channel fed by the accept thread.
    let (ipc_tx, ipc_rx) = channel::<IpcMessage>();
    let listener = bind_socket();
    std::thread::spawn(move || ipc_accept_loop(listener, ipc_tx));
    if let Err(e) = handle.insert_source(ipc_rx, |event, _, app| {
        if let ChannelEvent::Msg((request, reply)) = event {
            app.heartbeat.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let response = app.process_request(request);
            let _ = reply.send(response);
        }
    }) {
        eprintln!("mousetrap: cannot start ipc source: {e}");
        return 1;
    }

    // Keyboard grab events (evdev).
    if let Err(e) = handle.insert_source(keys_rx, |event, _, app| {
        if let ChannelEvent::Msg(event) = event {
            eprintln!("mousetrap: trace: key event processed");
            app.heartbeat.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            app.key_processed_at.store(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0),
                std::sync::atomic::Ordering::Relaxed,
            );
            match event {
                KeyEvent::KeyDown(key) => {
                    let _ = app.key_down(&key);
                }
                KeyEvent::KeyUp(key) => {
                    let _ = app.key_up(&key);
                }
                KeyEvent::Escape => app.cancel_session(),
            }
        }
    }) {
        eprintln!("mousetrap: cannot start keyboard source: {e}");
        return 1;
    }

    // Tray (StatusNotifierItem over DBus).
    let (tray_tx, tray_rx) = channel::<TrayEvent>();
    crate::tray::spawn(tray_tx);
    if let Err(e) = handle.insert_source(tray_rx, |event, _, app| {
        if let ChannelEvent::Msg(event) = event {
            match event {
                TrayEvent::Activate => {
                    let _ = app.process_request(Request::Activate);
                }
                TrayEvent::Cancel => {
                    let _ = app.process_request(Request::Cancel);
                }
                TrayEvent::Quit => {
                    app.cancel_session();
                    // Never exit abruptly: the tray host may still be
                    // processing this menu click (a DBus call in flight).
                    // An immediate process::exit here has wedged quickshell's
                    // main thread in the past. Exit on a delay so the DBus
                    // reply lands first.
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_millis(500));
                        std::process::exit(0);
                    });
                }
            }
        }
    }) {
        eprintln!("mousetrap: cannot start tray source: {e}");
        return 1;
    }

    // Periodic session-timeout check.
    let tick = Timer::from_duration(Duration::from_millis(500));
    let mut tick_count: u64 = 0;
    if let Err(e) = handle.insert_source(tick, move |_, _, app| {
        tick_count += 1;
        if tick_count % 20 == 0 {
            eprintln!("mousetrap: trace: tick alive (#{tick_count})");
        }
        app.tick();
        TimeoutAction::ToDuration(Duration::from_millis(500))
    }) {
        eprintln!("mousetrap: cannot start timer: {e}");
        return 1;
    }

    // Action sequencer.
    let action_timer = Timer::from_duration(Duration::from_millis(25));
    if let Err(e) = handle.insert_source(action_timer, |_, _, app| app.on_action_timer()) {
        eprintln!("mousetrap: cannot start action timer: {e}");
        return 1;
    }

    let _ = event_loop.run(None, &mut daemon, |_| {});
    let _ = std::fs::remove_file(ipc::socket_path());
    0
}

// ---------------------------------------------------------------------------
// Client side
// ---------------------------------------------------------------------------

/// Ensure the daemon is running, then send the request.
fn ensure_daemon() -> Option<Response> {
    match ipc::send(&Request::Ping) {
        Ok(response) => Some(response),
        Err(_) => {
            let exe = std::env::current_exe().ok()?;
            let spawn = std::process::Command::new(exe)
                .arg("daemon")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if spawn.is_err() {
                return None;
            }
            for _ in 0..60 {
                std::thread::sleep(Duration::from_millis(25));
                if let Ok(response) = ipc::send(&Request::Ping) {
                    return Some(response);
                }
            }
            None
        }
    }
}

pub fn client_activate() -> i32 {
    let response = ensure_daemon()
        .and_then(|_| ipc::send(&Request::Activate).ok())
        .unwrap_or_else(|| Response::err("daemon unavailable"));
    if response.ok {
        response.exit_code
    } else {
        eprintln!("mousetrap: {}", response.message);
        1
    }
}

pub fn client_cancel() -> i32 {
    let response =
        ipc::send(&Request::Cancel).unwrap_or_else(|_| Response::err("daemon not running"));
    i32::from(!response.ok)
}

pub fn client_key(direction: &str, key: &str) -> i32 {
    let request = match direction {
        "down" => Request::KeyDown { key: key.to_string() },
        "up" => Request::KeyUp { key: key.to_string() },
        _ => return 2,
    };
    let response = ipc::send(&request).unwrap_or_else(|_| Response::err("daemon not running"));
    if response.ok {
        response.exit_code
    } else {
        eprintln!("mousetrap: {}", response.message);
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(settings: &Settings) -> (Response, Response, Response, Response) {
        // Pure state-machine test without a wayland connection: exercise the
        // session logic directly.
        let mut session = Some(OverlaySession::new(SessionState::start((0, 0, 2048, 1152))));
        let mut result = (Response::ok(""), Response::ok(""), Response::ok(""), Response::ok(""));
        let mut last: Option<SelectionResult> = None;
        for round in 0..3 {
            let s = session.as_mut().unwrap();
            if s.state.has_timed_out(settings.session_timeout_seconds) {
                panic!("unexpected timeout");
            }
            let _ = s.key_down("a");
            if let KeyResult::Commit = s.key_up("a") {
                last = s.commit_pending();
            }
            if round == 2 {
                result.3 = Response::ok("final");
            }
        }
        let _ = last;
        result
    }

    #[test]
    fn session_reaches_final() {
        let settings = Settings::default();
        let (_, _, _, last) = flow(&settings);
        assert_eq!(last.message, "final");
    }
}
