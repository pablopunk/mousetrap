import argparse
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

from .actions import move_and_click
from .geometry import target_bounds
from .launcher import launch_overlay_detached
from .session import OverlaySession
from .timings import OVERLAY_DISMISS_DELAY_SECONDS

PID_FILE = Path('/tmp/mousetrap-hyprland-overlay.pid')


def _overlay_proc_alive() -> bool:
    if not PID_FILE.exists():
        return False
    try:
        pid = int(PID_FILE.read_text().strip())
        os.kill(pid, 0)
        return True
    except Exception:
        return False


def write_pid():
    PID_FILE.write_text(str(os.getpid()))


def clear_pid():
    try:
        if PID_FILE.exists() and PID_FILE.read_text().strip() == str(os.getpid()):
            PID_FILE.unlink()
    except Exception:
        pass


def stop_overlay():
    if not PID_FILE.exists():
        return
    try:
        pid = int(PID_FILE.read_text().strip())
        os.kill(pid, signal.SIGTERM)
    except Exception:
        pass
    try:
        PID_FILE.unlink()
    except Exception:
        pass


def set_submap(name: str):
    subprocess.run(['hyprctl', 'dispatch', 'submap', name], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def activate():
    if not _overlay_proc_alive():
        launch_overlay_detached()
        time.sleep(0.08)
    set_submap('mousetrap')


def cancel():
    set_submap('reset')
    stop_overlay()


def select(key: str):
    bounds = target_bounds()
    session = OverlaySession(bounds)
    selection = session.resolve_key(key)
    set_submap('reset')
    stop_overlay()
    time.sleep(OVERLAY_DISMISS_DELAY_SECONDS)
    if not selection:
        return 1
    move_and_click(selection['x'], selection['y'])
    return 0


def overlay():
    from .overlay import run
    write_pid()
    try:
        run()
    finally:
        clear_pid()


def main(argv=None):
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('activate')
    sub.add_parser('cancel')
    sel = sub.add_parser('select')
    sel.add_argument('key')
    sub.add_parser('overlay')
    args = parser.parse_args(argv)

    if args.command == 'activate':
        activate()
        return 0
    if args.command == 'cancel':
        cancel()
        return 0
    if args.command == 'select':
        return select(args.key)
    if args.command == 'overlay':
        overlay()
        return 0
    return 1


if __name__ == '__main__':
    raise SystemExit(main())
