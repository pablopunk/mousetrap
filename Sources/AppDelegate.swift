import AppKit
import Combine
import KeyboardShortcuts

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private static let unsafeStateTimeout: TimeInterval = 10

    private let overlayController = OverlayWindowController()
    private let screenResolver = FocusedScreenResolver()
    private let permissionManager = PermissionManager()
    private let settings = SettingsStore.shared
    private var settingsCancellables = Set<AnyCancellable>()
    private var previouslyFocusedApp: NSRunningApplication?
    private let keyboardLayoutResolver = KeyboardLayoutResolver()
    private let overlayKeyboardInterceptor = KeyboardInterceptor()
    private let freeMouseKeyboardInterceptor = KeyboardInterceptor()
    private let freeMouseIndicatorController = FreeMouseIndicatorController()
    private lazy var navigator = GridNavigator()
    private var unsafeStateTimer: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        overlayKeyboardInterceptor.onKey = { [weak self] key in
            self?.handleInterceptedKey(key)
        }
        freeMouseKeyboardInterceptor.onKey = { [weak self] key in
            self?.handleFreeMouseKey(key)
        }

        KeyboardShortcuts.onKeyDown(for: .activateMousetrap) { [weak self] in
            self?.toggleOverlay()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        cancelUnsafeStateTimeout()
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
        case .upArrow, .downArrow, .leftArrow, .rightArrow:
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
            deactivateFreeMouseMode()
        case .returnKey:
            let clickPoint = MouseController.currentCursorPositionAppKit
            deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.leftClick(at: clickPoint)
            }
        case .shiftReturnKey:
            let clickPoint = MouseController.currentCursorPositionAppKit
            deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.rightClick(at: clickPoint)
            }
        case .upArrow, .downArrow, .leftArrow, .rightArrow:
            if MouseController.moveFreeCursor(direction: key) {
                freeMouseIndicatorController.updatePosition(to: MouseController.currentCursorPositionAppKit)
            }
        case .character, .delete, .space:
            break
        }
    }

    private func startFreeMouseMode(withInitialMove key: InterceptedKey) {
        freeMouseKeyboardInterceptor.start()
        deactivateOverlay(restoreFocus: false)
        if MouseController.moveFreeCursor(direction: key) {
            freeMouseIndicatorController.show(at: MouseController.currentCursorPositionAppKit)
        }
        noteUnsafeActivity()
    }

    private func deactivateFreeMouseMode() {
        freeMouseKeyboardInterceptor.stop()
        freeMouseIndicatorController.hide()

        if !isInUnsafeState {
            cancelUnsafeStateTimeout()
        }
    }

    private func updateOverlayAndCursor() {
        overlayController.update(state: navigator.state)
        MouseController.moveCursor(to: navigator.state.currentRect.center)
    }

    private func noteUnsafeActivity() {
        guard isInUnsafeState else { return }

        unsafeStateTimer?.invalidate()
        unsafeStateTimer = Timer.scheduledTimer(withTimeInterval: Self.unsafeStateTimeout, repeats: false) { [weak self] _ in
            Task { @MainActor in
                self?.resetToSafeStateDueToTimeout()
            }
        }
    }

    private func cancelUnsafeStateTimeout() {
        unsafeStateTimer?.invalidate()
        unsafeStateTimer = nil
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
