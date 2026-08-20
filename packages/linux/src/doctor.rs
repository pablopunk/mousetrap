//! Runtime environment checks (`mousetrap doctor`).

use std::os::unix::fs::OpenOptionsExt;

/// Where the compositor socket lives; presence means "inside a Wayland session".
fn wayland_runtime_dir() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(std::path::PathBuf::from(dir))
}

fn wayland_socket_present() -> bool {
    match wayland_runtime_dir() {
        Some(dir) => std::fs::read_dir(&dir)
            .map(|entries| {
                entries.flatten().any(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy().into_owned();
                    name.starts_with("wayland-")
                })
            })
            .unwrap_or(false),
        None => false,
    }
}

fn uinput_writable() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/uinput")
        .is_ok()
}

pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub detail: &'static str,
}

pub fn run_checks() -> Vec<Check> {
    vec![
        Check {
            name: "wayland-session",
            ok: wayland_socket_present(),
            detail: "requires $XDG_RUNTIME_DIR with a wayland-* socket",
        },
        Check {
            name: "uinput",
            ok: uinput_writable(),
            detail: "write access to /dev/uinput (see udev rule in README)",
        },
        Check {
            name: "config",
            ok: crate::config::config_path().exists(),
            detail: "config file created via `mousetrap init-config`",
        },
    ]
}

pub fn run() -> i32 {
    let mut failures = 0;
    for check in run_checks() {
        let status = if check.ok { "ok" } else { "missing" };
        println!("[{status}] {}: {}", check.name, check.detail);
        if !check.ok && check.name != "config" {
            failures += 1;
        }
    }
    i32::from(failures > 0)
}
