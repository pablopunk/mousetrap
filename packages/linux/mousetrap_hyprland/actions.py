import time

from .clicking import (
    ClickBackendUnavailable,
    begin_left_drag,
    double_click,
    end_left_drag,
    left_click,
    right_click,
)
from .hyprctl import move_cursor
from .settings import Settings


def _move_then(x: int, y: int, action) -> str:
    settings = Settings.load()
    time.sleep(settings.pre_warp_delay_seconds)
    move_cursor(x, y)
    time.sleep(settings.post_warp_delay_seconds)
    try:
        action()
        return 'ok'
    except ClickBackendUnavailable:
        return 'moved-no-click-backend'


def move_and_click(x: int, y: int) -> str:
    result = _move_then(x, y, left_click)
    return 'clicked' if result == 'ok' else result


def move_and_right_click(x: int, y: int) -> str:
    result = _move_then(x, y, right_click)
    return 'right-clicked' if result == 'ok' else result


def move_and_double_click(x: int, y: int) -> str:
    result = _move_then(x, y, double_click)
    return 'double-clicked' if result == 'ok' else result


def move_and_begin_drag(x: int, y: int) -> str:
    result = _move_then(x, y, begin_left_drag)
    return 'drag-started' if result == 'ok' else result


def end_drag_at(x: int, y: int) -> str:
    result = _move_then(x, y, end_left_drag)
    return 'drag-ended' if result == 'ok' else result
