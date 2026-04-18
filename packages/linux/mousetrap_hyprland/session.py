from __future__ import annotations

import json
import os
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path

from .core import CellTarget, cell_bounds, cell_center, classify_chord, combine_bounds, expanded_bounds, find_cell_for_key, rect_center

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
    pending_keys: list[str] = field(default_factory=list)
    pending_since: float | None = None
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
            pending_keys=list(data.get('pending_keys', [])),
            pending_since=data.get('pending_since'),
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
    keys: list[str]
    step: int
    max_steps: int
    targets: list[CellTarget]
    selected_bounds: tuple[int, int, int, int]
    point: tuple[int, int]
    final: bool
    chord_kind: str | None = None


class OverlaySession:
    def __init__(self, state: SessionState):
        self.state = state

    @classmethod
    def start(cls, bounds: tuple[int, int, int, int]) -> 'OverlaySession':
        return cls(SessionState.start(bounds))

    def queue_key(self, key: str) -> str:
        target = find_cell_for_key(key)
        if not target:
            return 'invalid'
        key = key.lower()
        if self.state.pending_keys:
            if key in self.state.pending_keys:
                return 'duplicate-pending'
            self.state.pending_keys.append(key)
        else:
            self.state.pending_keys = [key]
            self.state.pending_since = time.time()
        self.state.updated_at = time.time()
        return 'pending'

    def commit_pending(self) -> SelectionResult | None:
        if not self.state.pending_keys:
            return None
        targets = [find_cell_for_key(key) for key in self.state.pending_keys]
        if any(target is None for target in targets):
            self.state.pending_keys = []
            self.state.pending_since = None
            return None
        resolved_targets = [target for target in targets if target is not None]
        chord_kind = classify_chord(resolved_targets)
        if chord_kind is None and len(resolved_targets) > 1:
            first = resolved_targets[:1]
            keys = self.state.pending_keys[:1]
            self.state.pending_keys = self.state.pending_keys[1:]
            self.state.pending_since = time.time() if self.state.pending_keys else None
            return self._apply_selection(first, keys, None)
        keys = list(self.state.pending_keys)
        self.state.pending_keys = []
        self.state.pending_since = None
        return self._apply_selection(resolved_targets, keys, chord_kind)

    def _apply_selection(self, targets: list[CellTarget], keys: list[str], chord_kind: str | None) -> SelectionResult:
        rects = [cell_bounds(self.state.current_bounds, target) for target in targets]
        selected_bounds = combine_bounds(rects)
        point = rect_center(selected_bounds) if chord_kind else cell_center(self.state.current_bounds, targets[0])
        final = self.state.step >= self.state.max_steps
        result = SelectionResult(
            keys=keys,
            step=self.state.step,
            max_steps=self.state.max_steps,
            targets=targets,
            selected_bounds=selected_bounds,
            point=point,
            final=final,
            chord_kind=chord_kind,
        )
        self.state.history.append(''.join(keys))
        next_depth = self.state.step
        self.state.current_bounds = expanded_bounds(
            selected_bounds,
            screen_bounds=self.state.initial_bounds,
            next_depth=next_depth,
        )
        self.state.step += 1
        self.state.updated_at = time.time()
        return result
