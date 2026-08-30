# Linux Feature TODO

| Priority | Feature cluster | Includes | Status |
|---:|---|---|---|
| 1 | **Free mouse mode** | Free mode, arrow movement, double-click, right-click, dragging, cursor indicator, movement safety reset, configurable movement step | ✅ |
| 2 | **Keyboard/input parity** | Active keyboard-layout detection, `Delete` back navigation | ❌ |
| 3 | **Chord and grid visuals** | Chord target preview, per-level pulse/reveal animation | ❌ |
| 4 | **Settings and preferences** | Native settings UI, timeout settings UI, hide/show tray icon | Partial |
| 5 | **Input safety plumbing** | Synthetic-event filtering | Partial |
| 6 | **Application polish** | About/version screen | ❌ |

## Recommended implementation order

1. **Free mouse mode** as one complete feature: movement, clicks, dragging, safety, indicator, and step configuration.
2. **Keyboard/input parity**, especially layout-aware mapping.
3. **Chord and grid visuals**.
4. **Settings and preferences** (the generic GTK window is now in place; hide/show tray icon remains).
5. **Application polish**.

Synthetic-event filtering should be implemented as part of free-mouse safety only if Linux adds a mouse-movement observer.
