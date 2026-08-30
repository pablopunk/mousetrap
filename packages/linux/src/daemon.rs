//! Resident daemon: owns the overlay, session state, and virtual pointer.
//!
//! CLI invocations talk to it over a UNIX socket (`ipc.rs`). The daemon runs
//! a single-threaded calloop event loop that integrates:
//!   - the Wayland connection (layer-shell overlay events)
//!   - an IPC channel fed by a small accept/read thread
//!   - periodic timers (session timeout, click sequencing)

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant, SystemTime};

use calloop::EventLoop;
use calloop::channel::{Event as ChannelEvent, channel};
use calloop::timer::{TimeoutAction, Timer};
use calloop_wayland_source::WaylandSource;
use sctk::compositor::{CompositorHandler, CompositorState};
use sctk::output::{OutputHandler, OutputState};
use sctk::registry::{ProvidesRegistryState, RegistryState};
use sctk::shell::wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure};
use sctk::shm::{Shm, ShmHandler};
use sctk::{delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm};
use smithay_client_toolkit as sctk;
use wayland_client::globals::{GlobalList, registry_queue_init};
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::{Connection, QueueHandle};

use crate::config::Settings;
use crate::cursor;
use crate::geometry::{Bounds, rect_center};
use crate::input::{BTN_LEFT, VirtualPointer};
use crate::ipc::{self, Request, Response};
use crate::keys::{ArrowDirection, KeyEvent, KeyboardGrab};
use crate::mouse::{MouseEvent, MouseObserver};
use crate::overlay::Overlay;
use crate::shortcuts::{ShortcutEvent, ShortcutState};
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
    free_mouse: Option<FreeMouseState>,
    pending_free_mouse_click: Option<PendingFreeMouseClick>,
    free_mouse_drag_active: bool,
    mouse_observer: Option<MouseObserver>,

    action: Option<PendingAction>,
    keys_tx: calloop::channel::Sender<KeyEvent>,
    /// Timestamp of the last tray activate; used to debounce duplicate
    /// click events from tray hosts (press + release double-firing).
    last_tray_activate: Option<Instant>,
    /// Global-shortcut registration state (labels tray menu, help panel).
    shortcut_state: Arc<std::sync::Mutex<ShortcutState>>,
    /// Commit deadline after the last chord key is released. A subsequent
    /// key-down before this deadline joins the same chord (macOS parity).
    chord_commit_at: Option<Instant>,
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

#[derive(Clone, Copy, Debug)]
struct FreeMouseState {
    position: (i32, i32),
    screen_size: (i32, i32),
    /// Whether absolute movement can keep the tracked and physical positions aligned.
    position_known: bool,
    last_activity: Instant,
}

#[derive(Clone, Copy, Debug)]
struct PendingFreeMouseClick {
    point: (i32, i32),
    ready_at: Instant,
}

#[derive(Clone, Copy, Debug)]
enum FreeClick {
    Single,
    Double,
    Right,
}

