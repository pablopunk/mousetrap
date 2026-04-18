import shutil
import subprocess

from .settings import Settings


class ClickBackendUnavailable(RuntimeError):
    pass


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


def left_click() -> None:
    settings = Settings.load()
    if settings.click_backend != 'ydotool':
        raise ClickBackendUnavailable(f'unsupported click backend: {settings.click_backend}')
    if not has_ydotool():
        raise ClickBackendUnavailable('ydotool not found in PATH')
    if not ydotool_service_active():
        raise ClickBackendUnavailable('ydotool.service is not active; run: systemctl --user enable --now ydotool.service')
    subprocess.check_call(['ydotool', 'click', '0xC0'])
