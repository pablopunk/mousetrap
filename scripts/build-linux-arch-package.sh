#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(tr -d '[:space:]' < "$ROOT/VERSION")"
WORKDIR="$ROOT/dist/arch-build"
PKGDIR="$ROOT/packaging/arch"
LOCAL_SRC="$WORKDIR/localsrc/mousetrap-$VERSION"

rm -rf "$WORKDIR"
mkdir -p "$LOCAL_SRC"

rsync -a \
  --exclude '.git' \
  --exclude '.build' \
  --exclude 'dist' \
  --exclude '__pycache__' \
  --exclude '*.pyc' \
  "$ROOT/" "$LOCAL_SRC/"

cp "$PKGDIR/mousetrap-hyprland.install" "$WORKDIR/"
cat > "$WORKDIR/PKGBUILD" <<EOF
# Maintainer: Pablo Varela <pablo@example.com>

pkgname=mousetrap-hyprland
pkgver=$VERSION
pkgrel=1
pkgdesc='Keyboard-driven mouse targeting for Hyprland'
arch=('any')
url='https://github.com/pablopunk/mousetrap'
license=('GPL-3.0-or-later')
depends=(
  'python'
  'python-gobject'
  'python-cairo'
  'gtk4'
  'gtk4-layer-shell'
  'hyprland'
  'jq'
  'ydotool'
)
makedepends=('python-build' 'python-installer' 'python-setuptools')
source=('mousetrap-hyprland.install')
sha256sums=('SKIP')
install=mousetrap-hyprland.install

_local_src='$LOCAL_SRC'

build() {
  cd "
	src_placeholder
"
  python -m build --wheel --no-isolation
}

package() {
  cd "
	repo_placeholder
"
  python -m installer --destdir="\$pkgdir" packages/linux/dist/*.whl
  install -Dm755 packages/linux/activate.sh "\$pkgdir/usr/bin/mousetrap-hyprland-activate"
  install -Dm755 packages/linux/select.sh "\$pkgdir/usr/bin/mousetrap-hyprland-select"
  install -Dm755 packages/linux/cancel.sh "\$pkgdir/usr/bin/mousetrap-hyprland-cancel"
  install -Dm644 packages/linux/mousetrap.conf "\$pkgdir/usr/share/doc/\$pkgname/mousetrap.conf"
  install -Dm644 packages/linux/README.md "\$pkgdir/usr/share/doc/\$pkgname/README.md"
  install -Dm644 README.md "\$pkgdir/usr/share/doc/\$pkgname/README-repo.md"
  install -Dm644 VERSION "\$pkgdir/usr/share/\$pkgname/VERSION"
}
EOF
python3 - <<PY
from pathlib import Path
p = Path('$WORKDIR/PKGBUILD')
text = p.read_text()
text = text.replace('cd "\n\tsrc_placeholder\n"', f'cd "{Path("$LOCAL_SRC") / "packages/linux"}"')
text = text.replace('cd "\n\trepo_placeholder\n"', f'cd "{Path("$LOCAL_SRC")}"')
p.write_text(text)
PY

( cd "$WORKDIR" && makepkg -fs --noconfirm )

echo "Built Arch package(s) in $WORKDIR"
