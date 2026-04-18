# Arch packaging for Mousetrap Hyprland

This directory contains the first distro-native packaging for Linux.

## Goals

- let pacman resolve architecture-specific dependencies like `ydotool`
- keep Linux releases distro-native instead of bundling random binaries
- use the repo root `VERSION` as the package version source

## Files

- `PKGBUILD`: package recipe
- `mousetrap-hyprland.install`: post-install guidance

## Local build

```bash
make package-linux-arch
```

## Local install

```bash
make install-linux-arch-local
```

## Notes

The PKGBUILD currently builds from a tagged GitHub release tarball (`v$VERSION`).
For local iteration, the Make targets generate a temporary PKGBUILD with the current repo snapshot.
