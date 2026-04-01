#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONFIGURATION="${CONFIGURATION:-debug}"
APP_NAME="Mousetrap"
BUNDLE_ID="com.pablopunk.mousetrap"
BUILD_DIR="$ROOT/.build/$CONFIGURATION"
STAGING_APP_DIR="$BUILD_DIR/$APP_NAME.app"
INSTALL_DIR="${INSTALL_DIR:-/Applications}"
INSTALLED_APP_DIR="$INSTALL_DIR/$APP_NAME.app"
EXECUTABLE="$BUILD_DIR/$APP_NAME"
ICON_ASSETS_DIR="$BUILD_DIR/icon-assets"
APP_ICON_NAME="AppIcon"

swift build -c "$CONFIGURATION"
"$ROOT/scripts/generate-icons.sh" "$ICON_ASSETS_DIR"

rm -rf "$STAGING_APP_DIR"
mkdir -p "$STAGING_APP_DIR/Contents/MacOS"
mkdir -p "$STAGING_APP_DIR/Contents/Resources"

cp "$EXECUTABLE" "$STAGING_APP_DIR/Contents/MacOS/$APP_NAME"
cp "$ICON_ASSETS_DIR/Assets.car" "$STAGING_APP_DIR/Contents/Resources/Assets.car"
cp "$ICON_ASSETS_DIR/$APP_ICON_NAME.icns" "$STAGING_APP_DIR/Contents/Resources/$APP_ICON_NAME.icns"

if [[ -f "$ROOT/assets/minimal-icon.png" ]]; then
  cp "$ROOT/assets/minimal-icon.png" "$STAGING_APP_DIR/Contents/Resources/minimal-icon.png"
fi

cat > "$STAGING_APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
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
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSAppleEventsUsageDescription</key>
  <string>Mousetrap may automate clicks later.</string>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$STAGING_APP_DIR"

mkdir -p "$INSTALL_DIR"
rm -rf "$INSTALLED_APP_DIR"
ditto "$STAGING_APP_DIR" "$INSTALLED_APP_DIR"
codesign --force --deep --sign - "$INSTALLED_APP_DIR"

echo "Built $STAGING_APP_DIR"
echo "Installed $INSTALLED_APP_DIR"
