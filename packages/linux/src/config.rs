//! User settings, stored as JSON at `~/.config/mousetrap/config.json`
//! (or `$XDG_CONFIG_HOME/mousetrap/config.json`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Seconds the overlay stays visible before the click lands.
    pub overlay_dismiss_delay_seconds: f64,
    /// Seconds to wait after moving the pointer, before clicking.
    pub pre_warp_delay_seconds: f64,
    /// Seconds to wait after clicking, before teardown.
    pub post_warp_delay_seconds: f64,
    /// Interval between the two clicks of a double-click.
    pub double_click_interval_seconds: f64,
    /// Click backend. Only `uinput` is implemented.
    pub click_backend: String,
    /// Number of refinement steps before a selection clicks.
    pub refinement_steps: u32,
    /// Session inactivity timeout; the grid resets after this.
    pub session_timeout_seconds: f64,
    /// (Reserved) chord commit timeout.
    pub chord_timeout_seconds: f64,
    /// Optional shell command run after a final selection commits
    /// (e.g. `hyprctl dispatch submap reset` to leave a Hyprland submap).
    pub on_commit_command: Option<String>,
    /// Optional shell command run when the session is cancelled.
    pub on_cancel_command: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            overlay_dismiss_delay_seconds: 0.12,
            pre_warp_delay_seconds: 0.05,
            post_warp_delay_seconds: 0.18,
            double_click_interval_seconds: 0.10,
            click_backend: "uinput".to_string(),
            refinement_steps: 3,
            session_timeout_seconds: 8.0,
            chord_timeout_seconds: 0.35,
            on_commit_command: None,
            on_cancel_command: None,
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("mousetrap").join("config.json")
}

impl Settings {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<PathBuf> {
        let path = config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self).unwrap() + "\n")?;
        Ok(path)
    }
}
