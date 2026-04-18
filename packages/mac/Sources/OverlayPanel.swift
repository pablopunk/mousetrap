import AppKit

@MainActor
final class OverlayPanel: NSPanel {
    var onKey: ((InterceptedKey) -> Void)?

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { true }

    override func keyDown(with event: NSEvent) {
        if let key = map(event) {
            onKey?(key)
            return
        }

        super.keyDown(with: event)
    }

    private func map(_ event: NSEvent) -> InterceptedKey? {
        switch event.keyCode {
        case 53: return .escape
        case 51: return .delete
        case 36, 76: return event.modifierFlags.contains(.shift) ? .shiftReturnKey : .returnKey
        case 49: return .space
        case 123: return event.modifierFlags.contains(.shift) ? .shiftLeftArrow : .leftArrow
        case 124: return event.modifierFlags.contains(.shift) ? .shiftRightArrow : .rightArrow
        case 125: return event.modifierFlags.contains(.shift) ? .shiftDownArrow : .downArrow
        case 126: return event.modifierFlags.contains(.shift) ? .shiftUpArrow : .upArrow
        default:
            break
        }

        guard let characters = event.charactersIgnoringModifiers?.lowercased(),
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
