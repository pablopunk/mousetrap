# Mousetrap

> Get it? It kills the mouse.

<p align="center">
  <img width="128" height="128" alt="icon" src="https://github.com/pablopunk/mousetrap/blob/main/assets/AppIcon.png?raw=true" />
</p>
<p align="center">
<span>Touch your screen. But not really.</span>
<img width="100%" alt="explain" src="https://github.com/pablopunk/mousetrap/blob/main/assets/keyboard-keys-screen.png?raw=true" />
</p>

https://github.com/user-attachments/assets/7aa6cd9a-ce6a-49e1-9844-28a37eb02382

## Platform Support

- **macOS**: Full-featured, production-ready
- **Linux (Hyprland)**: Work in progress — basic single-key targeting works

## Install

### macOS

```bash
brew install pablopunk/brew/mousetrap
```

Or grab the latest release from the [releases page](https://github.com/pablopunk/mousetrap/releases).

### Linux (Hyprland)

See [packages/linux/README.md](packages/linux/README.md) for setup instructions.

Quick start:

```bash
# Install dependencies (Arch example)
sudo pacman -S gtk4 gtk4-layer-shell python python-gobject cairo jq ydotool
systemctl --user enable --now ydotool.service

# Build + validate
make build-linux
make doctor-linux
make config-linux

# Configure Hyprland
# Add to your hyprland.conf:
bind = SUPER, SPACE, exec, /path/to/mousetrap/packages/linux/activate.sh
```

## Features

- ⚡ Native, lightweight, fast. Lives in your menu bar (macOS) or integrates with your compositor (Linux).
- 💅 Custom keyboard shortcuts.
- 🌐 Real keys, read from your current keyboard layout.
- 🎯 Three-step nested grid for fast, precise targeting.
- ✨ Multi-key chord targeting: press adjacent keys like `zx` or `aszx` to aim between cells or at shared corners.
- 🖱️ Free mouse mode. Use your arrow keys to move the cursor, click, double-click, right-click, and drag with as much precision as you want.
- 👀 Optional pulse reveal mode to fade the grid and better see what's underneath.
- 👨‍💻 Free and open source.

## Usage

- Use your keyboard shortcut to trigger the fullscreen grid.
- The grid follows your real keyboard layout.
- There are 3 nested grids to quickly narrow down the target.
- Press a key to focus that cell and continue refining.
- You can also press adjacent keys together, like `zx` or `aszx`, to target the midpoint or corner between cells.
- Chords work on every grid level: on earlier grids they recenter the next refinement, and on the final grid they click that exact point.
- At any point you can hit any arrow key to move the mouse freely, hit Enter to click, hit Enter twice to double-click, use Shift+Enter to right-click, or use Shift+arrow keys to drag.

## Development

### macOS

```bash
make build-mac
make run-mac
```

### Linux

```bash
make build-linux
make doctor-linux
make config-linux
make package-linux
```

## Repository Structure

```
packages/
  mac/          # macOS Swift implementation
  linux/        # Linux Hyprland implementation
scripts/        # Build and release scripts
assets/         # Icons and images
```

## Releases (macOS)

One-time setup: store notarization credentials in your keychain:

1. Generate an app-specific password at [appleid.apple.com](https://appleid.apple.com) (Sign-In and Security → App-Specific Passwords)
2. Run:

```bash
xcrun notarytool store-credentials "Mousetrap" --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID
```

Then release from `main` with a single command:

```bash
make release-mac VERSION=0.1.0
```

## Acknowledgements

Other apps I've used that haven't worked for me, but have definitely pushed me to create this app:

- [Mouseless](https://mouseless.click/). Probably my favorite one, I was subscribed for a few months. Issues: not free, not open source. I'm happy to pay but it just adds friction for new setups. Also the config was quite complex. I did achieve a good enough workflow with it but I've never liked it as much as I like Mousetrap now.
- [NoMouse](https://github.com/madanlalit/no-mouse). How on earth is it not possible to edit the default shortcut? Especially it being something so popular, for example, on code editors. Not to mention neither of the other settings can be customized. I still want to mention that it's a very nice app if you don't care about those.
- [Shortcat](https://shortcat.app/). Lovely idea! I was really excited to use it, and I did for a while. But it does not match as many items on screen as I would like it to and found myself still using the mouse a ton.
- [Superkey](https://superkey.app/). Just like Shortcat, but paid and closed source.
