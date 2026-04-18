#!/usr/bin/env bash
set -euo pipefail
key="${1:?missing key}"
DIR="$(cd "$(dirname "$0")" && pwd)"
if command -v mousetrap-hyprland >/dev/null 2>&1; then
  exec mousetrap-hyprland select "$key"
fi
PYTHONPATH="$DIR${PYTHONPATH:+:$PYTHONPATH}" exec python3 -m mousetrap_hyprland.cli select "$key"
