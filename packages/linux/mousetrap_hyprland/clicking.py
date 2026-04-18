import shutil
import subprocess


class ClickBackendUnavailable(RuntimeError):
    pass


def has_ydotool() -> bool:
    return shutil.which('ydotool') is not None


def left_click() -> None:
    if not has_ydotool():
        raise ClickBackendUnavailable('ydotool not found in PATH')
    subprocess.check_call(['ydotool', 'click', '0xC0'])
