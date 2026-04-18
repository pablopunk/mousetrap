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


def cell_center(bounds: tuple[int, int, int, int], target: CellTarget) -> tuple[int, int]:
    x, y, width, height = bounds
    col_w = width / target.column_count
    row_h = height / target.row_count
    cx = x + int((target.column + 0.5) * col_w)
    cy = y + int((target.row + 0.5) * row_h)
    return cx, cy
