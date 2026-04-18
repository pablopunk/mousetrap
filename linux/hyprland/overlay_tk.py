#!/usr/bin/env python3
import json
import math
import subprocess
import tkinter as tk

ROWS = ['1234567890', 'qwertyuiop', 'asdfghjkl;', 'zxcvbnm,./']
PADDING = 6


def hypr_json(cmd):
    out = subprocess.check_output(['hyprctl', '-j', cmd], text=True)
    return json.loads(out)


def focused_monitor():
    monitors = hypr_json('monitors')
    for mon in monitors:
        if mon.get('focused'):
            return mon
    return monitors[0]


def move_cursor(x, y):
    subprocess.check_call(['hyprctl', 'dispatch', 'movecursor', f'{int(x)} {int(y)}'])


class Overlay:
    def __init__(self):
        mon = focused_monitor()
        self.mx = mon['x']
        self.my = mon['y']
        self.mw = mon['width']
        self.mh = mon['height']

        self.root = tk.Tk()
        self.root.title('Mousetrap Hyprland POC')
        self.root.geometry(f'{self.mw}x{self.mh}+{self.mx}+{self.my}')
        self.root.overrideredirect(True)
        self.root.attributes('-topmost', True)
        try:
            self.root.attributes('-alpha', 0.82)
        except Exception:
            pass

        self.canvas = tk.Canvas(self.root, width=self.mw, height=self.mh, highlightthickness=0, bg='black')
        self.canvas.pack(fill='both', expand=True)
        self.canvas.focus_force()
        self.root.bind('<Escape>', lambda e: self.root.destroy())
        self.root.bind('<Key>', self.on_key)
        self.draw()

    def draw(self):
        self.canvas.delete('all')
        self.canvas.create_rectangle(0, 0, self.mw, self.mh, fill='black', outline='', stipple='gray50')
        row_h = self.mh / len(ROWS)
        for r, row in enumerate(ROWS):
            col_w = self.mw / len(row)
            for c, ch in enumerate(row):
                x1 = c * col_w + PADDING
                y1 = r * row_h + PADDING
                x2 = (c + 1) * col_w - PADDING
                y2 = (r + 1) * row_h - PADDING
                self.canvas.create_rectangle(x1, y1, x2, y2, outline='#cccccc', width=1, fill='')
                font_size = max(14, int(min(col_w, row_h) * 0.28))
                self.canvas.create_text((x1 + x2) / 2, (y1 + y2) / 2, text=ch.upper(), fill='white', font=('Monospace', font_size, 'bold'))

    def on_key(self, event):
        ch = (event.char or '').lower()
        for r, row in enumerate(ROWS):
            if ch in row:
                c = row.index(ch)
                row_h = self.mh / len(ROWS)
                col_w = self.mw / len(row)
                x = self.mx + (c + 0.5) * col_w
                y = self.my + (r + 0.5) * row_h
                move_cursor(x, y)
                self.root.destroy()
                return

    def run(self):
        self.root.mainloop()


if __name__ == '__main__':
    Overlay().run()
