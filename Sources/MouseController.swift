import AppKit
import CoreGraphics
import Foundation

@MainActor
enum MouseController {
    private static let syntheticHoverEventUserData: Int64 = 0x4D4F5553
    private static var lastProgrammaticQuartzMove: (point: CGPoint, time: CFAbsoluteTime)?
    private static var lastTrackedCursorPositionAppKit: CGPoint?

    private static var freeMoveStep: CGFloat {
        CGFloat(UserDefaults.standard.double(forKey: SettingsKeys.freeMouseStep)).clamped(to: 4...80)
    }

    @discardableResult
    static func moveCursor(to point: CGPoint) -> Bool {
        guard let screen = NSScreen.screens.first(where: { $0.frame.insetBy(dx: -1, dy: -1).contains(point) }) ?? NSScreen.main,
              let displayID = displayID(for: screen) else {
            print("[Mousetrap] failed to resolve screen/display for appkit=\(point)")
            return false
        }

        let displayPoint = displayLocalQuartzPoint(from: point, in: screen)
        CGDisplayMoveCursorToPoint(displayID, displayPoint)
        CGAssociateMouseAndMouseCursorPosition(1)

        let quartzPoint = currentQuartzCursorPosition
        lastProgrammaticQuartzMove = (quartzPoint, CFAbsoluteTimeGetCurrent())
        lastTrackedCursorPositionAppKit = point
        postSyntheticHoverEvent(at: quartzPoint)

        print("[Mousetrap] move cursor to appkit=\(point) displayLocalQuartz=\(displayPoint) displayID=\(displayID) quartz=\(quartzPoint)")
        return true
    }

    static func leftClick(at point: CGPoint? = nil) {
        let point = point ?? currentCursorPositionAppKit
        if point != currentCursorPositionAppKit {
            _ = moveCursor(to: point)
        }
        let eventPoint = currentQuartzCursorPosition
        let mouseDown = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: eventPoint, mouseButton: .left)
        let mouseUp = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: eventPoint, mouseButton: .left)
        mouseDown?.flags = []
        mouseUp?.flags = []
        mouseDown?.post(tap: .cghidEventTap)
        mouseUp?.post(tap: .cghidEventTap)
        print("[Mousetrap] left click at appkit=\(point) quartz=\(eventPoint)")
    }

    static func rightClick(at point: CGPoint? = nil) {
        let point = point ?? currentCursorPositionAppKit
        if point != currentCursorPositionAppKit {
            _ = moveCursor(to: point)
        }
        let eventPoint = currentQuartzCursorPosition
        let mouseDown = CGEvent(mouseEventSource: nil, mouseType: .rightMouseDown, mouseCursorPosition: eventPoint, mouseButton: .right)
        let mouseUp = CGEvent(mouseEventSource: nil, mouseType: .rightMouseUp, mouseCursorPosition: eventPoint, mouseButton: .right)
        mouseDown?.flags = []
        mouseUp?.flags = []
        mouseDown?.post(tap: .cghidEventTap)
        mouseUp?.post(tap: .cghidEventTap)
        print("[Mousetrap] right click at appkit=\(point) quartz=\(eventPoint)")
    }

    static func doubleClick(at point: CGPoint? = nil) {
        let point = point ?? currentCursorPositionAppKit
        if point != currentCursorPositionAppKit {
            _ = moveCursor(to: point)
        }
        let eventPoint = currentQuartzCursorPosition

        for clickState in [1, 2] {
            let mouseDown = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: eventPoint, mouseButton: .left)
            let mouseUp = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: eventPoint, mouseButton: .left)
            mouseDown?.flags = []
            mouseUp?.flags = []
            mouseDown?.setIntegerValueField(.mouseEventClickState, value: Int64(clickState))
            mouseUp?.setIntegerValueField(.mouseEventClickState, value: Int64(clickState))
            mouseDown?.post(tap: .cghidEventTap)
            mouseUp?.post(tap: .cghidEventTap)
        }

        print("[Mousetrap] double click at appkit=\(point) quartz=\(eventPoint)")
    }

    @discardableResult
    static func moveFreeCursor(direction: InterceptedKey) -> Bool {
        let start = currentCursorPositionAppKit
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

        let target = clampedCursorPosition(start.applying(.init(translationX: delta.x, y: delta.y)))
        return moveCursor(to: target)
    }

    static var currentCursorPositionAppKit: CGPoint {
        lastTrackedCursorPositionAppKit ?? NSEvent.mouseLocation
    }

    static func clearTrackedCursorPosition() {
        lastTrackedCursorPositionAppKit = nil
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

    static func shouldIgnoreObservedMouseMovement(_ event: CGEvent) -> Bool {
        if event.getIntegerValueField(.eventSourceUserData) == syntheticHoverEventUserData {
            return true
        }

        guard let lastProgrammaticQuartzMove else { return false }
        guard CFAbsoluteTimeGetCurrent() - lastProgrammaticQuartzMove.time < 0.2 else { return false }

        let dx = event.location.x - lastProgrammaticQuartzMove.point.x
        let dy = event.location.y - lastProgrammaticQuartzMove.point.y
        return (dx * dx + dy * dy) <= 4
    }

    private static var currentQuartzCursorPosition: CGPoint {
        CGEvent(source: nil)?.location ?? .zero
    }

    private static func displayID(for screen: NSScreen) -> CGDirectDisplayID? {
        (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber).map { CGDirectDisplayID($0.uint32Value) }
    }

    private static func displayLocalQuartzPoint(from point: CGPoint, in screen: NSScreen) -> CGPoint {
        CGPoint(
            x: point.x - screen.frame.minX,
            y: screen.frame.maxY - point.y
        )
    }

    private static func postSyntheticHoverEvent(at point: CGPoint) {
        guard let event = CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left) else {
            return
        }

        event.setIntegerValueField(.eventSourceUserData, value: syntheticHoverEventUserData)
        event.post(tap: .cghidEventTap)
    }
}

private extension CGFloat {
    func clamped(to range: ClosedRange<CGFloat>) -> CGFloat {
        Swift.min(Swift.max(self, range.lowerBound), range.upperBound)
    }
}
