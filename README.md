# Mousetrap

Minimal macOS keyboard-driven cursor navigator.

## Current prototype

- Menu bar app (`LSUIElement`)
- Hard-coded global hotkey: `⌘⇧Space`
- Overlays the monitor of the frontmost/focused app window when possible
- Draws a full keyboard-like grid on first activation
- After the first selection, switches to a smaller refinement grid
- After the second selection, switches to a final compact click grid
- Selecting a cell in that final grid moves the cursor there and clicks automatically
- Moves the cursor to the center of the selected cell
- `Delete` goes one level back
- `Esc` cancels
- `Space` or `Return` clicks
- Menu bar only has `Quit`

## Build

```bash
cd ~/src/mousetrap
./scripts/build-app.sh
```

App bundle output is staged at:

```bash
.build/debug/Mousetrap.app
```

Then installed to a stable path for macOS permissions:

```bash
~/Applications/Mousetrap.app
```

Run it:

```bash
./scripts/run-app.sh
```

## Permissions

You will likely need to grant **Accessibility** permissions to `Mousetrap.app` in:

- System Settings → Privacy & Security → Accessibility

The app uses accessibility APIs to:

- resolve the focused window/screen
- move the cursor
- synthesize clicks
- intercept keys while the overlay is active

## Hard-coded assumptions

This first pass is intentionally minimal:

- Full first-level layout plus a hard-coded smaller refinement layout
- No settings UI yet
- No custom hotkey recorder
- No per-layout support
- No multi-step click/drag modes yet

## Repo notes from public projects

I checked a few public repos for implementation patterns:

- `mjrusso/scoot`
  - Swift/AppKit menu bar app
  - grid + element navigation
  - uses `KeyboardShortcuts` and transparent overlay windows
- `madanlalit/no-mouse`
  - Swift menu bar app
  - uses a `CGEvent` tap for active keyboard interception
  - uses transparent overlay windows and `CGWarpMouseCursorPosition`
- `protortyp/mTile`
  - Swift/AppKit menu bar utility
  - uses Carbon `RegisterEventHotKey` for a reliable global toggle
  - uses non-activating overlay panels
- `mikker/LeaderKey`
  - Swift menu bar launcher
  - useful reference for lightweight menu bar app structure

That informed this bootstrap:

- Carbon hotkey for `⌘⇧Space`
- `CGEvent.tapCreate` only while overlay is active
- borderless transparent overlay window
- minimal menu bar app with no normal window

## Likely next steps

1. Better focused-screen detection
2. More refined key layout for deeper zoom levels
3. Click variants: right click, drag, scroll
4. Config file / settings UI
5. Better rendering and animation
6. Proper signed app bundle / Xcode project if wanted
