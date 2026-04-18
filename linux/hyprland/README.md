# Hyprland POC

This folder now has a small refactored Hyprland-specific prototype with a clearer production shape.

## Structure

- `mousetrap_hyprland/core.py`: platform-agnostic grid lookup and cell-center math
- `mousetrap_hyprland/hyprctl.py`: Hyprland IPC/process adapter
- `mousetrap_hyprland/overlay.py`: GTK4 layer-shell overlay window and key handling
- `mousetrap_hyprland/config.py`: constants and layout settings
- `overlay.py`: thin launcher
- `cursor_center.sh`: thin cursor-centering helper

## Dependencies

Python package deps:

```bash
cd linux/hyprland
python3 -m pip install --user -e .
```

System package deps currently expected on Arch/Hyprland:

```bash
sudo pacman -S gtk4 gtk4-layer-shell python python-gobject cairo jq hyprland
```

Optional for future synthetic clicking:

```bash
sudo pacman -S ydotool
```

## Run

```bash
python3 linux/hyprland/overlay.py
bash linux/hyprland/cursor_center.sh
bash linux/hyprland/activate.sh
bash linux/hyprland/select.sh a
bash linux/hyprland/cancel.sh
```

The launcher scripts currently work either with the editable install above or by setting `PYTHONPATH` as they already do.

## Current status

- Transparent fullscreen layer-shell overlay works
- Cursor warp through `hyprctl dispatch movecursor` works
- Active-window-first targeting is the default geometry behavior
- Hyprland submap-driven key selection path is implemented
- Key selection now performs `move + left click` through the click backend abstraction
- `activate.sh` gives a simple launcher path you can bind in Hyprland

## Suggested Hyprland bind

You can still `source = /home/pol/src/mousetrap/linux/hyprland/mousetrap.conf`, but for quick experiments `activate.sh` now also tries to register the temporary submap dynamically with `hyprctl` at runtime.

## Still missing

- Real global activation hotkey outside the current Hyprland bind/submap model
- Multi-step refinement and richer mouse actions
- Packaging/polish for a stable end-user Linux release
- Multi-step refinement flow
- Synthetic click/right-click/double-click/drag
- Guaranteed non-activating behavior parity with macOS
- Multi-monitor focused-window targeting parity
