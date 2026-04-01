import AppKit
import CoreGraphics

enum MouseController {
    private static var freeMoveStep: CGFloat {
        CGFloat(UserDefaults.standard.double(forKey: SettingsKeys.freeMouseStep)).clamped(to: 4...80)
    }

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

    static func rightClick(at point: CGPoint? = nil) {
        let point = point ?? currentCursorPositionAppKit
        let quartzPoint = quartzPoint(from: point)
        let mouseDown = CGEvent(mouseEventSource: nil, mouseType: .rightMouseDown, mouseCursorPosition: quartzPoint, mouseButton: .right)
        let mouseUp = CGEvent(mouseEventSource: nil, mouseType: .rightMouseUp, mouseCursorPosition: quartzPoint, mouseButton: .right)
        mouseDown?.flags = []
        mouseUp?.flags = []
        mouseDown?.post(tap: .cghidEventTap)
        mouseUp?.post(tap: .cghidEventTap)
        print("[Mousetrap] right click at appkit=\(point) quartz=\(quartzPoint)")
    }

    static func doubleClick(at point: CGPoint? = nil) {
        let point = point ?? currentCursorPositionAppKit
        let quartzPoint = quartzPoint(from: point)

        for clickState in [1, 2] {
            let mouseDown = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: quartzPoint, mouseButton: .left)
            let mouseUp = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: quartzPoint, mouseButton: .left)
            mouseDown?.flags = []
            mouseUp?.flags = []
            mouseDown?.setIntegerValueField(.mouseEventClickState, value: Int64(clickState))
            mouseUp?.setIntegerValueField(.mouseEventClickState, value: Int64(clickState))
            mouseDown?.post(tap: .cghidEventTap)
            mouseUp?.post(tap: .cghidEventTap)
        }

        print("[Mousetrap] double click at appkit=\(point) quartz=\(quartzPoint)")
    }

    @discardableResult
    static func moveFreeCursor(direction: InterceptedKey) -> Bool {
        let delta: CGPoint

        switch direction {
        case .upArrow:
            delta = CGPoint(x: 0, y: freeMoveStep)
        case .downArrow:
            delta = CGPoint(x: 0, y: -freeMoveStep)
        case .leftArrow:
            delta = CGPoint(x: -freeMoveStep, y: 0)
        case .rightArrow:
            delta = CGPoint(x: freeMoveStep, y: 0)
        default:
            return false
        }

        return moveCursor(to: clampedCursorPosition(currentCursorPositionAppKit.applying(.init(translationX: delta.x, y: delta.y))))
    }

    static var currentCursorPositionAppKit: CGPoint {
        NSEvent.mouseLocation
    }

    private static func clampedCursorPosition(_ point: CGPoint) -> CGPoint {
        let screen = NSScreen.screens.first(where: { $0.frame.insetBy(dx: -1, dy: -1).contains(currentCursorPositionAppKit) })
            ?? NSScreen.screens.first(where: { $0.frame.insetBy(dx: -1, dy: -1).contains(point) })
            ?? NSScreen.main

        guard let screen else { return point }

        return CGPoint(
            x: min(max(point.x, screen.frame.minX), screen.frame.maxX - 1),
            y: min(max(point.y, screen.frame.minY), screen.frame.maxY - 1)
        )
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

private extension CGFloat {
    func clamped(to range: ClosedRange<CGFloat>) -> CGFloat {
        Swift.min(Swift.max(self, range.lowerBound), range.upperBound)
    }
}
