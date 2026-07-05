#!/usr/bin/env bash
set -euo pipefail

if ! command -v hyprctl >/dev/null 2>&1; then
  echo "hyprctl not found" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 not found" >&2
  exit 1
fi

mode="${1:-overlay}"

move_cursor() {
  local x="$1"
  local y="$2"
  echo "[poc] moving cursor to ${x} ${y} via hyprctl dispatch movecursor"
  hyprctl dispatch movecursor "$x $y"
}

active_window_center() {
  hyprctl -j activewindow | jq -r '
    if .address == "" or .mapped == false then empty else
      [(.at[0] + (.size[0] / 2 | floor)), (.at[1] + (.size[1] / 2 | floor))] | @tsv end'
}

monitor_center() {
  hyprctl -j monitors | jq -r '.[] | select(.focused == true) | [(.x + (.width/2|floor)), (.y + (.height/2|floor))] | @tsv' | head -n1
}

show_overlay() {
  python3 - <<'PY'
import gi
import signal

gi.require_version('Gtk', '4.0')
gi.require_version('Gtk4LayerShell', '1.0')
from gi.repository import Gtk, Gdk, Gtk4LayerShell

app = Gtk.Application(application_id='dev.mousetrap.hyprlandpoc')

class Win(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_decorated(False)
        self.set_can_focus(False)
        self.set_focusable(False)
        self.set_default_size(800, 600)

        Gtk4LayerShell.init_for_window(self)
        Gtk4LayerShell.set_layer(self, Gtk4LayerShell.Layer.OVERLAY)
        Gtk4LayerShell.set_anchor(self, Gtk4LayerShell.Edge.LEFT, True)
        Gtk4LayerShell.set_anchor(self, Gtk4LayerShell.Edge.RIGHT, True)
        Gtk4LayerShell.set_anchor(self, Gtk4LayerShell.Edge.TOP, True)
        Gtk4LayerShell.set_anchor(self, Gtk4LayerShell.Edge.BOTTOM, True)
        Gtk4LayerShell.set_keyboard_mode(self, Gtk4LayerShell.KeyboardMode.NONE)

        box = Gtk.Box()
        box.set_hexpand(True)
        box.set_vexpand(True)
        box.add_css_class('overlay-box')
        self.set_child(box)

        area = Gtk.DrawingArea()
        area.set_hexpand(True)
        area.set_vexpand(True)
        area.set_draw_func(self.draw)
        box.append(area)

    def draw(self, area, cr, width, height):
        cr.set_source_rgba(0, 0, 0, 0.18)
        cr.rectangle(0, 0, width, height)
        cr.fill()

        rows = ['1234567890', 'qwertyuiop', 'asdfghjkl;', 'zxcvbnm,./']
        row_h = height / len(rows)
        for r, row in enumerate(rows):
            col_w = width / len(row)
            for c, ch in enumerate(row):
                x = c * col_w + 6
                y = r * row_h + 6
                w = col_w - 12
                h = row_h - 12
                cr.set_source_rgba(1, 1, 1, 0.10)
                cr.rectangle(x, y, w, h)
                cr.fill_preserve()
                cr.set_source_rgba(1, 1, 1, 0.25)
                cr.stroke()
                cr.select_font_face('monospace', 0, 0)
                cr.set_font_size(min(w, h) * 0.33)
                cr.set_source_rgba(1, 1, 1, 0.95)
                ext = cr.text_extents(ch.upper())
                tx = x + (w - ext.width) / 2 - ext.x_bearing
                ty = y + (h - ext.height) / 2 - ext.y_bearing
                cr.move_to(tx, ty)
                cr.show_text(ch.upper())


def on_activate(app):
    win = Win(app)
    win.present()

app.connect('activate', on_activate)
signal.signal(signal.SIGINT, signal.SIG_DFL)
app.run(None)
PY
}

case "$mode" in
  center-active)
    coords="$(active_window_center || true)"
    if [[ -z "$coords" ]]; then
      coords="$(monitor_center)"
    fi
    read -r x y <<<"$coords"
    move_cursor "$x" "$y"
    ;;
  center-monitor)
    read -r x y <<<"$(monitor_center)"
    move_cursor "$x" "$y"
    ;;
  overlay)
    show_overlay
    ;;
  *)
    echo "usage: $0 [overlay|center-active|center-monitor]" >&2
    exit 1
    ;;
esac
