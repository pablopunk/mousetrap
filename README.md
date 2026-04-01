# Mousetrap

> Get it? It kills the mouse.



<p align="center">
  <img width="128" height="128" alt="icon" src="https://github.com/user-attachments/assets/906a66a7-0c1d-4243-97ad-2e1de187e494" />
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

The release script will automatically:

- use your only installed `Developer ID Application` signing identity
- build, sign, notarize, and staple the app
- create and push the release commit + tag
- create the GitHub release and upload the zip
- update the Homebrew tap if `GH_PAT` is set

If you have multiple `Developer ID Application` certificates installed, set `CODESIGN_IDENTITY` explicitly before running the release.

## Acknowledgements

Other apps I've tried to do this, but haven't worked for me:

- [Mouseless](https://mouseless.click/). Probably my favorite one, I was subscribed for a few months. Issues: not free, not open source. I'm happy to pay but it just adds friction for new setups. Also the config was quite complex. I did achieve a good enough workflow with it but I've never liked it as much as I like Mousetrap now.
- [NoMouse](https://github.com/madanlalit/no-mouse). How on earth is it not possible to edit the default shortcut? Especially it being something so popular, for example, on code editors. Not to mention neither of the other settings can be customized. I still want to mention that it's a very nice app if you don't care about those.
- [Shortcat](https://shortcat.app/). Lovely idea! I was really excited to use it, and I did for a while. But it does not match as many items on screen as I would like it to and found myself still using the mouse a ton.
- [Superkey](https://superkey.app/). Just like Shortcat, but paid and closed source.
