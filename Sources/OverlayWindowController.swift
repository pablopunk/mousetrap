import AppKit

@MainActor
final class OverlayWindowController {
    private var window: OverlayPanel?
    private var overlayView: OverlayView?
    var onKey: ((InterceptedKey) -> Void)?

    var isVisible: Bool {
        window?.isVisible == true
    }

    func show(on screen: NSScreen, state: GridState) {
        if window == nil {
            createWindow(on: screen, state: state)
        }

        window?.setFrame(screen.frame, display: true)
        window?.onKey = onKey
        overlayView?.state = state
        overlayView?.frame = CGRect(origin: .zero, size: screen.frame.size)
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
    }

    func update(state: GridState) {
        overlayView?.state = state
        overlayView?.needsDisplay = true
    }

    func hide() {
        window?.orderOut(nil)
    }

    private func createWindow(on screen: NSScreen, state: GridState) {
        let window = OverlayPanel(
            contentRect: screen.frame,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        window.level = .screenSaver
        window.backgroundColor = .clear
        window.isOpaque = false
        window.hasShadow = false
        window.ignoresMouseEvents = true
        window.hidesOnDeactivate = false
        window.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]
        window.onKey = onKey

        let overlayView = OverlayView(frame: CGRect(origin: .zero, size: screen.frame.size))
        overlayView.state = state
        window.contentView = overlayView

        self.window = window
        self.overlayView = overlayView
    }
}
