#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_ICON_NAME="AppIcon"
ICON_SOURCE="$ROOT/assets/${APP_ICON_NAME}.icon"
MENU_BAR_ICON="$ROOT/assets/minimal-icon.png"
OUTPUT_DIR="${1:-$ROOT/.build/generated-icons}"
PARTIAL_PLIST="$OUTPUT_DIR/${APP_ICON_NAME}-partial-info.plist"

log() {
  printf '%s\n' "$*"
}

if [[ ! -d "$ICON_SOURCE" ]]; then
  echo "Icon Composer source not found: $ICON_SOURCE" >&2
  exit 1
fi

if [[ ! -f "$MENU_BAR_ICON" ]]; then
  echo "Menu bar icon not found: $MENU_BAR_ICON" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"
rm -f "$OUTPUT_DIR/Assets.car" "$OUTPUT_DIR/${APP_ICON_NAME}.icns" "$PARTIAL_PLIST"

log "Icon Composer source: $ICON_SOURCE"
log "Generating icon assets in: $OUTPUT_DIR"
log "Running actool with full output enabled for debugging"

xcrun actool \
  "$ICON_SOURCE" \
  --compile "$OUTPUT_DIR" \
  --app-icon "$APP_ICON_NAME" \
  --platform macosx \
  --minimum-deployment-target 14.0 \
  --output-partial-info-plist "$PARTIAL_PLIST" \
  --standalone-icon-behavior default \
  --errors \
  --warnings \
  --notices

if [[ ! -f "$OUTPUT_DIR/Assets.car" ]]; then
  echo "actool did not generate Assets.car" >&2
  exit 1
fi

if [[ ! -f "$OUTPUT_DIR/${APP_ICON_NAME}.icns" ]]; then
  echo "actool did not generate ${APP_ICON_NAME}.icns" >&2
  exit 1
fi

log "Done. Generated:"
log "- $OUTPUT_DIR/Assets.car"
log "- $OUTPUT_DIR/${APP_ICON_NAME}.icns"
log "- $PARTIAL_PLIST"
log "Menu bar icon remains:"
log "- $MENU_BAR_ICON"
