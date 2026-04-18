import cairo
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Gdk', '4.0')
gi.require_version('Gtk4LayerShell', '1.0')
from gi.repository import Gtk, Gdk, Gtk4LayerShell

from .config import APP_ID, CELL_PADDING, OVERLAY_ALPHA, ROWS, WINDOW_NAMESPACE
from .hyprctl import focused_monitor
from .session import OverlaySession


class OverlayWindow(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.monitor = focused_monitor()
        self.session = OverlaySession((self.monitor['x'], self.monitor['y'], self.monitor['width'], self.monitor['height']))
        self.set_decorated(False)
        self.set_can_focus(True)
        self.set_focusable(True)
        self.set_resizable(False)
        self.set_default_size(self.monitor['width'], self.monitor['height'])
        self.set_opacity(1.0)

        Gtk4LayerShell.init_for_window(self)
        Gtk4LayerShell.set_namespace(self, WINDOW_NAMESPACE)
        Gtk4LayerShell.set_layer(self, Gtk4LayerShell.Layer.OVERLAY)
        for edge in [Gtk4LayerShell.Edge.LEFT, Gtk4LayerShell.Edge.RIGHT, Gtk4LayerShell.Edge.TOP, Gtk4LayerShell.Edge.BOTTOM]:
            Gtk4LayerShell.set_anchor(self, edge, True)
        Gtk4LayerShell.set_keyboard_mode(self, Gtk4LayerShell.KeyboardMode.EXCLUSIVE)

        area = Gtk.DrawingArea()
        area.set_hexpand(True)
        area.set_vexpand(True)
        area.set_draw_func(self.draw)
        self.set_child(area)

        css = Gtk.CssProvider()
        css.load_from_data(b"window { background-color: transparent; } drawingarea { background-color: transparent; }")
        Gtk.StyleContext.add_provider_for_display(Gdk.Display.get_default(), css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

        self.connect('map', self.on_map)
        self.connect('realize', self.on_realize)

    def on_realize(self, *_):
        self.present()

    def on_map(self, *_):
        pass

    def draw(self, area, cr, width, height):
        cr.set_operator(cairo.OPERATOR_SOURCE)
        cr.set_source_rgba(0, 0, 0, 0)
        cr.paint()
        cr.set_operator(cairo.OPERATOR_OVER)

        cr.set_source_rgba(0, 0, 0, OVERLAY_ALPHA)
        cr.rectangle(0, 0, width, height)
        cr.fill()

        row_h = height / len(ROWS)
        for r, row in enumerate(ROWS):
            col_w = width / len(row)
            for c, ch in enumerate(row):
                x = c * col_w + CELL_PADDING
                y = r * row_h + CELL_PADDING
                w = col_w - 2 * CELL_PADDING
                h = row_h - 2 * CELL_PADDING
                cr.set_source_rgba(1, 1, 1, 0.10)
                cr.rectangle(x, y, w, h)
                cr.fill_preserve()
                cr.set_source_rgba(1, 1, 1, 0.25)
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
