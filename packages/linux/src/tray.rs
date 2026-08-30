//! Tray presence via StatusNotifierItem (SNI) over DBus.
//!
//! Registers `org.kde.StatusNotifierItem-<pid>-1` on the session bus with a
//! `com.canonical.dbusmenu` menu (Activate / Cancel / Open Settings / Quit),
//! and registers with the StatusNotifierWatcher so trays like Quickshell's
//! display it.
//! Runs on its own thread (zbus blocking API); menu events are forwarded to
//! the main loop through a calloop channel.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use calloop::channel::Sender;
use zbus::blocking::{Connection, Proxy};
use zbus::interface;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::shortcuts::ShortcutState;

/// Events forwarded from the tray thread to the daemon's main loop.
#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    Activate,
    Cancel,
    Quit,
    OpenSettings,
    /// User clicked the "Toggle shortcut" menu item.
    ShortcutHelp,
}

const ICON_SIZE: usize = 64;

/// Decode the embedded app icon (straight-alpha RGBA PNG), downscale it
/// (nearest neighbor), and convert to ARGB32 (network byte order), as
/// expected by SNI `IconPixmap`.
fn icon_pixmap() -> Vec<(i32, i32, Vec<u8>)> {
    let png_bytes: &[u8] = include_bytes!("../assets/AppIcon.png");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .expect("embedded app icon is a valid png");
    let mut raw = vec![0; reader.output_buffer_size().expect("icon size known")];
    let info = reader.next_frame(&mut raw).expect("icon decodes");
    let (sw, sh) = (info.width as usize, info.height as usize);
    let src = &raw[..info.buffer_size()];

    let (dw, dh) = if sw >= sh {
        (ICON_SIZE, (sh * ICON_SIZE / sw).max(1))
    } else {
        ((sw * ICON_SIZE / sh).max(1), ICON_SIZE)
    };
    let mut bytes = Vec::with_capacity(dw * dh * 4);
    for y in 0..dh {
        for x in 0..dw {
            let sx = x * sw / dw;
            let sy = y * sh / dh;
            let p = &src[(sy * sw + sx) * 4..(sy * sw + sx) * 4 + 4];
            bytes.extend_from_slice(&[p[3], p[0], p[1], p[2]]); // ARGB big-endian
        }
    }
    vec![(dw as i32, dh as i32, bytes)]
}

pub struct Sni {
    pixmap: Vec<(i32, i32, Vec<u8>)>,
    tx: Sender<TrayEvent>,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl Sni {
    #[zbus(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }

    #[zbus(property)]
    fn id(&self) -> &str {
        "mousetrap"
    }

    #[zbus(property)]
    fn title(&self) -> &str {
        "Mousetrap"
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }

    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from("/MenuBar").expect("valid path")
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        self.pixmap.clone()
    }

    fn activate(&self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayEvent::OpenSettings);
    }
}

/// DBusMenu layout: (id, properties, children-as-variants).
type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

fn insert_prop(
    props: &mut HashMap<String, OwnedValue>,
    key: &str,
    value: impl Into<Value<'static>>,
) {
    let owned = OwnedValue::try_from(value.into()).expect("own our own values");
    props.insert(key.to_string(), owned);
}

fn item(id: i32, label: &str, is_separator: bool) -> OwnedValue {
    let mut props = HashMap::new();
    if is_separator {
        insert_prop(&mut props, "type", "separator");
    } else {
        insert_prop(&mut props, "label", label.to_string());
    }
    insert_prop(&mut props, "enabled", true);
    insert_prop(&mut props, "visible", true);
    let layout: Layout = (id, props, Vec::new());
    OwnedValue::try_from(Value::from(layout)).expect("own our own values")
}

pub struct Menu {
    tx: Sender<TrayEvent>,
    /// Current shortcut registration state (labels the menu item).
    shortcut_state: Arc<Mutex<ShortcutState>>,
}

fn shortcut_label(state: &ShortcutState) -> String {
    match state {
        ShortcutState::Unavailable(_) => "Set toggle shortcut…".to_string(),
        ShortcutState::Registering => "Set toggle shortcut…".to_string(),
        ShortcutState::Registered { trigger, .. } if !trigger.is_empty() => {
            format!("Shortcut: {trigger}")
        }
        ShortcutState::Registered { .. } => "Set toggle shortcut…".to_string(),
    }
}

/// Show shortcut setup through the standard freedesktop notification
/// service. This deliberately lives outside the overlay: setup information
/// must not cover the screen, focus an app, or capture keyboard input.
pub fn notify_shortcut_setup(state: ShortcutState) {
    std::thread::spawn(move || {
        if let Err(e) = send_shortcut_notification(&state) {
            eprintln!("mousetrap: cannot show shortcut notification: {e}");
        }
    });
}

