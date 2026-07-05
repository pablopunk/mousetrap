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

The `packaging/arch/PKGBUILD` is the future stable-package recipe and expects a tagged GitHub release tarball (`v$VERSION`) that already contains `packages/linux/`.
For local iteration, the Make targets generate a temporary PKGBUILD with the current repo snapshot.

For AUR publication **right now**, use `packaging/aur/` and publish the `mousetrap-hyprland-git` package first.
