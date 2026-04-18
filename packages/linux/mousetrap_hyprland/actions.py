import time

from .clicking import ClickBackendUnavailable, left_click
from .hyprctl import move_cursor
from .settings import Settings


def move_and_click(x: int, y: int) -> str:
    settings = Settings.load()
    time.sleep(settings.pre_warp_delay_seconds)
    move_cursor(x, y)
    time.sleep(settings.post_warp_delay_seconds)
    try:
        left_click()
        return 'clicked'
    except ClickBackendUnavailable:
        return 'moved-no-click-backend'