fn send_shortcut_notification(state: &ShortcutState) -> Result<(), zbus::Error> {
    let (summary, body) = match state {
        ShortcutState::Unavailable(reason) => (
            "Mousetrap shortcut unavailable",
            format!("This compositor does not provide global shortcuts.\n\n{reason}"),
        ),
        ShortcutState::Registering => (
            "Mousetrap shortcut",
            "Registering the toggle action with the compositor…".to_string(),
        ),
        ShortcutState::Registered { trigger, .. } if !trigger.is_empty() => (
            "Mousetrap shortcut",
            format!(
                "Current toggle shortcut: {trigger}\n\nChange it in your system shortcut settings."
            ),
        ),
        ShortcutState::Registered { appid, .. } => (
            "Set Mousetrap toggle shortcut",
            format!("Bind your preferred keys to this Hyprland action:\n{appid}:toggle"),
        ),
    };

    let conn = Connection::session()?;
    let notifications = Proxy::new(
        &conn,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )?;
    let hints: HashMap<String, OwnedValue> = HashMap::new();
    let _: u32 = notifications.call(
        "Notify",
        &(
            "Mousetrap",
            0u32,
            "mousetrap",
            summary,
            body,
            Vec::<String>::new(),
            hints,
            10_000i32,
        ),
    )?;
    Ok(())
}

#[interface(name = "com.canonical.dbusmenu")]
impl Menu {
    fn get_layout(
        &self,
        _parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (u32, Layout) {
        let label = self
            .shortcut_state
            .lock()
            .map(|state| shortcut_label(&state))
            .unwrap_or_else(|_| "Toggle shortcut".to_string());
        let children = vec![
            item(1, "Show grid", false),
            item(2, "Cancel grid / free mouse", false),
            item(7, "Open Settings", false),
            item(5, &label, false),
            item(4, "", true),
            item(3, "Quit", false),
        ];
        let mut root_props = HashMap::new();
        insert_prop(&mut root_props, "children-display", "submenu");
        let root: Layout = (0, root_props, children);
        (1, root)
    }

    fn get_group_properties(
        &self,
        _ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        Vec::new()
    }

    fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) {
        if event_id != "clicked" {
            return;
        }
        let event = match id {
            1 => TrayEvent::Activate,
            2 => TrayEvent::Cancel,
            3 => TrayEvent::Quit,
            7 => TrayEvent::OpenSettings,
            5 => TrayEvent::ShortcutHelp,
            _ => return,
        };
        let _ = self.tx.send(event);
    }

    fn event_group(&self, _events: Vec<(i32, String, OwnedValue, u32)>) -> Vec<i32> {
        Vec::new()
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }
}

/// Start the tray on its own thread. The caller gets the thread handle;
/// the tray never returns unless the bus connection fails.
pub fn spawn(tx: Sender<TrayEvent>, shortcut_state: Arc<Mutex<ShortcutState>>) {
    std::thread::spawn(move || {
        if let Err(e) = run(tx, shortcut_state) {
            eprintln!("mousetrap: tray unavailable: {e}");
        }
    });
}

/// Open the generic GTK settings application outside the daemon's Wayland
/// event loop. GTK's application ID makes repeated launches present the
/// existing settings window instead of creating duplicates.
pub fn launch_settings() {
    std::thread::spawn(|| {
        let executable =
            std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("mousetrap"));
        match std::process::Command::new(executable)
            .arg("settings")
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!("mousetrap: settings launcher failed ({status})"),
            Err(e) => eprintln!("mousetrap: cannot open settings panel: {e}"),
        }
    });
}

fn run(
    tx: Sender<TrayEvent>,
    shortcut_state: Arc<Mutex<ShortcutState>>,
) -> Result<(), zbus::Error> {
    let conn = Connection::session()?;
    let service_name = format!("org.kde.StatusNotifierItem-{}-1", std::process::id());
    conn.request_name(service_name.as_str())?;

    let sni = Sni {
        pixmap: icon_pixmap(),
        tx: tx.clone(),
    };
    let menu = Menu { tx, shortcut_state };
    conn.object_server().at("/StatusNotifierItem", sni)?;
    conn.object_server().at("/MenuBar", menu)?;

    // Register with the StatusNotifierWatcher, retrying until a tray host
    // is present (the tray may start after us).
    let watcher = Proxy::new(
        &conn,
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
    )?;
    let mut registered = false;
    loop {
        if !registered {
            match watcher.call::<_, _, ()>("RegisterStatusNotifierItem", &(service_name.clone(),)) {
                Ok(()) => {
                    registered = true;
                }
                Err(e) => {
                    eprintln!("mousetrap: tray host not available yet ({e}); retrying");
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        } else {
            // Idle: keep the thread alive; the blocking connection serves
            // incoming method calls on its internal executor.
            std::thread::sleep(Duration::from_secs(5));
        }
    }
}
