# Mousetrap for Linux (Wayland)

Keyboard-driven mouse targeting for Wayland compositors, written in Rust.

A resident daemon draws a fullscreen grid on the focused monitor; you refine
it with keyboard keys and the final step moves the cursor and clicks. The
binary is self-contained (static musl build, embedded font and icon) and
works on any compositor with `wlr-layer-shell` — Hyprland, sway, river, and
most others.

## Architecture

| Concern | Mechanism |
|---------|-----------|
| Overlay window | `wlr-layer-shell` (overlay layer, non-focusable, follows the focused monitor) |
| Rendering | `wl_shm` + `tiny-skia` (pure Rust, no GTK/cairo) |
| Cursor movement & clicks | Virtual pointer via `/dev/uinput` (kernel input device) |
| Tray presence | StatusNotifierItem over DBus (planned) |
| Keyboard input | Input capture via libei (planned); optional user-defined compositor binds |

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
mousetrap cancel                # dismiss the grid
mousetrap doctor                # check the runtime environment
mousetrap init-config           # write ~/.config/mousetrap/config.json
```

Three refinements select the final target; adjacent keys pressed together
(e.g. `zx`, `aszx`) target midpoints and corners. When the final selection
commits, the overlay disappears, the cursor warps to the target, and a left
click is sent.

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
- [x] Cursor warp + click via uinput
- [x] Daemon + UNIX-socket IPC (CLI latency ~1ms)
- [ ] StatusNotifierItem tray presence
- [ ] libei input capture (bind-free keyboard input)
- [ ] Right-click / double-click / drag, free-mouse mode
- [ ] Settings UI parity with macOS
