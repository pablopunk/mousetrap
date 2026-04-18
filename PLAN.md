# Linux / Hyprland Port Plan

This document captures:

- what Mousetrap does on macOS today
- what we explored and proved during the first Hyprland porting session
- the current Linux/Hyprland architecture in this repo
- the gap to full feature parity with macOS
- the proposed implementation roadmap

## Repository Structure

The repository is organized as an OS-agnostic project with platform-specific packages:

```
packages/
  mac/          # macOS Swift implementation (Swift Package Manager project)
  linux/        # Linux Hyprland implementation (Python package)
scripts/        # Build and release scripts
assets/         # Shared icons and images
VERSION         # Shared version file
```

## Goal

Build a Linux version of Mousetrap with a Hyprland-first backend, while keeping the architecture clean enough to eventually support additional Linux compositors or a more native Wayland backend later.

The practical target is:

1. ship a good Hyprland version first
2. reach feature parity with the macOS app as much as Hyprland/Wayland allows
3. replace temporary external dependencies where it makes sense

---

## Original macOS Mousetrap feature set

The current macOS app provides the following core behavior:

1. **Global activation shortcut**
   - configurable hotkey
   - available system-wide

2. **Non-activating fullscreen overlay**
   - translucent overlay
   - does not steal app focus in the normal macOS design
   - keyboard handling is done through the event tap / interceptor path

3. **Keyboard-layout-aware grid**
   - grid keys reflect the real keyboard layout
   - layout can vary per user/keyboard configuration

4. **Three-step nested grid refinement**
   - first grid narrows the area
   - second grid refines it more
   - third grid allows final click precision

5. **Chord targeting**
   - pressing adjacent keys such as `zx` or `aszx` targets edges, midpoints, or corners between cells
   - supported across refinement levels

6. **Click on final selection**
   - final target results in a click at the computed point

7. **Free mouse mode**
   - arrow keys move the cursor
   - Enter / Space click
   - double-click
   - right-click
   - drag with modifier + arrows

8. **Timeout / reset safety behavior**
   - if the user pauses too long or moves the real mouse, Mousetrap resets to a safe state

9. **Pulse / reveal options**
   - optional visual pulsing / fade to help see underlying UI

10. **Active-window / focused-screen targeting**
    - prefers the focused window / relevant screen rather than blindly using the whole desktop

11. **Multi-monitor correctness**
    - screen-aware cursor movement and clamping
    - correct coordinate handling across displays

12. **Settings / menu bar app behavior**
    - menu bar presence
    - launch at login
    - settings for shortcut, pulse, mouse travel, timeout, etc.

---

## What we learned during the first Linux / Hyprland session

### High-level conclusion

A Linux port is very feasible for **Hyprland specifically**.

A generic “Linux” port is much more ambiguous because global input, overlays, and synthetic pointer injection vary heavily across compositors and Wayland security models.

For Hyprland, the most promising approach is:

- use **Hyprland binds/submaps** for input capture
- use **GTK4 + gtk4-layer-shell** for the overlay
- use **Hyprland IPC** for monitor/window geometry and cursor warp
- use **ydotool** for synthetic click initially
- keep the core grid/session logic platform-agnostic where possible

### Specific things we proved

We built and validated the following during this session:

1. **Hyprland IPC is available and usable**
   - `hyprctl -j monitors`
   - `hyprctl -j activewindow`
   - `hyprctl dispatch movecursor`

2. **Cursor warp works**
   - we successfully moved the cursor to the active window center and later to selected grid cells

3. **A translucent fullscreen overlay works on Hyprland**
   - using GTK4 + layer-shell
   - transparency required explicit transparent drawing behavior

4. **Overlay keyboard capture was unreliable**
   - trying to make the layer-shell overlay directly own the keypress path was not robust enough
   - this pushed us toward Hyprland-owned key capture instead

5. **Hyprland submap-driven key routing works**
   - activation enters a submap
   - grid keys dispatch selection commands
   - escape cancels

6. **Dynamic runtime submap registration via `hyprctl` is possible**
   - useful for fast iteration / experimentation
   - static sourced config is still cleaner long-term

