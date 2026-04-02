# Mousetrap

> Get it? It kills the mouse.



<p align="center">
  <img width="128" height="128" alt="icon" src="https://github.com/pablopunk/mousetrap/blob/main/assets/AppIcon.png?raw=true" />
</p>
<p align="center">
<span>Touch your screen. But not really.</span>
<img width="100%" alt="explain" src="https://github.com/pablopunk/mousetrap/blob/main/assets/keyboard-keys-screen.png?raw=true" />
</p>

https://github.com/user-attachments/assets/1eb0b6e0-582b-4748-bb40-ceda9cb46842



## Install

```bash
brew install pablopunk/brew/mousetrap
```

Or grab the latest release from the [releases page](https://github.com/pablopunk/mousetrap/releases).

## Features

- ⚡ Native, lightweight, fast. Lives in your menu bar.
- 💅 Custom keyboard shortcuts.
- 🖱️ Free mouse mode. Use your arrow keys to move the cursor, click, and drag with as much precision as you want.
- 👨‍💻 Free and open source.

## Usage

- Use any keyboard shortcut to trigger the fullscreen grid.
- Each key will move you to that key's cell, and will divide it again.
- Each key press will move the mouse to the center of the cell.
- There are 3 nested grids you can hit your target.
- At any point you can hit any arrow key to move the mouse freely, hit Enter to click, or use Shift to drag.

## Development

```bash
make build
make run
```

## Releases

One-time setup: store notarization credentials in your keychain:

1. Generate an app-specific password at [appleid.apple.com](https://appleid.apple.com) (Sign-In and Security → App-Specific Passwords)
2. Run:

```bash
xcrun notarytool store-credentials "Mousetrap" --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID
```

It will prompt you for the app-specific password.

Then release from `main` with a single command:

```bash
make release VERSION=0.1.0
```

## Acknowledgements

Other apps I've used that haven't worked for me, but have definitely pushed me to create this app:

- [Mouseless](https://mouseless.click/). Probably my favorite one, I was subscribed for a few months. Issues: not free, not open source. I'm happy to pay but it just adds friction for new setups. Also the config was quite complex. I did achieve a good enough workflow with it but I've never liked it as much as I like Mousetrap now.
- [NoMouse](https://github.com/madanlalit/no-mouse). How on earth is it not possible to edit the default shortcut? Especially it being something so popular, for example, on code editors. Not to mention neither of the other settings can be customized. I still want to mention that it's a very nice app if you don't care about those.
- [Shortcat](https://shortcat.app/). Lovely idea! I was really excited to use it, and I did for a while. But it does not match as many items on screen as I would like it to and found myself still using the mouse a ton.
- [Superkey](https://superkey.app/). Just like Shortcat, but paid and closed source.
