# Mousetrap for Linux (Wayland)

Keyboard-driven mouse targeting for Wayland compositors, written in Rust.

A resident daemon draws a fullscreen grid on the focused monitor; you refine
it with keyboard keys and the final step moves the cursor and clicks. The
daemon uses an embedded font and icon and works on any compositor with
`wlr-layer-shell` — Hyprland, sway, river, and most others. Its settings
window uses GTK 4 and libadwaita, with no compositor or desktop-shell coupling.

## Architecture

| Concern | Mechanism |
|---------|-----------|
| Overlay window | `wlr-layer-shell` (overlay layer, non-focusable, follows the focused monitor) |
| Rendering | `wl_shm` + hand-rolled rect/text rasterization (embedded font, no GTK/cairo) |
| Cursor movement & clicks | Virtual pointer via `/dev/uinput` (kernel input device) |
| Tray presence | StatusNotifierItem + DBusMenu over DBus (Quickshell, any SNI host) |
| Keyboard input | evdev + `EVIOCGRAB` while the grid is active — no compositor keybinds |
| Settings window | GTK 4 + libadwaita, opened from the SNI tray item |

## Installing (standard Linux app install)

```bash
# 1. Binary + launcher entry (user-local; no root needed)
install -Dm755 target/release/mousetrap ~/.local/bin/mousetrap
install -Dm644 assets/AppIcon.png ~/.local/share/icons/mousetrap.png
install -Dm644 packaging/mousetrap.desktop ~/.local/share/applications/mousetrap.desktop

# 2. systemd user unit: autostarts on login, restarts on crash.
#    The `app-` prefix is the systemd desktop convention — it is what lets
#    xdg-desktop-portal resolve this process to the mousetrap.desktop app id.
install -Dm644 packaging/app-mousetrap.service ~/.config/systemd/user/app-mousetrap.service
systemctl --user daemon-reload
systemctl --user enable --now app-mousetrap
```

`make install-linux` performs all of the steps above. Building and running the
settings window requires GTK 4.6 and libadwaita 1.4 or newer.

(For a system-wide install, use `/usr/local/bin` and `/usr/local/share/...`
with the same files.)

The daemon appears in the tray, survives logins via systemd, and the CLI
revives it through systemd if you quit it from the tray menu.

## Build

```bash
cargo build --release          # from this directory
# or from the repo root:
make build-linux
```

The result is `target/release/mousetrap` — a single binary.

## Usage

```bash
mousetrap daemon                # start the resident daemon
mousetrap activate              # show the grid
mousetrap key-down a            # grid key press
mousetrap key-up a              # grid key release (commits on release)
mousetrap key-down right        # enter/move free-mouse mode
mousetrap key-down shift+enter  # right-click in free-mouse mode
mousetrap cancel                # dismiss the grid
mousetrap doctor                # check the runtime environment
mousetrap init-config           # write ~/.config/mousetrap/config.json
mousetrap settings              # open the settings window
```

Three refinements select the final target; adjacent keys pressed together
(e.g. `zx`, `aszx`) target midpoints and corners. When the final selection
commits, the overlay disappears, the cursor warps to the target, and a left
click is sent.

At any point, an arrow key switches to free-mouse mode. Plain arrows move the
cursor by the configured travel step; `Enter` or `Space` clicks, a second
press within 250 ms double-clicks, `Shift+Enter` right-clicks, and
`Shift+Arrow` drags with the left button. `Escape`, a physical mouse move, or
the global timeout safely ends the mode. Left-clicking the tray icon, or using
its **Open Settings** menu item, opens the settings window with live controls
for travel step, timeout, double-click speed, launch at login, and keyboard
help.

### /dev/uinput permission

Click injection uses a virtual pointer device, which needs write access to
`/dev/uinput`. This is Mousetrap's equivalent of the macOS Accessibility
permission. Either add your user to the `input` group, or install a udev
rule, e.g.:

```
KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", MODE="0660"
```

### Optional: compositor keybindings

Mousetrap installs no keybindings — you choose how to trigger it. For
example, a Hyprland bind calling `mousetrap activate`, with grid keys bound
to `mousetrap key-down <key>` / `mousetrap key-up <key>`. See the tray and
input-capture work below for the bind-free flow.

## Status

- [x] Layer-shell overlay on the focused monitor (multi-monitor safe)
- [x] Three-step refinement, chord targeting (pairs + 2x2 quads)
- [x] Cursor warp + click via uinput (udev/input-group permission needed)
- [x] Tray presence via StatusNotifierItem (SNI) with DBusMenu
- [x] Keyboard capture via evdev + EVIOCGRAB (no compositor binds needed)
- [x] Daemon + UNIX-socket IPC (CLI latency ~1ms)
- [x] Safety nets: watchdog thread, grab self-release, graceful tray Quit
- [x] Free-mouse movement, single/double/right click, left drag, and safety reset
- [x] Generic GTK settings window for free-mouse travel, timeout, click speed, launch at login, and About

## Development

```bash
cargo build                     # debug
cargo test                      # unit tests (grid math, key mapping, uinput)
cargo build --release           # release binary

# Run the daemon (detached; it registers a tray icon):
setsid ./target/debug/mousetrap daemon > /tmp/mousetrap.log 2>&1 < /dev/null &

# Synthetic testing without a physical keyboard (clicks disabled):
python3 - <<'PY'
import json
from pathlib import Path
p = Path.home() / '.config/mousetrap/config.json'
cfg = json.loads(p.read_text())
cfg['click_backend'] = 'none'
p.write_text(json.dumps(cfg, indent=2) + '\n')
PY
mousetrap activate && mousetrap key-down a && mousetrap key-up a
```
