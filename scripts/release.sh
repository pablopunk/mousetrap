#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RAW_VERSION="${1:-}"
if [[ -z "$RAW_VERSION" ]]; then
  echo "Usage: ./scripts/release.sh x.y.z"
  exit 1
fi

VERSION="${RAW_VERSION#v}"
TAG="v$VERSION"
APP_NAME="Mousetrap"
ZIP_PATH="$ROOT/dist/$APP_NAME.zip"
SHA_PATH="$ZIP_PATH.sha256"
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:-}"
NOTARY_KEYCHAIN_PROFILE="${NOTARY_KEYCHAIN_PROFILE:-${NOTARY_PROFILE:-SwiftShift}}"

find_codesign_identity() {
  security find-identity -v -p codesigning 2>/dev/null \
    | grep '"Developer ID Application:' \
    | sed -E 's/^.*"(Developer ID Application:[^"]+)"$/\1/'
}

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Version must look like x.y.z"
  exit 1
fi

if [[ -z "$CODESIGN_IDENTITY" ]]; then
  mapfile -t AVAILABLE_IDENTITIES < <(find_codesign_identity)

  if [[ ${#AVAILABLE_IDENTITIES[@]} -eq 1 ]]; then
    CODESIGN_IDENTITY="${AVAILABLE_IDENTITIES[0]}"
    echo "Using signing identity: $CODESIGN_IDENTITY"
  elif [[ ${#AVAILABLE_IDENTITIES[@]} -gt 1 ]]; then
    echo "Found multiple Developer ID Application identities. Set CODESIGN_IDENTITY explicitly:" >&2
    printf '  %s\n' "${AVAILABLE_IDENTITIES[@]}" >&2
    exit 1
  else
    echo "No Developer ID Application signing identity found in your keychain." >&2
    echo "Install a Developer ID Application certificate, or set CODESIGN_IDENTITY explicitly." >&2
    exit 1
  fi
fi

if [[ -z "$NOTARY_KEYCHAIN_PROFILE" ]]; then
  echo "Missing NOTARY_KEYCHAIN_PROFILE" >&2
  exit 1
fi

if ! xcrun notarytool history --keychain-profile "$NOTARY_KEYCHAIN_PROFILE" >/dev/null 2>&1; then
  echo "Notary profile '$NOTARY_KEYCHAIN_PROFILE' is not available." >&2
  echo "Create it once with:" >&2
  echo "  xcrun notarytool store-credentials \"$NOTARY_KEYCHAIN_PROFILE\" --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID" >&2
  exit 1
fi

CURRENT_BRANCH="$(git branch --show-current)"
if [[ "$CURRENT_BRANCH" != "main" ]]; then
  echo "Release from main only. Current branch: $CURRENT_BRANCH"
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree must be clean before releasing."
  git status --short
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Tag already exists: $TAG"
  exit 1
fi

printf '%s\n' "$VERSION" > "$ROOT/VERSION"
git add VERSION

if git diff --cached --quiet; then
  echo "VERSION already at $VERSION"
else
  git commit -m "Release $TAG"
fi

echo "Building signed + notarized release..."
CODESIGN_IDENTITY="$CODESIGN_IDENTITY" NOTARY_KEYCHAIN_PROFILE="$NOTARY_KEYCHAIN_PROFILE" make build-release

PREV_TAG="$(git tag --sort=-v:refname | grep -v "^$TAG$" | head -1 || true)"

git tag "$TAG"
git push origin main
git push origin "$TAG"

if gh release view "$TAG" >/dev/null 2>&1; then
  echo "GitHub release already exists: $TAG"
  exit 1
fi

echo "Creating GitHub release..."
if [[ -n "$PREV_TAG" ]]; then
  gh release create "$TAG" "$ZIP_PATH" "$SHA_PATH" \
    --title "$TAG" \
    --generate-notes \
    --notes-start-tag "$PREV_TAG"
else
  gh release create "$TAG" "$ZIP_PATH" "$SHA_PATH" \
    --title "$TAG" \
    --generate-notes
fi

if [[ -n "${GH_PAT:-}" ]]; then
  echo "Updating Homebrew tap..."
  bash "$ROOT/scripts/update-homebrew-cask.sh" "$VERSION" "$ZIP_PATH"
else
  echo "Skipping Homebrew tap update because GH_PAT is not set."
fi

echo "✅ Release $TAG complete"
