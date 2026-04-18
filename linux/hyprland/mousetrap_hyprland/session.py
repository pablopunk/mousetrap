from dataclasses import dataclass

from .core import cell_center, find_cell_for_key


@dataclass
class OverlaySession:
    bounds: tuple[int, int, int, int]

    def resolve_key(self, key: str):
        target = find_cell_for_key(key)
        if not target:
            return None
        x, y = cell_center(self.bounds, target)
        return {'key': key.lower(), 'x': x, 'y': y, 'target': target}
