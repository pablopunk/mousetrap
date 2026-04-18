#!/usr/bin/env bash
set -euo pipefail
key="${1:?missing key}"
PYTHONPATH="$(dirname "$0")${PYTHONPATH:+:$PYTHONPATH}" exec python3 -m mousetrap_hyprland.cli select "$key"
