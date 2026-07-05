#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONFIGURATION="${CONFIGURATION:-debug}"
APP_NAME="${APP_NAME:-Mousetrap}"
BUNDLE_ID="${BUNDLE_ID:-com.pablopunk.mousetrap}"
EXECUTABLE_NAME="${EXECUTABLE_NAME:-Mousetrap}"
VERSION_FILE="$ROOT/VERSION"
APP_VERSION="${APP_VERSION:-$(tr -d '[:space:]' < "$VERSION_FILE")}" 
BUILD_NUMBER="${BUILD_NUMBER:-$APP_VERSION}"
BUILD_DIR="$ROOT/.build/$CONFIGURATION"
STAGING_APP_DIR="$BUILD_DIR/$APP_NAME.app"
INSTALL_DIR="${INSTALL_DIR:-/Applications}"
INSTALLED_APP_DIR="$INSTALL_DIR/$APP_NAME.app"
INSTALL_APP="${INSTALL_APP:-1}"
EXECUTABLE="$BUILD_DIR/$EXECUTABLE_NAME"
APP_ICON_NAME="AppIcon"
APP_ICON_FILE="$ROOT/assets/${APP_ICON_NAME}.icns"
PREFER_STABLE_DEV_SIGNING="${PREFER_STABLE_DEV_SIGNING:-0}"
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:-}"

find_apple_development_identity() {
  security find-identity -v -p codesigning 2>/dev/null \
    | grep '"Apple Development:' \
    | sed -E 's/^.*"(Apple Development:[^"]+)"$/\1/'
}

if [[ -z "$CODESIGN_IDENTITY" && "$PREFER_STABLE_DEV_SIGNING" == "1" ]]; then
  mapfile -t AVAILABLE_DEV_IDENTITIES < <(find_apple_development_identity)

  if [[ ${#AVAILABLE_DEV_IDENTITIES[@]} -eq 1 ]]; then
    CODESIGN_IDENTITY="${AVAILABLE_DEV_IDENTITIES[0]}"
    echo "Using Apple Development signing identity: $CODESIGN_IDENTITY"
  elif [[ ${#AVAILABLE_DEV_IDENTITIES[@]} -gt 1 ]]; then
    echo "Found multiple Apple Development signing identities; falling back to ad-hoc signing." >&2
    echo "Set CODESIGN_IDENTITY explicitly to keep Accessibility permissions stable." >&2
    printf '  %s\n' "${AVAILABLE_DEV_IDENTITIES[@]}" >&2
  fi
fi

if [[ -z "$CODESIGN_IDENTITY" ]]; then
  CODESIGN_IDENTITY="-"
  if [[ "$PREFER_STABLE_DEV_SIGNING" == "1" ]]; then
    echo "Warning: using ad-hoc signing for $APP_NAME." >&2
    echo "Accessibility permissions may need to be re-granted after rebuilds." >&2
    echo "Set CODESIGN_IDENTITY='Apple Development: Your Name (TEAMID)' to avoid that." >&2
  fi
fi

CODESIGN_OPTIONS=(--force --deep --sign "$CODESIGN_IDENTITY")

if [[ "$CODESIGN_IDENTITY" != "-" ]]; then
  CODESIGN_OPTIONS+=(--options runtime --timestamp)
fi

swift build -c "$CONFIGURATION"

if [[ ! -f "$APP_ICON_FILE" ]]; then
  echo "App icon not found: $APP_ICON_FILE" >&2
  echo "Generate it manually with: ./scripts/generate-icons.sh" >&2
  exit 1
fi

rm -rf "$STAGING_APP_DIR"
mkdir -p "$STAGING_APP_DIR/Contents/MacOS"
mkdir -p "$STAGING_APP_DIR/Contents/Resources"

cp "$EXECUTABLE" "$STAGING_APP_DIR/Contents/MacOS/$EXECUTABLE_NAME"
cp "$APP_ICON_FILE" "$STAGING_APP_DIR/Contents/Resources/$APP_ICON_NAME.icns"

if [[ -f "$ROOT/assets/minimal-icon.png" ]]; then
  cp "$ROOT/assets/minimal-icon.png" "$STAGING_APP_DIR/Contents/Resources/minimal-icon.png"
fi

cp "$VERSION_FILE" "$STAGING_APP_DIR/Contents/Resources/VERSION"

cat > "$STAGING_APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>$EXECUTABLE_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleIconFile</key>
  <string>$APP_ICON_NAME</string>
  <key>CFBundleIconName</key>
  <string>$APP_ICON_NAME</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$APP_VERSION</string>
  <key>CFBundleVersion</key>
  <string>$BUILD_NUMBER</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSAppleEventsUsageDescription</key>
  <string>Mousetrap may automate clicks later.</string>
</dict>
</plist>
PLIST

codesign "${CODESIGN_OPTIONS[@]}" "$STAGING_APP_DIR"

echo "Built $STAGING_APP_DIR"

if [[ "$INSTALL_APP" == "1" ]]; then
  mkdir -p "$INSTALL_DIR"
  rm -rf "$INSTALLED_APP_DIR"
  ditto "$STAGING_APP_DIR" "$INSTALLED_APP_DIR"
  codesign "${CODESIGN_OPTIONS[@]}" "$INSTALLED_APP_DIR"
  echo "Installed $INSTALLED_APP_DIR"
fi
