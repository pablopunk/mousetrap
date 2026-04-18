import json
import subprocess


def _run(*args: str) -> str:
    return subprocess.check_output(list(args), text=True)


def hypr_json(command: str):
    return json.loads(_run('hyprctl', '-j', command))


def focused_monitor():
    monitors = hypr_json('monitors')
    for mon in monitors:
        if mon.get('focused'):
            return mon
    return monitors[0]


def active_window():
    return hypr_json('activewindow')


def active_window_bounds():
    win = active_window()
    if not win.get('address') or not win.get('mapped'):
        return None
    return (int(win['at'][0]), int(win['at'][1]), int(win['size'][0]), int(win['size'][1]))


def move_cursor(x: int, y: int):
    subprocess.check_call(['hyprctl', 'dispatch', 'movecursor', f'{int(x)} {int(y)}'])
