# Mousetrap for Linux (Hyprland)

Hyprland-specific implementation of Mousetrap.

## Structure

- `mousetrap_hyprland/cli.py`: canonical CLI entrypoint
- `mousetrap_hyprland/settings.py`: user config loading/saving
- `mousetrap_hyprland/diagnostics.py`: dependency checks
- `mousetrap_hyprland/binds.py`: Hyprland dynamic bind/submap management
- `mousetrap_hyprland/core.py`: platform-agnostic grid lookup and cell-center math
- `mousetrap_hyprland/hyprctl.py`: Hyprland IPC/process adapter
- `mousetrap_hyprland/overlay.py`: GTK4 layer-shell overlay window
- `mousetrap_hyprland/geometry.py`: active-window-first geometry resolution
- `mousetrap_hyprland/clicking.py`: click backend abstraction
- `mousetrap_hyprland/actions.py`: higher-level actions
- `activate.sh`, `select.sh`, `cancel.sh`: tiny wrappers around the CLI
- `mousetrap.conf`: example Hyprland config

## Install

```bash
make build-linux
make doctor-linux
make config-linux
```

This installs the package in editable mode, checks runtime dependencies, and writes the default config file to:

```bash
~/.config/mousetrap/hyprland.json
```

System package deps currently expected on Arch/Hyprland:

```bash
sudo pacman -S gtk4 gtk4-layer-shell python python-gobject cairo jq hyprland ydotool
systemctl --user enable --now ydotool.service
```

## Run

```bash
bash packages/linux/activate.sh
bash packages/linux/select.sh a
bash packages/linux/cancel.sh
```

Or, after install:

```bash
mousetrap-hyprland activate
mousetrap-hyprland doctor
mousetrap-hyprland print-config
```

## Configuration

Current config fields:

- `activation_mode`: `dynamic` or `static`
- `overlay_dismiss_delay_seconds`
- `pre_warp_delay_seconds`
- `post_warp_delay_seconds`
- `click_backend`
- `bundle_ydotool_in_release`

`dynamic` activation installs temporary Hyprland binds at runtime.
`static` assumes you sourced `packages/linux/mousetrap.conf` yourself.

## Suggested Hyprland bind

Add to your Hyprland config:

```ini
bind = SUPER, SPACE, exec, /path/to/mousetrap/packages/linux/activate.sh
```

Or source the example config:

```ini
source = /path/to/mousetrap/packages/linux/mousetrap.conf
```

## Linux release bundling

For Linux we should not depend on PyPI-only packaging as the whole release story; we also need compositor/system dependencies. For now:

```bash
make package-linux
```

This creates a tarball in `dist/linux/` and, when available, bundles local copies of `ydotool`, `ydotoold`, and the user service file.

Important: bundling the binaries helps distribution, but it does **not** magically remove Wayland/uinput/session requirements. A polished Linux release will likely need either:

- a distro package, or
- an AppImage plus a managed user-service setup story.

## Current status

- Transparent fullscreen layer-shell overlay works
- Cursor warp through `hyprctl dispatch movecursor` works
- Active-window-first targeting is the default geometry behavior
- Hyprland submap-driven key selection path is implemented
- Key selection performs `move + left click`
- Runtime dependency diagnostics are available
- Settings are now configurable via a user config file

## Still missing

- Three-step refinement flow
- Chord targeting
- Right-click / double-click / drag
- Free-mouse mode
- Timeout / safe reset polish
- Better Linux packaging and publishing
- Guaranteed non-activating behavior parity with macOS
- Multi-monitor focused-window targeting parity
