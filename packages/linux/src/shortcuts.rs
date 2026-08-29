//! Global toggle shortcut via the XDG Desktop Portal (`GlobalShortcuts`).
//!
//! On Wayland only the compositor can own global key combinations. The
//! standard interface is the GlobalShortcuts portal:
//!   1. `CreateSession` → session handle
//!   2. `BindShortcuts(["toggle"])` → the compositor registers the shortcut
//!   3. `Activated` signal → the daemon toggles the grid
//!
//! Composer-specifics:
//! - Hyprland's portal registers the shortcut *id* (`<appid>:toggle`); the
//!   combo itself is a user-defined bind in the Hyprland config using the
//!   `global` dispatcher. The app never touches the user's config, and
//!   claims no combo by default.
//! - Compositors with a full portal implementation (e.g. KDE) present their
//!   own shortcut picker on first bind and report the combo back via
//!   `trigger_description`.
//!
//! Runs on its own thread (zbus blocking API); events are forwarded to the
//! main loop through a calloop channel, state through a shared mutex.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use calloop::channel::Sender;
use zbus::blocking::{Connection, MessageIterator, Proxy};
use zbus::message::Type;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::MatchRule;

pub const SHORTCUT_ID: &str = "toggle";
pub const SHORTCUT_DESCRIPTION: &str = "Toggle the Mousetrap grid";

/// State published to the tray menu label and the help overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutState {
    /// The portal is not available on this compositor.
    Unavailable(String),
    /// Registration in progress.
    Registering,
    /// Registered. `trigger` is a human-readable combo when the compositor
    /// reports one; empty means the combo lives in the user's compositor
    /// config (Hyprland model).
    Registered { appid: String, trigger: String },
}

pub enum ShortcutEvent {
    /// The user pressed the toggle shortcut.
    Activated,
}

fn ov(value: impl Into<Value<'static>>) -> OwnedValue {
    OwnedValue::try_from(value.into()).expect("own our own values")
}

/// Whether the current compositor session is Hyprland.
fn is_hyprland() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_lowercase().contains("hyprland"))
        .unwrap_or(false)
}

/// Look up our registration with `hyprctl globalshortcuts -j` (read-only)
/// to learn the appid Hyprland associated with this daemon. Falls back to
/// "mousetrap" (the systemd unit / desktop file name) if hyprctl is absent
/// or the registration is not found yet.
fn hyprland_appid() -> String {
    let Ok(output) = std::process::Command::new("hyprctl")
        .args(["globalshortcuts", "-j"])
        .output()
    else {
        return "mousetrap".to_string();
    };
    if !output.status.success() {
        return "mousetrap".to_string();
    }
    let Ok(list) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) else {
        return "mousetrap".to_string();
    };
    list.iter()
        .find(|entry| entry.get("description").and_then(|d| d.as_str()) == Some(SHORTCUT_DESCRIPTION))
        .and_then(|entry| entry.get("name").and_then(|n| n.as_str()))
        .and_then(|name| name.split(':').next())
        .map(str::to_string)
        .unwrap_or_else(|| "mousetrap".to_string())
}

fn set_state(state: &Arc<Mutex<ShortcutState>>, next: ShortcutState) {
    *state.lock().expect("shortcut state mutex") = next;
}

/// Start the shortcuts thread. It registers the toggle shortcut and then
/// waits for activation signals for the lifetime of the daemon.
pub fn spawn(tx: Sender<ShortcutEvent>, state: Arc<Mutex<ShortcutState>>) {
    std::thread::spawn(move || {
        set_state(&state, ShortcutState::Registering);
        if let Err(e) = run(tx, &state) {
            set_state(&state, ShortcutState::Unavailable(e.to_string()));
            eprintln!("mousetrap: global shortcuts unavailable: {e}");
        }
    });
}

