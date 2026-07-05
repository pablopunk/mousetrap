from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass


@dataclass(slots=True)
class CheckResult:
    name: str
    ok: bool
    detail: str


def _cmd_exists(name: str) -> bool:
    return shutil.which(name) is not None


def _user_service_active(name: str) -> bool:
    result = subprocess.run(
        ['systemctl', '--user', 'is-active', name],
        text=True,
        capture_output=True,
        check=False,
    )
    return result.returncode == 0 and result.stdout.strip() == 'active'


def run_checks() -> list[CheckResult]:
    checks = [
        CheckResult('hyprctl', _cmd_exists('hyprctl'), 'required for Hyprland IPC'),
        CheckResult('python3', _cmd_exists('python3'), 'required runtime'),
        CheckResult('ydotool', _cmd_exists('ydotool'), 'required for click injection'),
        CheckResult('ydotool.service', _user_service_active('ydotool.service'), 'required for ydotool clicks'),
    ]
    return checks


def require_runtime() -> None:
    missing = []
    for check in run_checks():
        if check.name in {'hyprctl', 'python3'} and not check.ok:
            missing.append(check.name)
    if missing:
        raise RuntimeError('Missing required runtime dependencies: ' + ', '.join(missing))
