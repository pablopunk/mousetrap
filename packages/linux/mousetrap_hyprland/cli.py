import argparse
import json
import os
import signal
import subprocess
import sys
import time
from dataclasses import asdict
from pathlib import Path

from .actions import move_and_click
from .binds import clear_dynamic_binds, install_dynamic_binds, reset_submap
from .config import PACKAGE_ROOT
from .diagnostics import run_checks
from .hyprctl import focused_monitor
from .launcher import launch_overlay_detached
from .session import OverlaySession, SessionState
from .settings import Settings

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


def _fresh_session(settings: Settings) -> SessionState:
    monitor = focused_monitor()
    session = SessionState.start((monitor['x'], monitor['y'], monitor['width'], monitor['height']))
    session.max_steps = settings.refinement_steps
    session.save()
    return session


def _spawn_pending_commit(delay_seconds: float) -> None:
    subprocess.Popen(
        [sys.executable, '-m', 'mousetrap_hyprland.cli', 'maybe-commit-pending'],
        env=os.environ.copy(),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )


def _commit_selection(settings: Settings, selection) -> int:
    if selection is None:
        return 1
    if not selection.final:
        return 0
    reset_submap()
    stop_overlay()
    state = SessionState.load()
    if state:
        state.clear()
    time.sleep(settings.overlay_dismiss_delay_seconds)
    move_and_click(*selection.point)
    return 0


def activate():
    settings = Settings.load()
    session = SessionState.load()
    if session is None or session.has_timed_out(settings.session_timeout_seconds):
        _fresh_session(settings)
    if not _overlay_proc_alive():
        launch_overlay_detached()
        time.sleep(0.08)
    if settings.activation_mode == 'dynamic':
        install_dynamic_binds(PACKAGE_ROOT)
    else:
        reset_submap()


def cancel():
    reset_submap()
    clear_dynamic_binds()
    stop_overlay()
    state = SessionState.load()
    if state:
        state.clear()


def select(key: str):
    settings = Settings.load()
    state = SessionState.load()
    if state is None or state.has_timed_out(settings.session_timeout_seconds):
        state = _fresh_session(settings)
    session = OverlaySession(state)
    queue_result = session.queue_key(key)
    if queue_result == 'invalid':
        return 1
    selection = None
    if len(state.pending_keys) >= 2:
        selection = session.commit_pending()
    state.save()
    if selection is not None:
        return _commit_selection(settings, selection)
    _spawn_pending_commit(settings.chord_timeout_seconds)
    return 0


def maybe_commit_pending():
    settings = Settings.load()
    time.sleep(settings.chord_timeout_seconds)
    state = SessionState.load()
    if state is None or state.has_timed_out(settings.session_timeout_seconds) or not state.pending_keys:
        return 0
    if state.pending_since and (time.time() - state.pending_since) < settings.chord_timeout_seconds:
        return 0
    session = OverlaySession(state)
    selection = session.commit_pending()
    state.save()
    return _commit_selection(settings, selection)


def overlay():
    from .overlay import run
    write_pid()
    try:
        run()
    finally:
        clear_pid()


def doctor() -> int:
    failures = 0
    for check in run_checks():
        status = 'ok' if check.ok else 'missing'
        print(f'[{status}] {check.name}: {check.detail}')
        if not check.ok and check.name in {'hyprctl', 'python3'}:
            failures += 1
    return 1 if failures else 0


def init_config() -> int:
    settings = Settings.load()
    path = settings.save()
    print(path)
    return 0


def print_config() -> int:
    settings = Settings.load()
    print(json.dumps(asdict(settings), indent=2))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('activate')
    sub.add_parser('cancel')
    sel = sub.add_parser('select')
    sel.add_argument('key')
    sub.add_parser('maybe-commit-pending')
    sub.add_parser('overlay')
    sub.add_parser('doctor')
    sub.add_parser('init-config')
    sub.add_parser('print-config')
    args = parser.parse_args(argv)

    if args.command == 'activate':
        activate()
        return 0
    if args.command == 'cancel':
        cancel()
        return 0
    if args.command == 'select':
        return select(args.key)
    if args.command == 'maybe-commit-pending':
        return maybe_commit_pending()
    if args.command == 'overlay':
        overlay()
        return 0
    if args.command == 'doctor':
        return doctor()
    if args.command == 'init-config':
        return init_config()
    if args.command == 'print-config':
        return print_config()
    return 1


if __name__ == '__main__':
    raise SystemExit(main())
