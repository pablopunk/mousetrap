import AppKit
import CoreGraphics

enum MouseController {
    @discardableResult
    static func moveCursor(to point: CGPoint) -> Bool {
        let quartzPoint = quartzPoint(from: point)
        let result = CGWarpMouseCursorPosition(quartzPoint)
        if result == .success {
            CGAssociateMouseAndMouseCursorPosition(1)
            print("[Mousetrap] move cursor to appkit=\(point) quartz=\(quartzPoint)")
            return true
        }
        print("[Mousetrap] failed moving cursor to appkit=\(point) quartz=\(quartzPoint) result=\(result.rawValue)")
        return false
    }

    static func leftClick(at point: CGPoint? = nil) {
        let point = point ?? currentCursorPositionAppKit
        let quartzPoint = quartzPoint(from: point)
        let mouseDown = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: quartzPoint, mouseButton: .left)
        let mouseUp = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: quartzPoint, mouseButton: .left)
        mouseDown?.flags = []
        mouseUp?.flags = []
        mouseDown?.post(tap: .cghidEventTap)
        mouseUp?.post(tap: .cghidEventTap)
        print("[Mousetrap] left click at appkit=\(point) quartz=\(quartzPoint)")
    }

    private static var currentCursorPositionAppKit: CGPoint {
        NSEvent.mouseLocation
    }

    private static func quartzPoint(from point: CGPoint) -> CGPoint {
        guard let screen = NSScreen.screens.first(where: { $0.frame.contains(point) }) ?? NSScreen.main else {
            return point
        }

        let localY = point.y - screen.frame.minY
        let flippedY = screen.frame.maxY - localY
        return CGPoint(x: point.x, y: flippedY)
    }
}
