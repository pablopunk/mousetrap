import AppKit
import ApplicationServices

final class FocusedScreenResolver {
    func resolveFocusedScreen() -> NSScreen? {
        if let screen = screenFromFocusedWindow() {
            return screen
        }

        let mouseLocation = NSEvent.mouseLocation
        if let mouseScreen = NSScreen.screens.first(where: { NSMouseInRect(mouseLocation, $0.frame, false) }) {
            return mouseScreen
        }

        return NSScreen.main ?? NSScreen.screens.first
    }

    private func screenFromFocusedWindow() -> NSScreen? {
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }

        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        for attribute in [kAXFocusedWindowAttribute as CFString, kAXMainWindowAttribute as CFString] {
            var value: CFTypeRef?
            let result = AXUIElementCopyAttributeValue(axApp, attribute, &value)
            guard result == .success, let window = value else { continue }

            let axWindow = unsafeBitCast(window, to: AXUIElement.self)
            guard let frame = frame(of: axWindow) else { continue }
            if let screen = screen(containing: frame.center) {
                return screen
            }
        }

        return nil
    }

    private func frame(of window: AXUIElement) -> CGRect? {
        var positionValue: CFTypeRef?
        var sizeValue: CFTypeRef?

        let positionResult = AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionValue)
        let sizeResult = AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeValue)

        guard positionResult == .success,
              sizeResult == .success,
              let positionValue,
              let sizeValue,
              CFGetTypeID(positionValue) == AXValueGetTypeID(),
              CFGetTypeID(sizeValue) == AXValueGetTypeID() else {
            return nil
        }

        var position = CGPoint.zero
        var size = CGSize.zero
        AXValueGetValue(positionValue as! AXValue, .cgPoint, &position)
        AXValueGetValue(sizeValue as! AXValue, .cgSize, &size)

        return CGRect(origin: position, size: size)
    }

    private func screen(containing point: CGPoint) -> NSScreen? {
        NSScreen.screens.first(where: { $0.frame.contains(point) })
    }
}