7. **Synthetic click works with `ydotool`**
   - after installing `ydotool`
   - after running the user service

8. **Timing matters**
   - successful `dismiss overlay -> move cursor -> click` behavior required short delays between phases

9. **Active-window-first geometry is important**
   - selecting within the active window bounds instead of the full monitor fixed targeting and clicking behavior

10. **End-to-end prototype works**
   - activate
   - show overlay
   - press/select a key via Hyprland submap
   - dismiss overlay
   - move cursor
   - click underlying app

---

## Current Linux / Hyprland state in the repo

All Linux work lives under `packages/linux/`.

### Current files and roles

- `packages/linux/pyproject.toml`
  - Python package metadata for the Hyprland implementation

- `packages/linux/README.md`
  - Linux/Hyprland setup and status notes

- `packages/linux/mousetrap.conf`
  - example Hyprland config with static binds/submap

- `packages/linux/activate.sh`
  - activate overlay and runtime bind flow

- `packages/linux/select.sh`
  - select a key through CLI

- `packages/linux/cancel.sh`
  - cancel submap and overlay

- `packages/linux/dynamic_bind.sh`
  - dynamically register the temporary submap/binds via `hyprctl`

- `packages/linux/cursor_center.sh`
  - helper / diagnostic cursor centering script

- `packages/linux/overlay.py`
  - thin launcher for the Python package

- `packages/linux/mousetrap_hyprland/config.py`
  - layout and app constants

- `packages/linux/mousetrap_hyprland/core.py`
  - basic grid math and key->cell lookup

- `packages/linux/mousetrap_hyprland/session.py`
  - simple overlay session / key resolution logic

- `packages/linux/mousetrap_hyprland/hyprctl.py`
  - Hyprland IPC adapter

- `packages/linux/mousetrap_hyprland/geometry.py`
  - active-window-first geometry resolution

- `packages/linux/mousetrap_hyprland/settings.py`
  - persisted user config under XDG config

- `packages/linux/mousetrap_hyprland/diagnostics.py`
  - runtime dependency and environment checks

- `packages/linux/mousetrap_hyprland/binds.py`
  - centralized dynamic Hyprland bind/submap management

- `packages/linux/mousetrap_hyprland/clicking.py`
  - click backend abstraction, currently via `ydotool`

- `packages/linux/mousetrap_hyprland/actions.py`
  - higher-level actions such as `move_and_click`

- `packages/linux/mousetrap_hyprland/launcher.py`
  - detached overlay launch helper

- `packages/linux/mousetrap_hyprland/cli.py`
  - main CLI entrypoint for activate / overlay / select / cancel

- `packages/linux/mousetrap_hyprland/overlay.py`
  - GTK4 layer-shell fullscreen overlay renderer

- `packages/linux/mousetrap_hyprland/app.py`
  - simple package app entrypoint

### What currently works

1. transparent fullscreen overlay on Hyprland
2. active-window-first target geometry with monitor fallback
3. submap-driven single-key selection
4. cursor warp to selected cell center
5. synthetic left click through `ydotool`
6. activation / cancel / select CLI flow
7. dynamic runtime bind experimentation through `hyprctl`
8. user config file and `doctor` checks
9. Arch-native packaging scaffolding and AUR-friendly `-git` packaging

### Current dependencies

#### Python package dependencies

- `PyGObject`
- `pycairo`

#### System dependencies

- `python`
- `gtk4`
- `gtk4-layer-shell`
- `python-gobject`
- `python-cairo`
- `jq`
- `hyprland`
- `ydotool` (currently required for clicking)

---

## Architectural direction

The Linux implementation should stay split into two layers:

### 1. Portable core logic

This should contain:

- grid layouts
- selection state
- refinement state machine
- chord resolution
- free-mouse state machine
- action scheduling / timeout policy

This layer should avoid GTK, Hyprland IPC, and shell commands as much as possible.

### 2. Hyprland backend

This should contain:

- overlay window implementation
- Hyprland bind/submap integration
- Hyprland IPC and geometry lookup
- cursor warping
- click backend integration
- config loading and runtime scripts

