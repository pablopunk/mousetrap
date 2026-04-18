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
