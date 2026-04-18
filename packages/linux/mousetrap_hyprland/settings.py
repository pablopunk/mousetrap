from __future__ import annotations

import json
import os
from dataclasses import asdict, dataclass
from pathlib import Path


CONFIG_DIR = Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config')) / 'mousetrap'
CONFIG_FILE = CONFIG_DIR / 'hyprland.json'


@dataclass(slots=True)
class Settings:
    activation_mode: str = 'dynamic'
    overlay_dismiss_delay_seconds: float = 0.12
    pre_warp_delay_seconds: float = 0.05
    post_warp_delay_seconds: float = 0.18
    click_backend: str = 'ydotool'
    bundle_ydotool_in_release: bool = True

    @classmethod
    def load(cls) -> 'Settings':
        if not CONFIG_FILE.exists():
            return cls()
        data = json.loads(CONFIG_FILE.read_text())
        return cls(**{k: v for k, v in data.items() if k in cls.__dataclass_fields__})

    def save(self) -> Path:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        CONFIG_FILE.write_text(json.dumps(asdict(self), indent=2) + '\n')
        return CONFIG_FILE