This separation matters because the current prototype already showed that compositor integration and core targeting logic evolve at different speeds.

---

## Gap analysis: macOS parity vs current Hyprland prototype

### Already achieved or partially achieved

- [x] fullscreen overlay
- [x] translucent overlay
- [x] active-window-first targeting
- [x] single-key targeting
- [x] move cursor to target
- [x] click target
- [x] activation / cancel flow in Hyprland
- [x] basic multi-monitor awareness through monitor/window geometry

### Not yet implemented

- [ ] configurable/global activation abstraction beyond current Hyprland bind flow
- [ ] keyboard-layout-aware grid on Linux
- [x] 3-step nested refinement
- [ ] chord targeting (`zx`, `aszx`, etc.)
- [ ] free-mouse mode
- [ ] double-click
- [ ] right-click
- [ ] drag mode
- [ ] inactivity timeout / safe reset logic
- [ ] real mouse-movement cancellation behavior
- [ ] pulse / reveal visual options
- [ ] settings UI
- [ ] launch-at-login / autostart UX
- [~] polished packaging / install flow
- [ ] replacement of `ydotool` with a native pointer backend
- [ ] broader Linux compositor support

---

## Proposed roadmap

## Phase 0: finish stabilizing the current Hyprland MVP

### Goals

Make the current single-step Linux path solid enough that parity work does not stack on top of flaky behavior.

### Tasks

1. **Lock the control plane**
   - keep `mousetrap_hyprland.cli` as the canonical entrypoint
   - keep shell scripts as thin wrappers only
   - avoid duplicating bind logic outside Python

2. **Decide and codify the default activation mode**
   - prefer static sourced Hyprland config if it proves more reliable
   - keep dynamic binds as dev/fallback mode
   - make docs/config reflect the default clearly

3. **Harden teardown and cancel invariants**
   - always reset submap
   - always stop overlay cleanly
   - avoid stale pidfile / stale bind state
   - make repeated activate/cancel/select cycles safe

4. **Tighten click reliability**
   - verify overlay-dismiss -> warp -> click timing
   - make backend errors actionable
   - keep `ydotool` as current default backend

5. **Geometry and multi-monitor validation**
   - verify active-window-first behavior across outputs
   - check partially off-screen windows
   - validate monitor fallback when no active mapped window exists

6. **Packaging/install hygiene**
   - keep Arch package path as preferred Linux install path
   - keep AUR `mousetrap-hyprland-git` recipe current until the first Linux-capable stable tag exists

### Exit criteria

- activation/cancel/select are reliable in repeated use
- config and doctor flow are stable
- packaging/docs are good enough that Linux feature work can proceed without install churn

---

## Phase 1: implement parity-critical interaction model

### Goals

Port the real Mousetrap interaction model, starting with the feature that unlocks the most parity fastest: **three-step nested refinement**.

### Tasks

1. **Design a Linux session state machine**
   - session starts with target bounds from active window or monitor fallback
   - each selection produces a refined rectangle
   - session tracks current step, current bounds, and selection history
   - final step resolves to a click point

2. **Refactor portable core logic**
   - separate single-step grid lookup from multi-step session progression
   - move refinement math into platform-agnostic modules
   - keep Hyprland-specific code limited to bounds lookup, overlay launch, and pointer actions

3. **Upgrade overlay rendering to session-aware UI**
   - render only the current refinement bounds
   - show current step visually
   - keep labels readable as the bounds shrink
   - maintain non-activating overlay behavior

4. **Implement reset/cancel/timeout rules**
   - Esc cancels at any stage
   - timeout resets safely
   - invalid key sequences do not leave broken state behind

5. **End-to-end validation before new click types**
   - three-step refine -> final click must work reliably
   - do not add right-click/double-click/drag until the core refine flow is solid

### Exit criteria

- Linux supports three-step refinement
- overlay follows the current refinement rectangle cleanly
- cancel/timeout/reset behavior is predictable

---

## Phase 2: expand target semantics

### Goals

Match more of the macOS targeting precision and interaction vocabulary.

### Tasks

