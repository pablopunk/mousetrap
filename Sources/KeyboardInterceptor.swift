import AppKit
import CoreGraphics

enum InterceptedKey {
    case character(Character)
    case escape
    case delete
    case returnKey
    case shiftReturnKey
    case space
    case upArrow
    case downArrow
    case leftArrow
    case rightArrow
    case shiftUpArrow
    case shiftDownArrow
    case shiftLeftArrow
    case shiftRightArrow
}

@MainActor
final class KeyboardInterceptor {
    var onKey: ((InterceptedKey) -> Void)?

    private var eventTap: CFMachPort?
    private var runLoopSource: CFRunLoopSource?

    var isActive: Bool {
        eventTap != nil
    }

    func start() {
        guard eventTap == nil else { return }

        let mask = (1 << CGEventType.keyDown.rawValue)
            | (1 << CGEventType.tapDisabledByTimeout.rawValue)
            | (1 << CGEventType.tapDisabledByUserInput.rawValue)

        let callback: CGEventTapCallBack = { _, type, event, userInfo in
            guard let userInfo else { return Unmanaged.passUnretained(event) }
            let interceptor = Unmanaged<KeyboardInterceptor>.fromOpaque(userInfo).takeUnretainedValue()
            return interceptor.handleEvent(type: type, event: event)
        }

        let selfPointer = Unmanaged.passUnretained(self).toOpaque()
        guard let eventTap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .defaultTap,
            eventsOfInterest: CGEventMask(mask),
            callback: callback,
            userInfo: selfPointer
        ) else {
            print("[Mousetrap] failed to create keyboard interceptor event tap")
            return
        }

        let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: eventTap, enable: true)

        self.eventTap = eventTap
        self.runLoopSource = source
    }

    func stop() {
        if let runLoopSource {
            CFRunLoopRemoveSource(CFRunLoopGetMain(), runLoopSource, .commonModes)
            self.runLoopSource = nil
        }

        if let eventTap {
            CFMachPortInvalidate(eventTap)
            self.eventTap = nil
        }
    }

    private func handleEvent(type: CGEventType, event: CGEvent) -> Unmanaged<CGEvent>? {
        if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
            if let eventTap {
                CGEvent.tapEnable(tap: eventTap, enable: true)
            }
            return Unmanaged.passUnretained(event)
        }

        guard type == .keyDown, let key = map(event) else {
            return Unmanaged.passUnretained(event)
        }

        DispatchQueue.main.async { [weak self] in
            self?.onKey?(key)
        }
        return nil
    }

    private func map(_ event: CGEvent) -> InterceptedKey? {
        let keyCode = event.getIntegerValueField(.keyboardEventKeycode)
        let flags = NSEvent.ModifierFlags(rawValue: UInt(event.flags.rawValue))
        let nonShiftModifiers = flags.intersection([.command, .control, .option])

        switch keyCode {
        case 53: return .escape
        case 51: return nonShiftModifiers.isEmpty ? .delete : nil
        case 36, 76:
            guard nonShiftModifiers.isEmpty else { return nil }
            return flags.contains(.shift) ? .shiftReturnKey : .returnKey
        case 49:
            return nonShiftModifiers.isEmpty ? .space : nil
        case 123:
            guard nonShiftModifiers.isEmpty else { return nil }
            return flags.contains(.shift) ? .shiftLeftArrow : .leftArrow
        case 124:
            guard nonShiftModifiers.isEmpty else { return nil }
            return flags.contains(.shift) ? .shiftRightArrow : .rightArrow
        case 125:
            guard nonShiftModifiers.isEmpty else { return nil }
            return flags.contains(.shift) ? .shiftDownArrow : .downArrow
        case 126:
            guard nonShiftModifiers.isEmpty else { return nil }
            return flags.contains(.shift) ? .shiftUpArrow : .upArrow
        default:
            break
        }

        guard nonShiftModifiers.isEmpty,
              let nsEvent = NSEvent(cgEvent: event),
              let characters = nsEvent.charactersIgnoringModifiers?.lowercased(),
              let character = characters.first else {
            return nil
        }

        let supportedCharacters = "1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./ñ"
        guard supportedCharacters.contains(character) else {
            return nil
        }

        return .character(character)
    }
}