fn run(tx: Sender<ShortcutEvent>, state: &Arc<Mutex<ShortcutState>>) -> Result<(), String> {
    let conn = Connection::session().map_err(|e| e.to_string())?;
    let portal = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
    )
    .map_err(|e| e.to_string())?;

    // We receive Request::Response signals through a single broad iterator
    // and match each response to its request via the signal's object path
    // (the portal emits Response on the request's own object path).
    let mut responses = MessageIterator::for_match_rule(
        MatchRule::builder()
            .msg_type(Type::Signal)
            .sender("org.freedesktop.portal.Desktop")
            .map_err(|e| e.to_string())?
            .interface("org.freedesktop.portal.Request")
            .map_err(|e| e.to_string())?
            .member("Response")
            .map_err(|e| e.to_string())?
            .build(),
        &conn,
        Some(16),
    )
    .map_err(|e| e.to_string())?;

    // 1. CreateSession.
    let pid = std::process::id();
    let mut session_opts: HashMap<String, OwnedValue> = HashMap::new();
    session_opts.insert("handle_token".into(), ov(format!("mousetrap_session_{pid}")));
    session_opts.insert("session_handle_token".into(), ov(format!("mousetrap_{pid}")));
    let session_req = portal
        .call::<_, _, OwnedObjectPath>("CreateSession", &(session_opts,))
        .map_err(|e| e.to_string())?;
    let (code, results) = wait_response(&mut responses, session_req)?;
    if code != 0 {
        return Err(format!("CreateSession rejected (code {code})"));
    }
    let session_handle = results
        .get("session_handle")
        .and_then(|v| v.downcast_ref::<zbus::zvariant::Str<'_>>().ok())
        .ok_or_else(|| "portal returned no session handle".to_string())?
        .to_string();

    // 2. BindShortcuts. No preferred_trigger: the compositor owns the combo
    //    (Hyprland takes it from the user's config; KDE shows its picker).
    let mut shortcut: HashMap<String, OwnedValue> = HashMap::new();
    shortcut.insert("description".into(), ov(SHORTCUT_DESCRIPTION));
    let mut bind_opts: HashMap<String, OwnedValue> = HashMap::new();
    bind_opts.insert("handle_token".into(), ov(format!("mousetrap_bind_{pid}")));
    let bind_req = portal
        .call::<_, _, OwnedObjectPath>(
            "BindShortcuts",
            &(
                OwnedObjectPath::try_from(session_handle.as_str())
                    .map_err(|e| e.to_string())?,
                vec![(SHORTCUT_ID.to_string(), shortcut)],
                String::new(), // no parent window
                bind_opts,
            ),
        )
        .map_err(|e| e.to_string())?;
    let (code, results) = wait_response(&mut responses, bind_req)?;
    if code != 0 {
        return Err(format!("BindShortcuts rejected (code {code})"));
    }
    // The response's `shortcuts` is a(sa{sv}); extract the first entry's
    // `trigger_description` if the compositor reports one.
    let trigger = trigger_from_response(&results);

    let appid = if is_hyprland() { hyprland_appid() } else { "mousetrap".to_string() };
    set_state(state, ShortcutState::Registered { appid: appid.clone(), trigger: trigger.clone() });

    // Drop the response iterator so its match rule is deregistered.
    drop(responses);

    // 3. Wait for activation signals on the session object for the
    //    lifetime of the daemon.
    let session_path = OwnedObjectPath::try_from(session_handle.as_str())
        .map_err(|e| e.to_string())?;
    let session_proxy = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        session_path.clone(),
        "org.freedesktop.portal.GlobalShortcuts",
    )
    .map_err(|e| e.to_string())?;
    let mut signals = session_proxy.receive_signal("Activated").map_err(|e| e.to_string())?;
    for msg in &mut signals {
        let Ok((_session, id, _ts, _opts)) = msg
            .body()
            .deserialize::<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)>()
        else {
            continue;
        };
        if id == SHORTCUT_ID {
            let _ = tx.send(ShortcutEvent::Activated);
        }
    }
    Ok(())
}

/// Extract the first bound shortcut's `trigger_description` from a
/// BindShortcuts Response result dict. Empty when the compositor does not
/// report one (Hyprland model: the combo lives in user config).
fn trigger_from_response(results: &HashMap<String, OwnedValue>) -> String {
    let Some(shortcuts) = results.get("shortcuts") else {
        return String::new();
    };
    let Ok(array) = shortcuts.downcast_ref::<zbus::zvariant::Array<'_>>() else {
        return String::new();
    };
    let Ok(Some(first)) = array.get::<zbus::zvariant::Structure<'_>>(0) else {
        return String::new();
    };
    let Some(props) = first.fields().get(1) else {
        return String::new();
    };
    let Ok(dict) = props.downcast_ref::<zbus::zvariant::Dict<'_, '_>>() else {
        return String::new();
    };
    let Ok(Some(trigger)) = dict.get::<&str, zbus::zvariant::Str<'_>>(&"trigger_description")
    else {
        return String::new();
    };
    trigger.to_string()
}

/// Block until the Request::Response signal for `expected_handle` arrives.
fn wait_response(
    responses: &mut MessageIterator,
    expected_handle: OwnedObjectPath,
) -> Result<(u32, HashMap<String, OwnedValue>), String> {
    loop {
        let Some(msg) = responses.next() else {
            return Err("portal connection closed".to_string());
        };
        let msg = msg.map_err(|e| e.to_string())?;
        // The Response signal is emitted on the request's own object path;
        // ignore responses addressed to other requests or other apps.
        let header = msg.header();
        let Some(path) = header.path() else { continue };
        if path.as_str() != expected_handle.as_str() {
            continue;
        }
        let (code, results): (u32, HashMap<String, OwnedValue>) =
            msg.body().deserialize().map_err(|e| e.to_string())?;
        return Ok((code, results));
    }
}
