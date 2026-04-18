from __future__ import annotations

import json
import os
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path

from .core import CellTarget, cell_bounds, cell_center, find_cell_for_key

STATE_DIR = Path(os.environ.get('XDG_RUNTIME_DIR', '/tmp')) / 'mousetrap-hyprland'
STATE_FILE = STATE_DIR / 'session.json'
MAX_REFINEMENT_STEPS = 3


@dataclass(slots=True)
class SessionState:
    initial_bounds: tuple[int, int, int, int]
    current_bounds: tuple[int, int, int, int]
    step: int = 1
    max_steps: int = MAX_REFINEMENT_STEPS
    history: list[str] = field(default_factory=list)
    started_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)

    @classmethod
    def start(cls, bounds: tuple[int, int, int, int]) -> 'SessionState':
        return cls(initial_bounds=bounds, current_bounds=bounds)

    @classmethod
    def load(cls) -> 'SessionState | None':
        if not STATE_FILE.exists():
            return None
        data = json.loads(STATE_FILE.read_text())
        return cls(
            initial_bounds=tuple(data['initial_bounds']),
            current_bounds=tuple(data['current_bounds']),
            step=data['step'],
            max_steps=data.get('max_steps', MAX_REFINEMENT_STEPS),
            history=list(data.get('history', [])),
            started_at=data.get('started_at', time.time()),
            updated_at=data.get('updated_at', time.time()),
        )

    def save(self) -> Path:
        STATE_DIR.mkdir(parents=True, exist_ok=True)
        STATE_FILE.write_text(json.dumps(asdict(self), indent=2) + '\n')
        return STATE_FILE

    def clear(self) -> None:
        try:
            STATE_FILE.unlink()
        except FileNotFoundError:
            pass

    def has_timed_out(self, timeout_seconds: float) -> bool:
        return timeout_seconds > 0 and (time.time() - self.updated_at) > timeout_seconds


@dataclass(frozen=True, slots=True)
class SelectionResult:
    key: str
    step: int
    max_steps: int
    target: CellTarget
    selected_bounds: tuple[int, int, int, int]
    point: tuple[int, int]
    final: bool


class OverlaySession:
    def __init__(self, state: SessionState):
        self.state = state

    @classmethod
    def start(cls, bounds: tuple[int, int, int, int]) -> 'OverlaySession':
        return cls(SessionState.start(bounds))

    def resolve_key(self, key: str) -> SelectionResult | None:
        target = find_cell_for_key(key)
        if not target:
            return None
        bounds = cell_bounds(self.state.current_bounds, target)
        point = cell_center(self.state.current_bounds, target)
        final = self.state.step >= self.state.max_steps
        result = SelectionResult(
            key=key.lower(),
            step=self.state.step,
            max_steps=self.state.max_steps,
            target=target,
            selected_bounds=bounds,
            point=point,
            final=final,
        )
        self.state.history.append(key.lower())
        self.state.current_bounds = bounds
        self.state.step += 1
        self.state.updated_at = time.time()
        return result
