from pathlib import Path

ROWS = ['1234567890', 'qwertyuiop', 'asdfghjkl;', 'zxcvbnm,./']
MAX_COLUMNS = max(len(row) for row in ROWS)
CELL_PADDING = 1
OVERLAY_ALPHA = 0.18
WINDOW_NAMESPACE = 'mousetrap'
APP_ID = 'dev.mousetrap.hyprland.overlay'
PACKAGE_ROOT = Path(__file__).resolve().parents[1]
