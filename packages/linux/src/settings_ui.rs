//! Generic GTK/libadwaita settings window launched from the SNI tray item.

use adw::prelude::*;
use gtk::{Adjustment, SpinButtonUpdatePolicy};

use crate::config::Settings;
use crate::ipc::{self, Request};

const APP_ID: &str = "com.pablopunk.mousetrap.Settings";
const APP_VERSION: &str = include_str!("../../../VERSION");

pub fn run() -> i32 {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_window);
    // The outer Mousetrap CLI already consumed its `settings` subcommand.
    // Do not let GApplication reinterpret it as a file to open.
    let _ = app.run_with_args(&["mousetrap-settings"]);
    0
}

fn build_window(app: &adw::Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }

    let settings = Settings::load();
    let window = adw::PreferencesWindow::builder()
        .application(app)
        .default_width(520)
        .default_height(640)
        .title("Mousetrap")
        .icon_name("mousetrap")
        .search_enabled(false)
        .build();

    let settings_page = adw::PreferencesPage::builder()
        .title("Settings")
        .icon_name("preferences-system-symbolic")
        .build();

    let general = adw::PreferencesGroup::builder()
        .title("General")
        .description("Settings apply immediately to the running Mousetrap daemon.")
        .build();
    let launch_at_login = adw::SwitchRow::builder()
        .title("Launch at login")
        .subtitle("Start Mousetrap with your graphical session")
        .active(launch_at_login_enabled())
        .build();
    launch_at_login.connect_active_notify(|row| {
        set_setting(
            "launch-at-login",
            if row.is_active() { "on" } else { "off" },
        );
    });
    general.add(&launch_at_login);

    let free_mouse = adw::PreferencesGroup::builder()
        .title("Free Mouse")
        .description("Use an arrow key while the grid is visible to enter free-mouse mode.")
        .build();

    let travel = spin_row(
        "Mouse travel",
        "Logical pixels moved by each arrow press",
        settings.free_mouse_step,
        1.0,
        20.0,
        1.0,
        0,
    );
    travel.connect_value_notify(|row| {
        set_setting("free-mouse-step", &format!("{:.0}", row.value()));
    });
    free_mouse.add(&travel);

    let timeout = spin_row(
        "Global timeout",
        "Seconds without keyboard input before Mousetrap cancels",
        settings.session_timeout_seconds,
        3.0,
        60.0,
        1.0,
        0,
    );
    timeout.connect_value_notify(|row| {
        set_setting("session-timeout-seconds", &format!("{:.0}", row.value()));
    });
    free_mouse.add(&timeout);

    let double_click = spin_row(
        "Double-click interval",
        "Milliseconds between the two generated clicks",
        settings.double_click_interval_seconds * 1000.0,
        50.0,
        500.0,
        10.0,
        0,
    );
    double_click.connect_value_notify(|row| {
        set_setting(
            "double-click-interval-seconds",
            &format!("{:.2}", row.value() / 1000.0),
        );
    });
    free_mouse.add(&double_click);

    let controls = adw::PreferencesGroup::builder().title("Controls").build();
    for (title, subtitle) in [
        ("Arrow keys", "Move the cursor"),
        (
            "Enter or Space",
            "Click; press twice within 250 ms to double-click",
        ),
        ("Shift + Enter", "Right-click"),
        ("Shift + arrow keys", "Drag with the left button"),
        ("Escape", "Cancel and return to a safe state"),
    ] {
        controls.add(
            &adw::ActionRow::builder()
                .title(title)
                .subtitle(subtitle)
                .build(),
        );
    }

    settings_page.add(&general);
    settings_page.add(&free_mouse);
    settings_page.add(&controls);

    let about_page = adw::PreferencesPage::builder()
        .title("About")
        .icon_name("help-about-symbolic")
        .build();
    let about = adw::PreferencesGroup::builder().title("Mousetrap").build();
    for (title, subtitle) in [
        ("Version", APP_VERSION.trim()),
        ("Platform", "Linux / Wayland"),
        ("Project", "github.com/pablopunk/mousetrap"),
        ("Configuration", "~/.config/mousetrap/config.json"),
    ] {
        about.add(
            &adw::ActionRow::builder()
                .title(title)
                .subtitle(subtitle)
                .build(),
        );
    }
    about_page.add(&about);

    window.add(&settings_page);
    window.add(&about_page);
    window.set_visible_page(&settings_page);
    window.present();
}

fn spin_row(
    title: &str,
    subtitle: &str,
    value: f64,
    minimum: f64,
    maximum: f64,
    step: f64,
    digits: u32,
) -> adw::SpinRow {
    let adjustment = Adjustment::new(value, minimum, maximum, step, step * 5.0, 0.0);
    adw::SpinRow::builder()
        .title(title)
        .subtitle(subtitle)
        .adjustment(&adjustment)
        .digits(digits)
        .numeric(true)
        .snap_to_ticks(true)
        .update_policy(SpinButtonUpdatePolicy::IfValid)
        .build()
}

fn set_setting(key: &str, value: &str) {
    let request = Request::SetSetting {
        key: key.to_string(),
        value: value.to_string(),
    };
    match ipc::send(&request) {
        Ok(response) if response.ok => {}
        Ok(response) => eprintln!("mousetrap: cannot update {key}: {}", response.message),
        Err(error) => eprintln!("mousetrap: cannot update {key}: {error}"),
    }
}

fn launch_at_login_enabled() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "app-mousetrap.service"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::APP_VERSION;

    #[test]
    fn cargo_version_matches_product_version() {
        assert_eq!(env!("CARGO_PKG_VERSION"), APP_VERSION.trim());
    }
}
