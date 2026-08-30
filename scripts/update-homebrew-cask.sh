#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
ZIP_PATH="${2:-}"

if [[ -z "$VERSION" || -z "$ZIP_PATH" ]]; then
  echo "Usage: ./scripts/update-homebrew-cask.sh x.y.z /path/to/Mousetrap.zip"
  exit 1
fi

if [[ ! -f "$ZIP_PATH" ]]; then
  echo "ZIP not found: $ZIP_PATH"
  exit 1
fi

TAP_REPO="${TAP_REPO:-pablopunk/homebrew-brew}"
TAP_DIR="$(mktemp -d /tmp/mousetrap-homebrew.XXXXXX)"
CASK_PATH="$TAP_DIR/Casks/mousetrap.rb"
ZIP_SHA="$(shasum -a 256 "$ZIP_PATH" | awk '{print $1}')"
GIT_REMOTE="https://github.com/$TAP_REPO.git"

cleanup() {
  rm -rf "$TAP_DIR"
}
trap cleanup EXIT

if [[ -n "${GH_PAT:-}" ]]; then
  GIT_REMOTE="https://x-access-token:${GH_PAT}@github.com/$TAP_REPO.git"
fi

git clone "$GIT_REMOTE" "$TAP_DIR"

cat > "$CASK_PATH" <<EOF
cask "mousetrap" do
  version "$VERSION"
  sha256 "$ZIP_SHA"

  url "https://github.com/pablopunk/mousetrap/releases/download/v#{version}/Mousetrap.zip"
  name "Mousetrap"
  desc "Keyboard-driven mouse control for macOS"
  homepage "https://github.com/pablopunk/mousetrap"

  app "Mousetrap.app"

  zap trash: [
    "~/Library/Preferences/com.pablopunk.mousetrap.plist",
  ]
end
EOF

ruby -c "$CASK_PATH"

pushd "$TAP_DIR" >/dev/null
git config user.name "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"

if ! git diff --quiet -- Casks/mousetrap.rb; then
  git add Casks/mousetrap.rb
  git commit -m "mousetrap: update to v$VERSION"
  git push
else
  echo "Homebrew cask already up to date."
fi
popd >/dev/null

echo "Updated Homebrew cask to v$VERSION"
