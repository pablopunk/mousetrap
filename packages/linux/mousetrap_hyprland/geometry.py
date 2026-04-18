from .hyprctl import active_window_bounds, focused_monitor, logical_monitor_bounds


def target_bounds() -> tuple[int, int, int, int]:
    monitor = focused_monitor()
    return active_window_bounds() or logical_monitor_bounds(monitor)
