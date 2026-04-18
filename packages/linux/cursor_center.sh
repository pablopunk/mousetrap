#!/usr/bin/env bash
set -euo pipefail
PYTHONPATH="$(dirname "$0")${PYTHONPATH:+:$PYTHONPATH}" python3 - <<'PY'
from mousetrap_hyprland.hyprctl import active_window, focused_monitor, move_cursor

win = active_window()
if win.get('address') and win.get('mapped'):
    x = int(win['at'][0] + win['size'][0] / 2)
    y = int(win['at'][1] + win['size'][1] / 2)
else:
    mon = focused_monitor()
    x = int(mon['x'] + mon['width'] / 2)
    y = int(mon['y'] + mon['height'] / 2)
move_cursor(x, y)
PY