1. **Chord targeting**
   - support adjacent-key combinations such as `zx` and `aszx`
   - resolve edges, corners, and midpoints where practical
   - keep scoring/selection rules deterministic

2. **Keyboard/layout fidelity**
   - move beyond hardcoded US-QWERTY assumptions
   - support configurable key rows first
   - later investigate real layout-aware behavior closer to macOS

3. **Overlay previews for advanced selection**
   - show chord preview / final point preview when useful
   - preserve readability and low visual noise

### Exit criteria

- users can target more precisely than simple cell centers
- grid definition is no longer rigidly tied to one layout assumption

---

## Phase 3: mouse actions parity

### Goals

Move from “final left click only” to a richer set of pointer actions.

### Tasks

1. **Generalize the click backend API**
   - left click
   - right click
   - double click
   - drag start / move / end

2. **Add keyboard-driven action mappings**
   - Enter / Space click
   - explicit right-click trigger
   - explicit double-click trigger

3. **Preserve state cleanup invariants**
   - no stuck drag
   - no repeated pending clicks
   - clear safe return path on cancel

### Exit criteria

- Linux supports the primary pointer actions Mousetrap users expect
- click backend abstractions are ready for future native replacement

---

## Phase 4: free-mouse mode parity

### Goals

Support arrow/key-driven mouse movement in addition to grid targeting.

### Tasks

1. **Free-mouse session state**
   - enter/exit mode predictably
   - separate it cleanly from grid refine state

2. **Keyboard-driven pointer movement**
   - configurable movement step size
   - later possibly acceleration behavior

3. **Actions while in free-mouse mode**
   - click
   - double click
   - right click
   - drag

4. **Safety rules**
   - cleanup ordering on mode switches
   - timeout/reset semantics
   - ignore synthetic motion where needed to avoid self-cancellation patterns

### Exit criteria

- free-mouse mode is practical for real use
- switching between modes is safe and predictable

---

## Phase 5: polish and daily-driver quality

### Goals

Close the remaining usability and robustness gaps with the macOS app.

### Tasks

1. **Pulse / reveal visuals**
   - optional visual pulsing
   - better visibility controls

2. **Multi-monitor correctness audit**
   - clamping at edges
   - output selection correctness
   - window/screen transform correctness

3. **Timeout and recovery polish**
   - safe reset from all active states
   - actionable diagnostics when backend actions fail

4. **Autostart and settings polish**
   - clean docs for Hyprland startup integration
   - optionally a lightweight settings/helper UI later

### Exit criteria

- Linux Mousetrap feels robust enough for daily use
- most remaining gaps are intentional platform differences, not missing basics

---

## Phase 6: backend quality ceiling work

### Goals

Reduce long-term fragility from the current click backend and compositor coupling where possible.

### Tasks

1. **Keep `ydotool` as the short-term default**
   - do not block parity work on backend replacement

2. **Investigate native pointer injection options**
   - Hyprland/Wayland virtual pointer paths
   - small native helper if needed

3. **Re-evaluate packaging once backend strategy is clearer**
   - stable Arch package once a Linux-capable release tag exists
   - possibly broader distro support later

### Exit criteria

- we understand whether replacing `ydotool` is worth it
- backend constraints are no longer a hidden risk to feature quality

---

## Phase 1: stabilize the current Hyprland MVP

### Goals

Turn the working prototype into a clean, reliable single-level click tool.

### Tasks

1. **Clean activation flow**
   - choose between static sourced config and dynamic binds for the default UX
   - likely keep static config as preferred, dynamic binds as dev fallback

2. **Consolidate CLI / scripts**
   - reduce script duplication
   - make `cli.py` the canonical control surface

3. **Improve error handling**
   - clear error messages when `ydotool` or GTK deps are missing
   - better diagnostics if Hyprland IPC fails

4. **Tighten timing behavior**
   - make timings configurable
   - tune for reliable click delivery

5. **Document installation clearly**
   - especially `ydotool` setup and user service requirements

### Exit criteria

- activation is reliable
- single-key targeting and click is reliable
- cancellation is reliable
- setup instructions are straightforward

---

