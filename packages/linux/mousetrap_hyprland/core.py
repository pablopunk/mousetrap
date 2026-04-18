from dataclasses import dataclass

from .config import ROWS


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
