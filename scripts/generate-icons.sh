#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_ICON_NAME="AppIcon"
ICON_SOURCE="$ROOT/assets/${APP_ICON_NAME}.icon"
OUTPUT_ICNS="${1:-$ROOT/assets/${APP_ICON_NAME}.icns}"
OUTPUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/mousetrap-icon-assets.XXXXXX")"
PARTIAL_PLIST="$OUTPUT_DIR/${APP_ICON_NAME}-partial-info.plist"

cleanup() {
  rm -rf "$OUTPUT_DIR"
}
trap cleanup EXIT

log() {
  printf '%s\n' "$*"
}

if [[ ! -d "$ICON_SOURCE" ]]; then
  echo "Icon Composer source not found: $ICON_SOURCE" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUTPUT_ICNS")"
rm -f "$OUTPUT_ICNS"

log "Icon Composer source: $ICON_SOURCE"
log "Generating app icon: $OUTPUT_ICNS"

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

if [[ ! -f "$OUTPUT_DIR/${APP_ICON_NAME}.icns" ]]; then
  echo "actool did not generate ${APP_ICON_NAME}.icns" >&2
  exit 1
fi

cp "$OUTPUT_DIR/${APP_ICON_NAME}.icns" "$OUTPUT_ICNS"

log "Done. Generated:"
log "- $OUTPUT_ICNS"
