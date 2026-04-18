# AUR package files

These files are the AUR-friendly packaging path for Linux **right now**.

Why `-git` first?
- the repo's existing tagged releases do not yet include the Linux/Hyprland package layout
- the `linux` branch does
- so the cleanest current AUR path is `mousetrap-hyprland-git`

## Publish flow

1. Copy `PKGBUILD`, `.SRCINFO`, and `mousetrap-hyprland.install` into an AUR repo for `mousetrap-hyprland-git`
2. Commit and push to AUR
3. Users can install with their preferred helper

## Future

Once the first repo tag that contains `packages/linux/` is released, add a non-`-git` AUR package that builds from the release tarball.