## Phase 2: reach basic Mousetrap interaction parity

### Goals

Port the actual Mousetrap targeting model, not just a one-shot cell click.

### Tasks

1. **Port grid navigator / refinement logic**
   - three refinement levels
   - current-rect / history state
   - final click stage

2. **Implement chord targeting**
   - adjacent keys
   - midpoint / corner targeting
   - parity with macOS selection scoring semantics where practical

3. **Overlay rendering updates**
   - render the current refinement rect rather than always full-monitor grid
   - render previews for chord selection and final point

4. **Target active window or monitor correctly at each level**
   - preserve correct coordinate transforms and bounds clipping

### Exit criteria

- users can refine target area across multiple levels
- users can click with single keys or supported chords

---

## Phase 3: free-mouse mode parity

### Goals

Support arrow-driven mouse movement and richer pointer actions.

### Tasks

1. **Hyprland submap for free-mouse mode**
   - arrow movement
   - exit behavior

2. **Mouse actions**
   - left click
   - double-click
   - right-click
   - drag start / drag move / drag end

3. **Configurable step size**
   - equivalent to macOS mouse travel setting

4. **State handling**
   - pending click / double-click window
   - drag state
   - cancel behavior

### Exit criteria

- free-mouse mode is practical and reliable
- drag and click semantics are predictable

---

## Phase 4: safety and polish parity

### Goals

Bring the Linux version closer to the robustness of the macOS app.

### Tasks

1. **Timeout handling**
   - inactivity reset
   - configurable timeout

2. **Real mouse movement cancellation**
   - if feasible in Hyprland without fighting synthetic events

3. **Pulse / reveal options**
   - animation and visibility controls

4. **Multi-monitor correctness audit**
   - edge clamping
   - active-window monitor choice
   - cross-output behavior

5. **Config file**
   - store timings, step size, pulse flags, activation behavior, etc.

### Exit criteria

- Linux behavior feels robust enough for daily use
- failures are recoverable and easy to understand

---

## Phase 5: productization for Linux users

### Goals

Make the Linux version easy to install and maintain.

### Tasks

1. **Settings UI**
   - likely GTK-based
   - configure timings, mode, activation bind guidance, pulse, step size

2. **Autostart integration**
   - user autostart entry / desktop integration

3. **Packaging**
   - Arch package or PKGBUILD first
   - possibly Flatpak later if compatible with Hyprland workflow constraints

4. **Dependency strategy**
   - keep `ydotool` short-term
   - evaluate a native Wayland / virtual-pointer backend long-term

5. **README / docs refresh**
   - mention Linux support clearly once mature enough

### Exit criteria

- another Hyprland user can install and use it without hand-holding

---

## Long-term technical decision: `ydotool` vs native pointer backend

### Short-term

Keep `ydotool` as the click backend because it already works and unblocks feature development.

### Long-term

Investigate replacing it with a native backend if one of these becomes worthwhile:

- direct Wayland virtual pointer client implementation
- a Hyprland-specific IPC/protocol path
- a Rust/C helper integrated with the Python frontend/core

### Decision criteria

Replace `ydotool` only if it provides at least one of:

- simpler setup
- fewer runtime dependencies
- better reliability
- cleaner drag/right-click/double-click semantics

---

## Open questions

1. Should the default UX rely on:
   - a static sourced Hyprland config, or
   - dynamic runtime binds?

2. Should the Linux implementation remain Python + GTK, or later move to:
   - Rust + GTK,
   - Rust + a smaller Wayland client,
   - or another stack?

3. How close can we get to “non-activating” macOS semantics on Hyprland while keeping the UX simple?

4. When should we stop calling this a POC and start treating it as a supported Linux target?

---

## Summary

At the end of today’s work, we have a real Hyprland prototype that can:

- show a translucent fullscreen overlay
- enter a Hyprland key-selection mode
- target the active window area
- move the cursor to the chosen key cell
- click the underlying application

That is enough proof that a Hyprland-first Linux Mousetrap is viable.

The next major milestone is to port the real Mousetrap interaction model — multi-step refinement, chord targeting, and free-mouse mode — on top of this working backend.
