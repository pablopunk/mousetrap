#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MENU_BAR_ICON="$ROOT/assets/minimal-icon.png"
APP_ICON_OUT="$ROOT/assets/AppIcon.icns"
ICONSET_DIR="$(mktemp -d "${TMPDIR:-/tmp}/mousetrap-iconset.XXXXXX").iconset"

cleanup() {
  rm -rf "$ICONSET_DIR"
}
trap cleanup EXIT

find_compose_icon_source() {
  local candidate

  while IFS= read -r candidate; do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$ROOT/assets" -type f \( -path '*/Assets/*.png' -o -path '*/Assets/*.jpg' -o -path '*/Assets/*.jpeg' \) | sort)

  return 1
}

APP_ICON_SRC="${1:-}"
if [[ -z "$APP_ICON_SRC" ]]; then
  if ! APP_ICON_SRC="$(find_compose_icon_source)"; then
    echo "Could not find an Icon Composer source image under assets/" >&2
    echo "Expected something like assets/*.icon/Assets/*.png" >&2
    exit 1
  fi
fi

if [[ ! -f "$MENU_BAR_ICON" ]]; then
  echo "Menu bar icon not found: $MENU_BAR_ICON" >&2
  exit 1
fi

if [[ ! -f "$APP_ICON_SRC" ]]; then
  echo "App icon source not found: $APP_ICON_SRC" >&2
  exit 1
fi

mkdir -p "$ICONSET_DIR"
mkdir -p "$(dirname "$APP_ICON_OUT")"

echo "Menu bar icon: $MENU_BAR_ICON"
echo "App icon source: $APP_ICON_SRC"
echo "Generating: $APP_ICON_OUT"

sips -z 16 16     "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null
sips -z 32 32     "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
sips -z 32 32     "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null
sips -z 64 64     "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
sips -z 128 128   "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null
sips -z 256 256   "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null
sips -z 512 512   "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$APP_ICON_SRC" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null
cp "$APP_ICON_SRC" "$ICONSET_DIR/icon_512x512@2x.png"

iconutil -c icns "$ICONSET_DIR" -o "$APP_ICON_OUT"

echo "Done. Build uses:"
echo "- Menu bar icon: $MENU_BAR_ICON"
echo "- App icon: $APP_ICON_OUT"
