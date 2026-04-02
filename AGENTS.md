# Rules for Agents

## Follow these rules

Not following these rules have caused bugs in the past, so please do:

* Overlay windows/panels must remain non-activating (`NSPanel` + non-activating behavior); showing the grid must not make Mousetrap the focused app or collapse transient UI in other apps.
* Keyboard input for grid/free-mouse flows must be handled through the global event tap / `KeyboardInterceptor`, not by relying on the overlay window becoming key/main.
* Programmatic cursor movement and drag events must be marked as synthetic and explicitly ignored by mouse-movement observers to avoid cancelling Mousetrap from its own events.
* Any free-mouse state transition must preserve teardown invariants in order: cancel pending click, end active drag, stop interceptors/indicators as needed, and clear unsafe-state timers when returning to safe state.
* Cursor movement, clamping, and target selection must be correct across multiple displays, including focused-window screen resolution, per-display coordinates, and screen-edge behavior.
* Hiding the menu bar icon must never make the app unreachable; reopening or reactivating the app must restore `showMenuBarIcon` / `MenuBarExtra` visibility.

