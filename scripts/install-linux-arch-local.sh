#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
"$ROOT/scripts/build-linux-arch-package.sh"
LATEST_PKG="$(find "$ROOT/dist/arch-build" -maxdepth 1 -type f -name '*.pkg.tar.zst' | sort | tail -n1)"
if [[ -z "$LATEST_PKG" ]]; then
  echo 'No package artifact found' >&2
  exit 1
fi
sudo pacman -U --noconfirm "$LATEST_PKG"
