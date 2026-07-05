from dataclasses import dataclass

from .config import MAX_COLUMNS, ROWS

REFINEMENT_WIDTH_EXPANSION_RATIO = 0.5
FINAL_CLICK_BASE_WIDTH_EXPANSION_RATIO = 0.5
FINAL_CLICK_LAPTOP_HEIGHT_EXPANSION_RATIO = 0.08
FINAL_CLICK_DESKTOP_HEIGHT_EXPANSION_RATIO = 0.03
FINAL_CLICK_LAPTOP_TARGET_KEY_WIDTH = 22
FINAL_CLICK_DESKTOP_TARGET_KEY_WIDTH = 19
FINAL_CLICK_MAX_SCREEN_WIDTH_FRACTION = 0.4
COMPACT_SCREEN_WIDTH_THRESHOLD = 1600


@dataclass(frozen=True)
class CellTarget:
    key: str
    row: int
    column: int
    row_count: int
    column_count: int


def find_cell_for_key(key: str):
    key = (key or '').lower()
    for row_index, row in enumerate(ROWS):
        if key in row:
            return CellTarget(
                key=key,
                row=row_index,
                column=row.index(key),
                row_count=len(ROWS),
                column_count=len(row),
            )
    return None


def cell_bounds(bounds: tuple[int, int, int, int], target: CellTarget) -> tuple[int, int, int, int]:
    x, y, width, height = bounds
    col_w = width / target.column_count
    row_h = height / target.row_count
    left = x + int(target.column * col_w)
    top = y + int(target.row * row_h)
    right = x + int((target.column + 1) * col_w)
    bottom = y + int((target.row + 1) * row_h)
    return left, top, max(1, right - left), max(1, bottom - top)


def cell_center(bounds: tuple[int, int, int, int], target: CellTarget) -> tuple[int, int]:
    left, top, width, height = cell_bounds(bounds, target)
    return left + width // 2, top + height // 2


def combine_bounds(cell_rects: list[tuple[int, int, int, int]]) -> tuple[int, int, int, int]:
    left = min(x for x, _, _, _ in cell_rects)
    top = min(y for _, y, _, _ in cell_rects)
    right = max(x + w for x, _, w, _ in cell_rects)
    bottom = max(y + h for _, y, _, h in cell_rects)
    return left, top, max(1, right - left), max(1, bottom - top)


def rect_center(bounds: tuple[int, int, int, int]) -> tuple[int, int]:
    x, y, w, h = bounds
    return x + w // 2, y + h // 2


def classify_chord(targets: list[CellTarget]) -> str | None:
    if len(targets) == 2:
        rows = {target.row for target in targets}
        cols = {target.column for target in targets}
        same_row = len(rows) == 1 and max(cols) - min(cols) == 1
        same_col = len(cols) == 1 and max(rows) - min(rows) == 1
        if same_row or same_col:
            return 'pair'
    if len(targets) == 4:
        rows = sorted({target.row for target in targets})
        cols = sorted({target.column for target in targets})
        if len(rows) == 2 and len(cols) == 2:
            expected = {(r, c) for r in rows for c in cols}
            actual = {(target.row, target.column) for target in targets}
            if actual == expected:
                return 'quad'
    return None


def expanded_bounds(rect: tuple[int, int, int, int], *, screen_bounds: tuple[int, int, int, int], next_depth: int) -> tuple[int, int, int, int]:
    width_expansion_ratio = 0.0
    height_expansion_ratio = 0.0
    _, _, screen_width, _ = screen_bounds
    _, _, rect_width, _ = rect

    if next_depth == 1:
        width_expansion_ratio = REFINEMENT_WIDTH_EXPANSION_RATIO
    elif next_depth == 2:
        compact_screen_factor = max(0.0, min(1.0, (COMPACT_SCREEN_WIDTH_THRESHOLD - screen_width) / 500.0))
        target_key_width = FINAL_CLICK_DESKTOP_TARGET_KEY_WIDTH + ((FINAL_CLICK_LAPTOP_TARGET_KEY_WIDTH - FINAL_CLICK_DESKTOP_TARGET_KEY_WIDTH) * compact_screen_factor)
        height_expansion_ratio = FINAL_CLICK_DESKTOP_HEIGHT_EXPANSION_RATIO + ((FINAL_CLICK_LAPTOP_HEIGHT_EXPANSION_RATIO - FINAL_CLICK_DESKTOP_HEIGHT_EXPANSION_RATIO) * compact_screen_factor)
        base_width = rect_width * (1 + 2 * FINAL_CLICK_BASE_WIDTH_EXPANSION_RATIO)
        target_width_from_keys = target_key_width * MAX_COLUMNS
        max_allowed_width = screen_width * FINAL_CLICK_MAX_SCREEN_WIDTH_FRACTION
        desired_width = min(max(base_width, target_width_from_keys), max_allowed_width)
        width_expansion_ratio = max(0.0, (desired_width / max(rect_width, 1) - 1) / 2)

    if width_expansion_ratio <= 0 and height_expansion_ratio <= 0:
        return rect

    return inset_and_clip(rect, screen_bounds, width_expansion_ratio, height_expansion_ratio)


def inset_and_clip(rect: tuple[int, int, int, int], screen_bounds: tuple[int, int, int, int], width_expansion_ratio: float, height_expansion_ratio: float) -> tuple[int, int, int, int]:
    x, y, width, height = rect
    sx, sy, sw, sh = screen_bounds
    expanded_x = x - int(width * width_expansion_ratio)
    expanded_y = y - int(height * height_expansion_ratio)
    expanded_w = width + int(width * width_expansion_ratio * 2)
    expanded_h = height + int(height * height_expansion_ratio * 2)

    left = max(expanded_x, sx)
    top = max(expanded_y, sy)
    right = min(expanded_x + expanded_w, sx + sw)
    bottom = min(expanded_y + expanded_h, sy + sh)
    return left, top, max(1, right - left), max(1, bottom - top)