impl Daemon {
    fn new(
        globals: &GlobalList,
        qh: QueueHandle<Daemon>,
        keys_tx: calloop::channel::Sender<KeyEvent>,
        shortcut_state: Arc<std::sync::Mutex<ShortcutState>>,
    ) -> Result<Self, String> {
        let registry_state = RegistryState::new(globals);
        let compositor =
            CompositorState::bind(globals, &qh).map_err(|e| format!("wl_compositor: {e:?}"))?;
        let shm = Shm::bind(globals, &qh).map_err(|e| format!("wl_shm: {e:?}"))?;
        let layer_shell =
            LayerShell::bind(globals, &qh).map_err(|e| format!("zwlr_layer_shell_v1: {e:?}"))?;
        let output_state = OutputState::new(globals, &qh);
        let settings = Settings::load();
        Ok(Self {
            registry_state,
            compositor,
            shm,
            layer_shell,
            output_state,
            qh,
            overlay: Overlay::new(),
            settings,
            session: None,
            pointer: None,
            pointer_warning: None,
            keyboard: None,
            keyboard_warning: None,
            free_mouse: None,
            pending_free_mouse_click: None,
            free_mouse_drag_active: false,
            mouse_observer: None,
            action: None,
            keys_tx,
            last_tray_activate: None,
            shortcut_state,
            chord_commit_at: None,
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
            Request::SetSetting { key, value } => self.set_setting(&key, &value),
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
            self.key_processed_at.store(
                SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos() as u64)
                    .unwrap_or(0),
                std::sync::atomic::Ordering::Relaxed,
            );
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
            let Daemon {
                overlay,
                compositor,
                layer_shell,
                qh,
                ..
            } = self;
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

    fn enter_free_mouse(&mut self, direction: ArrowDirection, shift: bool) {
        if self.free_mouse.is_none() {
            // Transition from grid to free mode without releasing the grab.
            // The same keyboard stream must consume the arrow that triggered
            // the transition, just as the macOS event tap does.
            let current_bounds = self
                .session
                .as_ref()
                .map(|session| session.state.current_bounds)
                .unwrap_or(self.overlay.bounds);
            self.session = None;
            self.chord_commit_at = None;
            self.action = None;

            let screen_size = (
                (self.overlay.size.0 as i32).max(1),
                (self.overlay.size.1 as i32).max(1),
            );
            let position = free_mouse_start_position(current_bounds, screen_size);
            let position_known = self.pointer.as_mut().is_some_and(|pointer| {
                match pointer.move_abs(position.0, position.1, screen_size.0, screen_size.1) {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("mousetrap: free cursor initial move failed: {e}");
                        false
                    }
                }
            });
            self.free_mouse = Some(FreeMouseState {
                position,
                screen_size,
                position_known,
                last_activity: Instant::now(),
            });
            self.overlay.show_indicator(position);
            self.overlay.redraw(&self.shm);
        }
        self.move_free_mouse(direction, shift);
    }

    fn move_free_mouse(&mut self, direction: ArrowDirection, shift: bool) {
        self.cancel_pending_free_mouse_click();
        if !shift {
            self.end_free_mouse_drag();
        } else if !self.free_mouse_drag_active {
            if let Some(pointer) = self.pointer.as_mut() {
                match pointer.drag_start(BTN_LEFT) {
                    Ok(()) => self.free_mouse_drag_active = true,
                    Err(e) => eprintln!("mousetrap: drag start failed: {e}"),
                }
            }
        }
        let Some(state) = self.free_mouse else { return };
        let step = self.settings.free_mouse_step.clamp(4.0, 80.0).round() as i32;
        let (dx, dy) = match direction {
            ArrowDirection::Up => (0, -step),
            ArrowDirection::Down => (0, step),
            ArrowDirection::Left => (-step, 0),
            ArrowDirection::Right => (step, 0),
        };
        let target = clamp_point(
            (state.position.0 + dx, state.position.1 + dy),
            state.screen_size,
        );
        let actual_dx = target.0 - state.position.0;
        let actual_dy = target.1 - state.position.1;
        let moved = if actual_dx == 0 && actual_dy == 0 {
            true
        } else if let Some(pointer) = self.pointer.as_mut() {
            let result = if state.position_known {
                pointer.move_abs(target.0, target.1, state.screen_size.0, state.screen_size.1)
            } else {
                pointer.move_relative(actual_dx, actual_dy)
            };
            if let Err(e) = result {
                eprintln!("mousetrap: free cursor move failed: {e}");
                false
            } else {
                true
            }
        } else {
            false
        };

        if moved {
            if let Some(state) = &mut self.free_mouse {
                state.position = target;
                state.last_activity = Instant::now();
            }
            self.overlay.update_indicator(target);
            self.overlay.invalidate();
            self.overlay.redraw(&self.shm);
        }
    }

    fn handle_free_mouse_enter(&mut self, shift: bool) {
        self.end_free_mouse_drag();
        let Some(state) = self.free_mouse else { return };
        if let Some(state) = &mut self.free_mouse {
            state.last_activity = Instant::now();
        }

        if shift {
            self.cancel_pending_free_mouse_click();
            self.finish_free_mouse_click(state.position, FreeClick::Right);
        } else if self.pending_free_mouse_click.is_some() {
            self.cancel_pending_free_mouse_click();
            self.finish_free_mouse_click(state.position, FreeClick::Double);
        } else {
            self.pending_free_mouse_click = Some(PendingFreeMouseClick {
                point: state.position,
                ready_at: Instant::now() + Duration::from_millis(250),
            });
        }
    }

    fn finish_free_mouse_click(&mut self, point: (i32, i32), click: FreeClick) {
        // Release the grab before posting the click so it is delivered to the
        // application beneath the pointer, not to Mousetrap's input stream.
        let position_known = self
            .free_mouse
            .map(|state| state.position_known)
            .unwrap_or(true);
        self.cancel_pending_free_mouse_click();
        self.end_free_mouse_drag();
        self.free_mouse = None;
        if let Some(mut keyboard) = self.keyboard.take() {
            keyboard.stop();
        }
        self.overlay.hide();

        let Some(pointer) = self.pointer.as_mut() else {
            return;
        };
        let screen_size = (
            (self.overlay.size.0 as i32).max(1),
            (self.overlay.size.1 as i32).max(1),
        );
        if position_known {
            if let Err(e) = pointer.move_abs(point.0, point.1, screen_size.0, screen_size.1) {
                eprintln!("mousetrap: free click cursor move failed: {e}");
                return;
            }
        }
        let result = match click {
            FreeClick::Single => pointer.left_click(),
            FreeClick::Double => pointer.double_click(
                BTN_LEFT,
                self.settings.double_click_interval_seconds.max(0.0),
            ),
            FreeClick::Right => pointer.right_click(),
        };
        if let Err(e) = result {
            eprintln!("mousetrap: free click failed: {e}");
        }
    }

    fn end_free_mouse_drag(&mut self) {
        if !self.free_mouse_drag_active {
            return;
        }
        self.free_mouse_drag_active = false;
        if let Some(pointer) = self.pointer.as_mut() {
            if let Err(e) = pointer.drag_end(BTN_LEFT) {
                eprintln!("mousetrap: drag release failed: {e}");
            }
        }
    }

    fn cancel_pending_free_mouse_click(&mut self) {
        self.pending_free_mouse_click = None;
    }

    fn key_down(&mut self, key: &str) -> Response {
        let normalized = key.trim().to_lowercase();
        if let Some((direction, shift)) = parse_arrow(&normalized) {
            if self.session.is_some() || self.free_mouse.is_some() {
                self.handle_arrow(direction, shift);
                return Response::ok("moved");
            }
            return Response::err("no active session");
        }
        match normalized.as_str() {
            "escape" => {
                self.cancel_session();
                return Response::ok("cancelled");
            }
            "enter" => {
                self.handle_enter(false);
                return Response::ok("handled");
            }
            "shift+enter" => {
                self.handle_enter(true);
                return Response::ok("handled");
            }
            "space" => {
                self.handle_space();
                return Response::ok("handled");
            }
            "delete" | "backspace" => {
                self.handle_delete();
                return Response::ok("handled");
            }
            _ => {}
        }
        if self.free_mouse.is_some() {
            // Grid characters are swallowed while free mode is active, but a
            // character also ends an active drag just like macOS.
            self.end_free_mouse_drag();
            return Response::ok("ignored");
        }
        // A new key during the grace period joins the pending chord.
        self.chord_commit_at = None;
        let Some(session) = self.session.as_mut() else {
            return Response::err("no active session");
        };
        if session
            .state
            .has_timed_out(self.settings.session_timeout_seconds)
        {
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

    fn handle_arrow(&mut self, direction: ArrowDirection, shift: bool) {
        if self.free_mouse.is_some() {
            self.move_free_mouse(direction, shift);
        } else if self.session.is_some() {
            self.enter_free_mouse(direction, shift);
        }
    }

    fn handle_enter(&mut self, shift: bool) {
        if self.free_mouse.is_some() {
            self.handle_free_mouse_enter(shift);
        } else if self.session.is_some() {
            self.click_grid_cursor(shift);
        }
    }

    fn handle_space(&mut self) {
        self.handle_enter(false);
    }

    fn handle_delete(&mut self) {
        if self.free_mouse.is_some() {
            self.end_free_mouse_drag();
        }
    }

    fn click_grid_cursor(&mut self, right: bool) {
        let screen_size = (
            (self.overlay.size.0 as i32).max(1),
            (self.overlay.size.1 as i32).max(1),
        );
        let point = cursor::query(screen_size)
            .map(|point| clamp_point(point, screen_size))
            .unwrap_or((screen_size.0 / 2, screen_size.1 / 2));

        self.session = None;
        self.chord_commit_at = None;
        self.action = None;
        if let Some(mut keyboard) = self.keyboard.take() {
            keyboard.stop();
        }
        self.overlay.hide();

        let Some(pointer) = self.pointer.as_mut() else {
            return;
        };
        if let Err(e) = pointer.move_abs(point.0, point.1, screen_size.0, screen_size.1) {
            eprintln!("mousetrap: grid click cursor move failed: {e}");
            return;
        }
        let result = if right {
            pointer.right_click()
        } else {
            pointer.left_click()
        };
        if let Err(e) = result {
            eprintln!("mousetrap: grid click failed: {e}");
        }
    }

    fn physical_mouse_moved(&mut self) {
        if self.session.is_some() || self.free_mouse.is_some() {
            eprintln!("mousetrap: physical mouse movement; resetting to safe state");
            self.cancel_session();
        }
    }

    fn key_up(&mut self, key: &str) -> Response {
        if self.free_mouse.is_some() || is_special_key(key) {
            return Response::ok("ignored");
        }
        let Some(session) = self.session.as_mut() else {
            return Response::err("no active session");
        };
        if let KeyResult::Commit = session.key_up(key) {
            let grace = self.settings.chord_timeout_seconds.max(0.0);
            if grace == 0.0 {
                return self.commit_pending_chord();
            }
            self.chord_commit_at = Some(Instant::now() + Duration::from_secs_f64(grace));
        }
        Response::ok("pending")
    }

    fn commit_pending_chord(&mut self) -> Response {
        self.chord_commit_at = None;
        let selection = self
            .session
            .as_mut()
            .and_then(OverlaySession::commit_pending);
        let Some(selection) = selection else {
            return Response::ok("no pending selection");
        };
        self.refresh_overlay();
        if selection.is_final {
            self.commit_selection(selection)
        } else {
            Response::ok("selected")
        }
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
        self.cancel_pending_free_mouse_click();
        self.end_free_mouse_drag();
        // Release the keyboard grab first so keystrokes resume reaching
        // applications immediately.
        if let Some(mut keyboard) = self.keyboard.take() {
            keyboard.stop();
        }
        self.session = None;
        self.free_mouse = None;
        self.action = None;
        self.chord_commit_at = None;
        let Daemon { overlay, .. } = self;
        overlay.hide();
        self.run_optional_command(&self.settings.on_cancel_command.clone());
    }

    /// Toggle the grid from the global shortcut: show if hidden, hide if
    /// visible (matches the macOS toggle behavior).
    fn toggle_from_shortcut(&mut self) {
        if self.session.is_some() {
            self.cancel_session();
        } else {
            self.activate();
        }
    }

    /// Show shortcut setup instructions through the desktop notification
    /// service. This must never map an overlay or capture keyboard input.
    fn show_shortcut_help(&mut self) {
        let state = self
            .shortcut_state
            .lock()
            .map(|state| state.clone())
            .unwrap_or(ShortcutState::Registering);
        crate::tray::notify_shortcut_setup(state);
    }

    fn refresh_overlay(&mut self) {
        let Daemon {
            overlay,
            session,
            shm,
            ..
        } = self;
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

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            eprintln!("mousetrap: settings save failed: {e}");
        }
    }

    fn set_setting(&mut self, key: &str, value: &str) -> Response {
        match key {
            "free-mouse-step" => {
                let Ok(step) = value.parse::<f64>() else {
                    return Response::err("free-mouse-step must be a number");
                };
                if !step.is_finite() {
                    return Response::err("free-mouse-step must be finite");
                }
                self.settings.free_mouse_step = step.clamp(1.0, 20.0);
                self.save_settings();
                Response::ok("free-mouse-step updated")
            }
            "session-timeout-seconds" => {
                let Ok(timeout) = value.parse::<f64>() else {
                    return Response::err("session-timeout-seconds must be a number");
                };
                if !timeout.is_finite() {
                    return Response::err("session-timeout-seconds must be finite");
                }
                self.settings.session_timeout_seconds = timeout.clamp(3.0, 60.0);
                self.save_settings();
                Response::ok("session-timeout-seconds updated")
            }
            "double-click-interval-seconds" => {
                let Ok(interval) = value.parse::<f64>() else {
                    return Response::err("double-click-interval-seconds must be a number");
                };
                if !interval.is_finite() {
                    return Response::err("double-click-interval-seconds must be finite");
                }
                self.settings.double_click_interval_seconds = interval.clamp(0.05, 0.5);
                self.save_settings();
                Response::ok("double-click-interval-seconds updated")
            }
            "launch-at-login" => {
                let Some(enabled) = parse_bool(value) else {
                    return Response::err("launch-at-login must be on or off");
                };
                self.set_launch_at_login(enabled)
            }
            _ => Response::err(format!("unknown setting: {key}")),
        }
    }

    fn set_launch_at_login(&mut self, enabled: bool) -> Response {
        let action = if enabled { "enable" } else { "disable" };
        match std::process::Command::new("systemctl")
            .args(["--user", action, "app-mousetrap.service"])
            .status()
        {
            Ok(status) if status.success() => Response::ok("launch-at-login updated"),
            Ok(status) => Response::err(format!("systemd {action} failed ({status})")),
            Err(e) => Response::err(format!("cannot change launch-at-login: {e}")),
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
                        if let Err(e) =
                            pointer.move_abs(action.point.0, action.point.1, w as i32, h as i32)
                        {
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
        if self
            .pending_free_mouse_click
            .is_some_and(|click| Instant::now() >= click.ready_at)
        {
            if let Some(click) = self.pending_free_mouse_click {
                self.finish_free_mouse_click(click.point, FreeClick::Single);
            }
        }
        if self.overlay.has_indicator() {
            self.overlay.invalidate();
            self.overlay.redraw(&self.shm);
        }
        if self.chord_commit_at.is_some_and(|at| Instant::now() >= at) {
            let _ = self.commit_pending_chord();
        }
        if let Some(action) = &mut self.action {
            let now = Instant::now();
            if now >= action.ready_at {
                self.advance_action();
                return TimeoutAction::ToDuration(Duration::from_millis(25));
            }
            let remaining = action.ready_at.saturating_duration_since(now);
            return TimeoutAction::ToDuration(remaining.max(Duration::from_millis(1)));
        }
        // Keep chord commits within one frame of their 80ms deadline. The
        // timer cannot be rescheduled directly from an evdev channel event.
        TimeoutAction::ToDuration(Duration::from_millis(25))
    }

    /// Periodic tick: enforce the session timeout.
    fn tick(&mut self) {
        self.heartbeat
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(session) = &self.session {
            if session
                .state
                .has_timed_out(self.settings.session_timeout_seconds)
            {
                eprintln!("mousetrap: session timed out; cancelling");
                self.cancel_session();
            }
        }
        if self.free_mouse.as_ref().is_some_and(|state| {
            self.settings.session_timeout_seconds > 0.0
                && state.last_activity.elapsed().as_secs_f64()
                    > self.settings.session_timeout_seconds
        }) {
            eprintln!("mousetrap: free-mouse timeout reached; cancelling");
            self.cancel_session();
        }
    }
}

fn clamp_point(point: (i32, i32), size: (i32, i32)) -> (i32, i32) {
    (
        point.0.clamp(0, size.0.saturating_sub(1)),
        point.1.clamp(0, size.1.saturating_sub(1)),
    )
}

fn free_mouse_start_position(bounds: Bounds, screen_size: (i32, i32)) -> (i32, i32) {
    let center = if bounds.2 > 0 && bounds.3 > 0 {
        rect_center(bounds)
    } else {
        (screen_size.0 / 2, screen_size.1 / 2)
    };
    clamp_point(center, screen_size)
}

fn parse_arrow(key: &str) -> Option<(ArrowDirection, bool)> {
    let (shift, direction) = match key.strip_prefix("shift+") {
        Some(direction) => (true, direction),
        None => (false, key),
    };
    let direction = match direction {
        "up" => ArrowDirection::Up,
        "down" => ArrowDirection::Down,
        "left" => ArrowDirection::Left,
        "right" => ArrowDirection::Right,
        _ => return None,
    };
    Some((direction, shift))
}

fn is_special_key(key: &str) -> bool {
    let key = key.trim().to_lowercase();
    parse_arrow(&key).is_some()
        || matches!(
            key.as_str(),
            "escape" | "enter" | "shift+enter" | "space" | "delete" | "backspace"
        )
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

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
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
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        // A destroy of an OLD surface can deliver `closed` after a NEW
        // surface has already been shown; only hide when it is still the
        // current surface.
        let is_current = self.overlay.is_current_surface(layer);
        if is_current {
            // Losing the layer while a keyboard grab/free drag is active is
            // an unsafe state; release all input ownership immediately.
            self.cancel_session();
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Ignore configure events for surfaces that are no longer current
        // (an old surface's destroy can race a fresh re-show).
        let is_current = self.overlay.is_current_surface(layer);
        if !is_current {
            return;
        }
        let Daemon { overlay, .. } = self;
        overlay.configure(configure.new_size);
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
            // A socket file already exists. If something is actually
            // listening on it, another daemon is alive: exit quietly rather
            // than register a second tray icon (exit code 0 so systemd does
            // not treat it as a failure to restart).
            if UnixStream::connect(&path).is_ok() {
                eprintln!("mousetrap: another instance is already running; exiting");
                std::process::exit(0);
            }
            // The connect was refused: the file is stale (a crashed or
            // killed daemon left it behind) and safe to replace.
            let _ = std::fs::remove_file(&path);
            UnixListener::bind(&path).expect("bind ipc socket")
        }
    }
}

fn ipc_accept_loop(listener: UnixListener, tx: calloop::channel::Sender<IpcMessage>) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut line = String::new();
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
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

/// Ensure a Wayland display is selected. A systemd user manager does not
/// inherit the graphical session's environment, so when started from a
/// service the daemon finds the socket itself: the newest `wayland-*` socket
/// in `XDG_RUNTIME_DIR` wins (older ones belong to dead compositors).
fn resolve_wayland_display() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return;
    }
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(runtime) else {
        return;
    };
    let mut newest: Option<(SystemTime, std::ffi::OsString)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        let Some(num) = name_str.strip_prefix("wayland-") else {
            continue;
        };
        if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
            newest = Some((modified, name));
        }
    }
    if let Some((_, name)) = newest {
        // SAFETY: this runs before the event loop or any threads exist,
        // so there is no possible concurrent reader of the environment.
        unsafe { std::env::set_var("WAYLAND_DISPLAY", name) };
    }
}

