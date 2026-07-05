#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LINUX_DIR="$ROOT/packages/linux"
VERSION="$(tr -d '[:space:]' < "$ROOT/VERSION")"
DIST_DIR="$ROOT/dist/linux/mousetrap-hyprland-$VERSION"
BIN_DIR="$DIST_DIR/bin"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR" "$BIN_DIR"

cp -R "$LINUX_DIR" "$DIST_DIR/package"
rm -rf "$DIST_DIR/package"/__pycache__ "$DIST_DIR/package"/mousetrap_hyprland/__pycache__ 2>/dev/null || true

cat > "$DIST_DIR/README-BUNDLE.md" <<'EOF'
This Linux bundle includes the Mousetrap Hyprland package and, when available,
local copies of ydotool/ydotoold.

Important:
- ydotool still depends on kernel/uinput access and a running user session.
- bundling the binaries helps distribution, but it does not remove the need for
  proper permissions or a working ydotool user service / equivalent launcher.
- for a polished release we should likely ship an AppImage or distro package plus
  a managed user service file.
EOF

for bin in ydotool ydotoold; do
  if command -v "$bin" >/dev/null 2>&1; then
    cp "$(command -v "$bin")" "$BIN_DIR/$bin"
  fi
done

if [[ -f /usr/lib/systemd/user/ydotool.service ]]; then
  mkdir -p "$DIST_DIR/systemd-user"
  cp /usr/lib/systemd/user/ydotool.service "$DIST_DIR/systemd-user/"
fi

cd "$ROOT/dist/linux"
tar -czf "mousetrap-hyprland-$VERSION.tar.gz" "mousetrap-hyprland-$VERSION"
echo "Created $ROOT/dist/linux/mousetrap-hyprland-$VERSION.tar.gz"
