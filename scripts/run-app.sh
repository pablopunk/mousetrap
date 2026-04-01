#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="${APP_NAME:-Mousetrap Dev}"
BUNDLE_ID="${BUNDLE_ID:-com.pablopunk.mousetrap.dev}"
EXECUTABLE_NAME="${EXECUTABLE_NAME:-Mousetrap}"
INSTALL_DIR="${INSTALL_DIR:-/Applications}"
APP_PATH="$INSTALL_DIR/$APP_NAME.app"

pkill -x "$EXECUTABLE_NAME" || true
sleep 0.2

APP_NAME="$APP_NAME" BUNDLE_ID="$BUNDLE_ID" EXECUTABLE_NAME="$EXECUTABLE_NAME" INSTALL_DIR="$INSTALL_DIR" "$ROOT/scripts/build-app.sh"
open "$APP_PATH"
