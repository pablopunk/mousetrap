#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_DIR="${INSTALL_DIR:-/Applications}"
APP_PATH="$INSTALL_DIR/Mousetrap.app"

pkill -x Mousetrap || true
sleep 0.2

"$ROOT/scripts/build-app.sh"
open "$APP_PATH"
