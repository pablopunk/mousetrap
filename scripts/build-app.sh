#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONFIGURATION="${CONFIGURATION:-debug}"
APP_NAME="Mousetrap"
BUNDLE_ID="com.pablopunk.mousetrap"
BUILD_DIR="$ROOT/.build/$CONFIGURATION"
STAGING_APP_DIR="$BUILD_DIR/$APP_NAME.app"
INSTALL_DIR="${INSTALL_DIR:-$HOME/Applications}"
INSTALLED_APP_DIR="$INSTALL_DIR/$APP_NAME.app"
EXECUTABLE="$BUILD_DIR/$APP_NAME"

swift build -c "$CONFIGURATION"

rm -rf "$STAGING_APP_DIR"
mkdir -p "$STAGING_APP_DIR/Contents/MacOS"
mkdir -p "$STAGING_APP_DIR/Contents/Resources"

cp "$EXECUTABLE" "$STAGING_APP_DIR/Contents/MacOS/$APP_NAME"

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

mkdir -p "$INSTALL_DIR"
rm -rf "$INSTALLED_APP_DIR"
cp -R "$STAGING_APP_DIR" "$INSTALLED_APP_DIR"

echo "Built $STAGING_APP_DIR"
echo "Installed $INSTALLED_APP_DIR"
