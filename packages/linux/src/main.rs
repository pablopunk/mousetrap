//! Mousetrap — keyboard-driven mouse targeting for Wayland.
//!
//! Commands:
//!   mousetrap activate            Start/reset the grid session
//!   mousetrap cancel              End the grid session
//!   mousetrap key-down <key>      Grid/free-mouse key pressed
//!   mousetrap key-up <key>        Grid key released
//!   mousetrap doctor              Check runtime environment
//!   mousetrap init-config         Write default config
//!   mousetrap print-config        Print current config as JSON
//!   mousetrap settings            Open the settings window
//!   mousetrap settings set <key> <value>
//!   mousetrap daemon              Run the resident daemon (internal)

mod config;
mod cursor;
mod daemon;
mod doctor;
mod geometry;
mod input;
mod ipc;
mod keys;
mod mouse;
mod overlay;
mod settings_ui;
mod shortcuts;
mod state;
mod tray;

fn usage() -> ! {
    eprintln!(
        "usage: mousetrap <activate|cancel|key-down <key>|key-up <key>|doctor|init-config|print-config|settings [set <key> <value>]>"
    );
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = match args.first().map(String::as_str) {
        Some("activate") => daemon::client_activate(),
        Some("cancel") => daemon::client_cancel(),
        Some("key-down") => {
            let key = args.get(1).cloned().unwrap_or_default();
            daemon::client_key("down", &key)
        }
        Some("key-up") => {
            let key = args.get(1).cloned().unwrap_or_default();
            daemon::client_key("up", &key)
        }
        Some("doctor") => doctor::run(),
        Some("init-config") => match config::Settings::default().save() {
            Ok(path) => {
                println!("{}", path.display());
                0
            }
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        },
        Some("print-config") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&config::Settings::load()).unwrap()
            );
            0
        }
        Some("settings") => {
            if args.len() == 1 {
                settings_ui::run()
            } else {
                if args.get(1).map(String::as_str) != Some("set") {
                    usage();
                }
                let key = args.get(2).cloned().unwrap_or_default();
                let value = args.get(3).cloned().unwrap_or_default();
                daemon::client_set_setting(&key, &value)
            }
        }
        Some("daemon") => daemon::run(),
        _ => usage(),
    };
    std::process::exit(exit_code);
}
