import os
import subprocess
import sys
from pathlib import Path

from .config import APP_ID


def project_pythonpath() -> str:
    root = Path(__file__).resolve().parent.parent
    existing = os.environ.get('PYTHONPATH', '')
    return f"{root}:{existing}" if existing else str(root)


def launch_overlay_detached():
    env = os.environ.copy()
    env['PYTHONPATH'] = project_pythonpath()
    subprocess.Popen(
        [sys.executable, '-m', 'mousetrap_hyprland.cli', 'overlay'],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )


def main():
    launch_overlay_detached()


if __name__ == '__main__':
    main()