/// Run the resident daemon in the foreground.
pub fn run() -> i32 {
    resolve_wayland_display();
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
    let shortcut_state = Arc::new(std::sync::Mutex::new(ShortcutState::Registering));
    let mut daemon = match Daemon::new(&globals, qh.clone(), keys_tx, shortcut_state.clone()) {
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

    if let Err(e) = handle.insert_source(
        WaylandSource::new(conn.clone(), event_queue),
        |_, queue, data| match queue.dispatch_pending(data) {
            Ok(count) => Ok(count),
            Err(err) => {
                eprintln!("mousetrap: wayland error: {err:?}; exiting");
                std::process::exit(1);
            }
        },
    ) {
        eprintln!("mousetrap: cannot start wayland source: {e}");
        return 1;
    }

    // IPC channel fed by the accept thread.
    let (ipc_tx, ipc_rx) = channel::<IpcMessage>();
    let listener = bind_socket();
    std::thread::spawn(move || ipc_accept_loop(listener, ipc_tx));
    if let Err(e) = handle.insert_source(ipc_rx, |event, _, app| {
        if let ChannelEvent::Msg((request, reply)) = event {
            app.heartbeat
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            app.heartbeat
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                KeyEvent::Arrow { direction, shift } => app.handle_arrow(direction, shift),
                KeyEvent::Enter { shift } => app.handle_enter(shift),
                KeyEvent::Space => app.handle_space(),
                KeyEvent::Delete => app.handle_delete(),
            }
        }
    }) {
        eprintln!("mousetrap: cannot start keyboard source: {e}");
        return 1;
    }

    // Passive physical-pointer observer. It never grabs a device and filters
    // Mousetrap's own virtual pointer in mouse.rs.
    let (mouse_tx, mouse_rx) = channel::<MouseEvent>();
    daemon.mouse_observer = Some(MouseObserver::start(mouse_tx));
    if let Err(e) = handle.insert_source(mouse_rx, |event, _, app| {
        if let ChannelEvent::Msg(MouseEvent::Moved) = event {
            app.heartbeat
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            app.physical_mouse_moved();
        }
    }) {
        eprintln!("mousetrap: cannot start mouse source: {e}");
        return 1;
    }

    // Tray (StatusNotifierItem over DBus).
    let (tray_tx, tray_rx) = channel::<TrayEvent>();
    crate::tray::spawn(tray_tx, shortcut_state.clone());
    if let Err(e) = handle.insert_source(tray_rx, |event, _, app| {
        if let ChannelEvent::Msg(event) = event {
            match event {
                TrayEvent::Activate => {
                    // Tray hosts may deliver the menu click on both press
                    // and release; debounce so one click = one grid.
                    let now = Instant::now();
                    let duplicate = app
                        .last_tray_activate
                        .map(|t| now.duration_since(t) < Duration::from_millis(500))
                        .unwrap_or(false);
                    app.last_tray_activate = Some(now);
                    if !duplicate {
                        let _ = app.process_request(Request::Activate);
                    }
                }
                TrayEvent::Cancel => {
                    let _ = app.process_request(Request::Cancel);
                }
                TrayEvent::ShortcutHelp => {
                    app.show_shortcut_help();
                }
                TrayEvent::OpenSettings => {
                    crate::tray::launch_settings();
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

    // Global toggle shortcut (XDG portal). Runs on its own thread; the
    // Activated event toggles the grid.
    let (shortcut_tx, shortcut_rx) = channel::<ShortcutEvent>();
    crate::shortcuts::spawn(shortcut_tx, shortcut_state);
    if let Err(e) = handle.insert_source(shortcut_rx, |event, _, app| {
        if let ChannelEvent::Msg(ShortcutEvent::Activated) = event {
            app.heartbeat
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            app.toggle_from_shortcut();
        }
    }) {
        eprintln!("mousetrap: cannot start shortcut source: {e}");
        return 1;
    }

    // Periodic session-timeout check.
    let tick = Timer::from_duration(Duration::from_millis(500));
    if let Err(e) = handle.insert_source(tick, |_, _, app| {
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
            // Preferred revival path: the installed systemd user unit
            // (survives the CLI, restarts on failure, autostarts on login).
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "start", "--no-block", "app-mousetrap"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            for _ in 0..40 {
                std::thread::sleep(Duration::from_millis(25));
                if let Ok(response) = ipc::send(&Request::Ping) {
                    return Some(response);
                }
            }
            // Fallback: no systemd unit installed — spawn a detached daemon.
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
        "down" => Request::KeyDown {
            key: key.to_string(),
        },
        "up" => Request::KeyUp {
            key: key.to_string(),
        },
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

pub fn client_set_setting(key: &str, value: &str) -> i32 {
    let request = Request::SetSetting {
        key: key.to_string(),
        value: value.to_string(),
    };
    let response = ensure_daemon()
        .and_then(|_| ipc::send(&request).ok())
        .unwrap_or_else(|| Response::err("daemon unavailable"));
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
        let mut result = (
            Response::ok(""),
            Response::ok(""),
            Response::ok(""),
            Response::ok(""),
        );
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

    #[test]
    fn free_mouse_coordinates_stop_at_screen_edges() {
        assert_eq!(clamp_point((-5, 20), (100, 50)), (0, 20));
        assert_eq!(clamp_point((100, 50), (100, 50)), (99, 49));
    }

    #[test]
    fn free_mouse_starts_at_current_grid_center() {
        assert_eq!(
            free_mouse_start_position((320, 120, 400, 200), (1920, 1080)),
            (520, 220)
        );
        assert_eq!(
            free_mouse_start_position((0, 0, 0, 0), (1920, 1080)),
            (960, 540)
        );
    }

    #[test]
    fn free_mouse_key_names_include_shift_variants() {
        assert!(matches!(
            parse_arrow("up"),
            Some((ArrowDirection::Up, false))
        ));
        assert!(matches!(
            parse_arrow("shift+right"),
            Some((ArrowDirection::Right, true))
        ));
        assert!(is_special_key("shift+enter"));
        assert!(!is_special_key("a"));
    }
}
