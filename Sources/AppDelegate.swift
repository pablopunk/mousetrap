import AppKit
import Combine
import KeyboardShortcuts

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let overlayController = OverlayWindowController()
    private let screenResolver = FocusedScreenResolver()
    private let permissionManager = PermissionManager()
    private let settings = SettingsStore.shared
    private var settingsCancellables = Set<AnyCancellable>()
    private var previouslyFocusedApp: NSRunningApplication?
    private let keyboardLayoutResolver = KeyboardLayoutResolver()
    private let overlayKeyboardInterceptor = KeyboardInterceptor()
    private let freeMouseKeyboardInterceptor = KeyboardInterceptor()
    private let mouseMovementInterceptor = MouseMovementInterceptor()
    private let freeMouseIndicatorController = FreeMouseIndicatorController()
    private lazy var navigator = GridNavigator()
    private var pendingFreeMouseClickWorkItem: DispatchWorkItem?
    private var pendingFreeMouseClickPoint: CGPoint?
    private let freeMouseDoubleTapWindow: TimeInterval = 0.25
    private var unsafeStateTimer: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        overlayKeyboardInterceptor.onKey = { [weak self] key in
            self?.handleInterceptedKey(key)
        }
        overlayController.onKey = { [weak self] key in
            self?.handleInterceptedKey(key)
        }
        freeMouseKeyboardInterceptor.onKey = { [weak self] key in
            self?.handleFreeMouseKey(key)
        }
        mouseMovementInterceptor.onMouseMovement = { [weak self] event in
            self?.handleObservedMouseMovement(event)
        }
        mouseMovementInterceptor.start()

        KeyboardShortcuts.onKeyDown(for: .activateMousetrap) { [weak self] in
            self?.toggleOverlay()
        }
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        restoreMenuBarIconIfNeeded()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        restoreMenuBarIconIfNeeded()
        return true
    }

    func applicationWillTerminate(_ notification: Notification) {
        cancelUnsafeStateTimeout()
        mouseMovementInterceptor.stop()
    }

    private var isInUnsafeState: Bool {
        overlayController.isVisible || freeMouseKeyboardInterceptor.isActive
    }

    private func toggleOverlay() {
        if freeMouseKeyboardInterceptor.isActive {
            deactivateFreeMouseMode()
        } else if overlayController.isVisible {
            deactivateOverlay()
        } else {
            activateOverlay()
            noteUnsafeActivity()
        }
    }

    private func activateOverlay() {
        guard permissionManager.hasAccessibilityPermission else {
            permissionManager.openAccessibilitySettings()
            return
        }

        previouslyFocusedApp = NSWorkspace.shared.frontmostApplication

        let layouts = keyboardLayoutResolver.resolveLayouts()
        navigator.configureLayouts(root: layouts.root, refinement: layouts.refinement, finalClick: layouts.finalClick)

        let screen = screenResolver.resolveFocusedScreen() ?? NSScreen.main ?? NSScreen.screens.first
        guard let screen else { return }

        navigator.reset(to: screen.frame)
        overlayKeyboardInterceptor.start()
        overlayController.show(on: screen, state: navigator.state)
    }

    private func deactivateOverlay(restoreFocus: Bool = true) {
        overlayKeyboardInterceptor.stop()
        overlayController.hide()

        if !isInUnsafeState {
            cancelUnsafeStateTimeout()
        }

        guard restoreFocus, let previouslyFocusedApp else {
            if !isInUnsafeState {
                self.previouslyFocusedApp = nil
            }
            return
        }

        self.previouslyFocusedApp = nil
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
            previouslyFocusedApp.activate()
        }
    }

    private func handleInterceptedKey(_ key: InterceptedKey) {
        noteUnsafeActivity()

        switch key {
        case .escape:
            deactivateOverlay()
        case .delete:
            navigator.back()
            updateOverlayAndCursor()
        case .returnKey:
            MouseController.leftClick()
            deactivateOverlay()
        case .shiftReturnKey:
            MouseController.rightClick()
            deactivateOverlay()
        case .space:
            MouseController.leftClick()
            deactivateOverlay()
        case .upArrow, .downArrow, .leftArrow, .rightArrow, .shiftUpArrow, .shiftDownArrow, .shiftLeftArrow, .shiftRightArrow:
            startFreeMouseMode(withInitialMove: key)
        case .character(let character):
            let shouldClick = navigator.expectsClickOnNextSelection
            guard navigator.select(character) else { return }
            if shouldClick {
                let target = navigator.state.currentRect.center
                _ = MouseController.moveCursor(to: target)
                MouseController.leftClick(at: target)
                deactivateOverlay()
            } else {
                updateOverlayAndCursor()
            }
        }
    }

    private func handleFreeMouseKey(_ key: InterceptedKey) {
        noteUnsafeActivity()

        switch key {
        case .escape:
            cancelPendingFreeMouseClick()
            deactivateFreeMouseMode()
        case .returnKey, .space:
            MouseController.endLeftDrag()
            handleFreeMouseReturn()
        case .shiftReturnKey:
            MouseController.endLeftDrag()
            let clickPoint = MouseController.currentCursorPositionAppKit
            cancelPendingFreeMouseClick()
            deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.rightClick(at: clickPoint)
            }
        case .upArrow, .downArrow, .leftArrow, .rightArrow:
            MouseController.endLeftDrag()
            cancelPendingFreeMouseClick()
            if MouseController.moveFreeCursor(direction: key) {
                freeMouseIndicatorController.updatePosition(to: MouseController.currentCursorPositionAppKit)
            }
        case .shiftUpArrow, .shiftDownArrow, .shiftLeftArrow, .shiftRightArrow:
            cancelPendingFreeMouseClick()
            MouseController.beginLeftDrag()
            if MouseController.moveFreeCursor(direction: baseArrowKey(for: key)) {
                freeMouseIndicatorController.updatePosition(to: MouseController.currentCursorPositionAppKit)
            }
        case .character, .delete:
            MouseController.endLeftDrag()
            break
        }
    }

    private func startFreeMouseMode(withInitialMove key: InterceptedKey) {
        freeMouseKeyboardInterceptor.start()
        deactivateOverlay(restoreFocus: true)

        switch key {
        case .shiftUpArrow, .shiftDownArrow, .shiftLeftArrow, .shiftRightArrow:
            MouseController.beginLeftDrag()
            if MouseController.moveFreeCursor(direction: baseArrowKey(for: key)) {
                freeMouseIndicatorController.show(at: MouseController.currentCursorPositionAppKit)
            }
        default:
            if MouseController.moveFreeCursor(direction: key) {
                freeMouseIndicatorController.show(at: MouseController.currentCursorPositionAppKit)
            }
        }

        noteUnsafeActivity()
    }

    private func deactivateFreeMouseMode() {
        cancelPendingFreeMouseClick()
        MouseController.endLeftDrag()
        freeMouseKeyboardInterceptor.stop()
        freeMouseIndicatorController.hide()

        if !isInUnsafeState {
            cancelUnsafeStateTimeout()
        }
    }

    private func handleFreeMouseReturn() {
        let clickPoint = MouseController.currentCursorPositionAppKit

        if pendingFreeMouseClickWorkItem != nil {
            cancelPendingFreeMouseClick()
            deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.doubleClick(at: clickPoint)
            }
            return
        }

        pendingFreeMouseClickPoint = clickPoint
        let workItem = DispatchWorkItem { [weak self] in
            guard let self, let clickPoint = self.pendingFreeMouseClickPoint else { return }
            self.pendingFreeMouseClickWorkItem = nil
            self.pendingFreeMouseClickPoint = nil
            self.deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.leftClick(at: clickPoint)
            }
        }

        pendingFreeMouseClickWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + freeMouseDoubleTapWindow, execute: workItem)
    }

    private func cancelPendingFreeMouseClick() {
        pendingFreeMouseClickWorkItem?.cancel()
        pendingFreeMouseClickWorkItem = nil
        pendingFreeMouseClickPoint = nil
    }

    private func updateOverlayAndCursor() {
        overlayController.update(state: navigator.state)
        MouseController.moveCursor(to: navigator.state.currentRect.center)
    }

    private func baseArrowKey(for key: InterceptedKey) -> InterceptedKey {
        switch key {
        case .shiftUpArrow:
            .upArrow
        case .shiftDownArrow:
            .downArrow
        case .shiftLeftArrow:
            .leftArrow
        case .shiftRightArrow:
            .rightArrow
        default:
            key
        }
    }

    private func noteUnsafeActivity() {
        guard isInUnsafeState else { return }

        unsafeStateTimer?.invalidate()
        unsafeStateTimer = Timer.scheduledTimer(withTimeInterval: TimeInterval(settings.unsafeStateTimeoutSeconds), repeats: false) { [weak self] _ in
            Task { @MainActor in
                self?.resetToSafeStateDueToTimeout()
            }
        }
    }

    private func handleObservedMouseMovement(_ event: CGEvent) {
        guard !MouseController.shouldIgnoreObservedMouseMovement(event) else { return }
        MouseController.clearTrackedCursorPosition()
        guard isInUnsafeState else { return }

        print("[Mousetrap] mouse movement detected, resetting to safe state")
        deactivateFreeMouseMode()
        deactivateOverlay()
    }

    private func cancelUnsafeStateTimeout() {
        unsafeStateTimer?.invalidate()
        unsafeStateTimer = nil
    }

    private func restoreMenuBarIconIfNeeded() {
        guard UserDefaults.standard.bool(forKey: "showMenuBarIcon") == false else { return }
        UserDefaults.standard.set(true, forKey: "showMenuBarIcon")
    }

    private func resetToSafeStateDueToTimeout() {
        guard isInUnsafeState else {
            cancelUnsafeStateTimeout()
            return
        }

        print("[Mousetrap] inactivity timeout reached, resetting to safe state")
        deactivateFreeMouseMode()
        deactivateOverlay()
    }
}
