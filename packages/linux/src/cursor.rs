//! Best-effort cursor position lookup for free-mouse indicators.
//!
//! Wayland intentionally does not expose a general cursor-position query.
//! Hyprland does expose one through its read-only IPC, so use it when present
//! and let the relative uinput path keep movement working elsewhere.

use serde_json::Value;

fn command_json(args: &[&str]) -> Option<Value> {
    let output = std::process::Command::new("hyprctl")
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

/// Return the current cursor position local to the monitor containing it.
pub fn query(screen_size: (i32, i32)) -> Option<(i32, i32)> {
    let cursor = command_json(&["cursorpos", "-j"])?;
    let x = cursor.get("x")?.as_i64()? as i32;
    let y = cursor.get("y")?.as_i64()? as i32;
    let monitors = command_json(&["monitors", "-j"])?;
    let monitors = monitors.as_array()?;

    for monitor in monitors {
        let mx = monitor.get("x")?.as_i64()? as i32;
        let my = monitor.get("y")?.as_i64()? as i32;
        let width = monitor.get("width")?.as_i64()? as i32;
        let height = monitor.get("height")?.as_i64()? as i32;
        let scale = monitor
            .get("scale")
            .and_then(Value::as_f64)
            .filter(|scale| *scale > 0.0)
            .unwrap_or(1.0);
        let logical_width = (width as f64 / scale).round() as i32;
        let logical_height = (height as f64 / scale).round() as i32;
        if logical_width > 0
            && logical_height > 0
            && x >= mx
            && x < mx + logical_width
            && y >= my
            && y < my + logical_height
        {
            return Some((x - mx, y - my));
        }
    }

    // A compositor-specific query can return a coordinate without monitor
    // metadata during a monitor reconfiguration. Keep the indicator usable
    // if that coordinate already looks local to our layer surface.
    let (width, height) = screen_size;
    if x >= 0 && y >= 0 && x < width && y < height {
        Some((x, y))
    } else {
        None
    }
}
