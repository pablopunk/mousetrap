#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
PYTHONPATH="$DIR${PYTHONPATH:+:$PYTHONPATH}" python3 -m mousetrap_hyprland.cli activate
"$DIR/dynamic_bind.sh"
