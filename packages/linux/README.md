# Mousetrap for Linux (Hyprland)

Hyprland-specific implementation of Mousetrap.

## Structure

- `mousetrap_hyprland/core.py`: platform-agnostic grid lookup and cell-center math
- `mousetrap_hyprland/hyprctl.py`: Hyprland IPC/process adapter
- `mousetrap_hyprland/overlay.py`: GTK4 layer-shell overlay window
- `mousetrap_hyprland/config.py`: constants and layout settings
- `mousetrap_hyprland/cli.py`: main CLI entrypoint
- `mousetrap_hyprland/geometry.py`: active-window-first geometry resolution
- `mousetrap_hyprland/timings.py`: timing constants
- `mousetrap_hyprland/clicking.py`: click backend abstraction
- `mousetrap_hyprland/actions.py`: higher-level actions
- `activate.sh`, `select.sh`, `cancel.sh`: launcher scripts
- `mousetrap.conf`: example Hyprland config

## Dependencies

Python package deps:

```bash
cd packages/linux
python3 -m pip install --user -e .
```

Or from repo root:

```bash
make build-linux
```

System package deps currently expected on Arch/Hyprland:

```bash
sudo pacman -S gtk4 gtk4-layer-shell python python-gobject cairo jq hyprland
```

Optional for clicking:

```bash
sudo pacman -S ydotool
systemctl --user enable --now ydotool.service
```

## Run

```bash
# From repo root
bash packages/linux/activate.sh
bash packages/linux/select.sh a
bash packages/linux/cancel.sh
```

## Current status

- Transparent fullscreen layer-shell overlay works
- Cursor warp through `hyprctl dispatch movecursor` works
- Active-window-first targeting is the default geometry behavior
- Hyprland submap-driven key selection path is implemented
- Key selection now performs `move + left click` through the click backend abstraction
- `activate.sh` gives a simple launcher path you can bind in Hyprland

## Suggested Hyprland bind

Add to your Hyprland config:

```ini
bind = SUPER, SPACE, exec, /path/to/mousetrap/packages/linux/activate.sh
```

Or source the example config:

```ini
source = /path/to/mousetrap/packages/linux/mousetrap.conf
```

## Still missing

- Real global activation hotkey outside the current Hyprland bind/submap model
- Multi-step refinement and richer mouse actions
- Packaging/polish for a stable end-user Linux release
- Multi-step refinement flow
- Synthetic click/right-click/double-click/drag
- Guaranteed non-activating behavior parity with macOS
- Multi-monitor focused-window targeting parity
