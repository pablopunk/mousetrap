import time

from .clicking import ClickBackendUnavailable, left_click
from .hyprctl import move_cursor
from .timings import POST_WARP_DELAY_SECONDS, PRE_WARP_DELAY_SECONDS


def move_and_click(x: int, y: int) -> str:
    time.sleep(PRE_WARP_DELAY_SECONDS)
    move_cursor(x, y)
    time.sleep(POST_WARP_DELAY_SECONDS)
    try:
        left_click()
        return 'clicked'
    except ClickBackendUnavailable:
        return 'moved-no-click-backend'
