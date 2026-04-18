import shutil
import subprocess
import time

from .settings import Settings


class ClickBackendUnavailable(RuntimeError):
    pass


YDTOOL_LEFT = '0xC0'
YDTOOL_RIGHT = '0xC1'


def has_ydotool() -> bool:
    return shutil.which('ydotool') is not None


def ydotool_service_active() -> bool:
    result = subprocess.run(
        ['systemctl', '--user', 'is-active', 'ydotool.service'],
        text=True,
        capture_output=True,
        check=False,
    )
    return result.returncode == 0 and result.stdout.strip() == 'active'


def _ensure_backend() -> None:
    settings = Settings.load()
    if settings.click_backend != 'ydotool':
        raise ClickBackendUnavailable(f'unsupported click backend: {settings.click_backend}')
    if not has_ydotool():
        raise ClickBackendUnavailable('ydotool not found in PATH')
    if not ydotool_service_active():
        raise ClickBackendUnavailable('ydotool.service is not active; run: systemctl --user enable --now ydotool.service')


def _run_ydotool(*args: str) -> None:
    _ensure_backend()
    subprocess.check_call(['ydotool', *args])


def left_click() -> None:
    _run_ydotool('click', YDTOOL_LEFT)


def right_click() -> None:
    _run_ydotool('click', YDTOOL_RIGHT)


def double_click() -> None:
    settings = Settings.load()
    left_click()
    time.sleep(settings.double_click_interval_seconds)
    left_click()


def begin_left_drag() -> None:
    _run_ydotool('mousedown', YDTOOL_LEFT)


def end_left_drag() -> None:
    _run_ydotool('mouseup', YDTOOL_LEFT)
