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
        case 36: return .returnKey
        case 49: return .space
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
