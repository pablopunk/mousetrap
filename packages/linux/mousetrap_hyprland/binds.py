from __future__ import annotations

import subprocess
from pathlib import Path

KEYS = ['1','2','3','4','5','6','7','8','9','0','q','w','e','r','t','y','u','i','o','p','a','s','d','f','g','h','j','k','l','semicolon','z','x','c','v','b','n','m','comma','period','slash']
SPECIAL_ARGS = {'semicolon': ';', 'comma': ',', 'period': '.', 'slash': '/'}


def _run_keyword(*parts: str) -> None:
    subprocess.run(['hyprctl', 'keyword', *parts], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def _run_dispatch(*parts: str) -> None:
    subprocess.run(['hyprctl', 'dispatch', *parts], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def clear_dynamic_binds() -> None:
    _run_keyword('unbind', ', escape')
    for key in KEYS:
        _run_keyword('unbind', f', {key}')
        _run_keyword('unbindr', f', {key}')


def install_dynamic_binds(package_root: Path) -> None:
    clear_dynamic_binds()
    _run_keyword('submap', 'mousetrap')
    _run_keyword('binde', f', escape, exec, {package_root / "cancel.sh"}')
    for key in KEYS:
        arg = SPECIAL_ARGS.get(key, key)
        _run_keyword('bind', f', {key}, exec, {package_root / "key_down.sh"} {arg}')
        _run_keyword('bindr', f', {key}, exec, {package_root / "key_up.sh"} {arg}')
    _run_dispatch('submap', 'mousetrap')


def reset_submap() -> None:
    _run_dispatch('submap', 'reset')
