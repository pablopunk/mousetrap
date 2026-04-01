import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private var statusItem: NSStatusItem!
    private let overlayController = OverlayWindowController()
    private let screenResolver = FocusedScreenResolver()
    private let permissionManager = PermissionManager()
    private var previouslyFocusedApp: NSRunningApplication?
    private let keyboardLayoutResolver = KeyboardLayoutResolver()
    private lazy var navigator = GridNavigator()
    private lazy var hotKeyController = GlobalHotKeyController { [weak self] in
        self?.toggleOverlay()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        overlayController.onKey = { [weak self] key in
            self?.handleInterceptedKey(key)
        }
        setupMenuBar()
        _ = hotKeyController
    }

    func applicationWillTerminate(_ notification: Notification) {
    }

    private func setupMenuBar() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        statusItem.button?.image = NSImage(systemSymbolName: "cursorarrow.click", accessibilityDescription: "Mousetrap")
        statusItem.button?.image?.isTemplate = true
        statusItem.button?.title = ""
        statusItem.button?.toolTip = "Mousetrap"

        let menu = NSMenu()
        menu.delegate = self
        statusItem.menu = menu
        rebuildMenu()
    }

    func menuWillOpen(_ menu: NSMenu) {
        rebuildMenu()
    }

    private func rebuildMenu() {
        guard let menu = statusItem.menu else { return }
        menu.removeAllItems()

        if !permissionManager.hasAccessibilityPermission {
            let item = NSMenuItem(title: "Open Accessibility Permissions…", action: #selector(openAccessibilityPermissions), keyEquivalent: "")
            item.attributedTitle = NSAttributedString(
                string: "Open Accessibility Permissions…",
                attributes: [.foregroundColor: NSColor.systemOrange]
            )
            menu.addItem(item)
            menu.addItem(.separator())
        }

        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }

    @objc private func openAccessibilityPermissions() {
        permissionManager.openAccessibilitySettings()
    }

    private func toggleOverlay() {
        if overlayController.isVisible {
            deactivateOverlay()
        } else {
            activateOverlay()
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
        overlayController.show(on: screen, state: navigator.state)
    }

    private func deactivateOverlay() {
        overlayController.hide()
        if let previouslyFocusedApp {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                previouslyFocusedApp.activate()
            }
        }
    }

    private func handleInterceptedKey(_ key: InterceptedKey) {
        switch key {
        case .escape:
            deactivateOverlay()
        case .delete:
            navigator.back()
            updateOverlayAndCursor()
        case .returnKey:
            MouseController.leftClick()
            deactivateOverlay()
        case .space:
            MouseController.leftClick()
            deactivateOverlay()
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

    private func updateOverlayAndCursor() {
        overlayController.update(state: navigator.state)
        MouseController.moveCursor(to: navigator.state.currentRect.center)
    }
}
