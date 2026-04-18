import cairo
import gi

from .config import APP_ID, CELL_PADDING, OVERLAY_ALPHA, ROWS, WINDOW_NAMESPACE
from .hyprctl import focused_monitor
from .session import SessionState
from .settings import Settings

gi.require_version('Gtk', '4.0')
gi.require_version('Gdk', '4.0')
gi.require_version('Gtk4LayerShell', '1.0')
from gi.repository import Gdk, GLib, Gtk, Gtk4LayerShell


class OverlayWindow(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.settings = Settings.load()
        loaded_state = SessionState.load()
        monitor = focused_monitor()
        self.monitor_bounds = (monitor['x'], monitor['y'], monitor['width'], monitor['height'])
        self.root_bounds = self.monitor_bounds
        self.current_state = loaded_state or SessionState.start(self.monitor_bounds)
        self.set_decorated(False)
        self.set_can_focus(False)
        self.set_focusable(False)
        self.set_resizable(False)
        self.set_default_size(self.root_bounds[2], self.root_bounds[3])
        self.set_opacity(1.0)

        Gtk4LayerShell.init_for_window(self)
        Gtk4LayerShell.set_namespace(self, WINDOW_NAMESPACE)
        Gtk4LayerShell.set_layer(self, Gtk4LayerShell.Layer.OVERLAY)
        monitor = self._gdk_monitor_for_bounds(self.monitor_bounds)
        if monitor is not None:
            Gtk4LayerShell.set_monitor(self, monitor)
        for edge in [Gtk4LayerShell.Edge.LEFT, Gtk4LayerShell.Edge.RIGHT, Gtk4LayerShell.Edge.TOP, Gtk4LayerShell.Edge.BOTTOM]:
            Gtk4LayerShell.set_anchor(self, edge, True)
        Gtk4LayerShell.set_keyboard_mode(self, Gtk4LayerShell.KeyboardMode.NONE)

        self.area = Gtk.DrawingArea()
        self.area.set_hexpand(True)
        self.area.set_vexpand(True)
        self.area.set_draw_func(self.draw)
        self.set_child(self.area)

        css = Gtk.CssProvider()
        css.load_from_data(b"window { background-color: transparent; } drawingarea { background-color: transparent; }")
        Gtk.StyleContext.add_provider_for_display(Gdk.Display.get_default(), css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

        self.connect('realize', self.on_realize)
        GLib.timeout_add(100, self.refresh_state)

    def _gdk_monitor_for_bounds(self, bounds: tuple[int, int, int, int]):
        display = Gdk.Display.get_default()
        if display is None:
            return None
        monitors = display.get_monitors()
        x, y, width, height = bounds
        for index in range(monitors.get_n_items()):
            monitor = monitors.get_item(index)
            geometry = monitor.get_geometry()
            if geometry.x == x and geometry.y == y and geometry.width == width and geometry.height == height:
                return monitor
        return display.get_monitor_at_surface(self.get_surface()) if self.get_surface() else None

    def on_realize(self, *_):
        self.present()

    def refresh_state(self):
        state = SessionState.load()
        if state is None:
            self.close()
            return False
        self.current_state = state
        if self.current_state.has_timed_out(self.settings.session_timeout_seconds):
            self.current_state.clear()
            self.close()
            return False
        self.area.queue_draw()
        return True

    def _local_rect(self, bounds: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
        x, y, w, h = bounds
        return x, y, w, h

    def draw(self, area, cr, width, height):
        cr.set_operator(cairo.OPERATOR_SOURCE)
        cr.set_source_rgba(0, 0, 0, 0)
        cr.paint()
        cr.set_operator(cairo.OPERATOR_OVER)

        cr.set_source_rgba(0, 0, 0, OVERLAY_ALPHA)
        cr.rectangle(0, 0, width, height)
        cr.fill()

        bx, by, bw, bh = self._local_rect(self.current_state.current_bounds)
        cr.set_source_rgba(1, 1, 1, 0.08)
        cr.rectangle(bx, by, bw, bh)
        cr.fill_preserve()
        cr.set_source_rgba(1, 1, 1, 0.45)
        cr.set_line_width(2)
        cr.stroke()

        row_h = bh / len(ROWS)
        for r, row in enumerate(ROWS):
            col_w = bw / len(row)
            for c, ch in enumerate(row):
                x = bx + c * col_w + CELL_PADDING
                y = by + r * row_h + CELL_PADDING
                w = col_w - 2 * CELL_PADDING
                h = row_h - 2 * CELL_PADDING
                cr.set_source_rgba(1, 1, 1, 0.12)
                cr.rectangle(x, y, w, h)
                cr.fill_preserve()
                cr.set_source_rgba(1, 1, 1, 0.28)
                cr.set_line_width(1)
                cr.stroke()
                cr.select_font_face('monospace', cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
                cr.set_font_size(max(10, min(w, h) * 0.33))
                cr.set_source_rgba(1, 1, 1, 0.95)
                ext = cr.text_extents(ch.upper())
                tx = x + (w - ext.width) / 2 - ext.x_bearing
                ty = y + (h - ext.height) / 2 - ext.y_bearing
                cr.move_to(tx, ty)
                cr.show_text(ch.upper())



class App(Gtk.Application):
    def __init__(self):
        super().__init__(application_id=APP_ID)
        self.connect('activate', self.on_activate)

    def on_activate(self, app):
        self.win = OverlayWindow(app)
        self.win.present()


def run():
    App().run(None)
