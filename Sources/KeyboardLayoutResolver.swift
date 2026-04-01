import Carbon
import CoreServices

struct KeyboardLayoutResolver {
    private let rootTemplate: [[UInt16]] = [
        [18, 19, 20, 21, 23, 22, 26, 28, 25, 29],
        [12, 13, 14, 15, 17, 16, 32, 34, 31, 35],
        [0, 1, 2, 3, 5, 4, 38, 40, 37, 41],
        [6, 7, 8, 9, 11, 45, 46, 43, 47, 44]
    ]

    private let refinementTemplate: [[UInt16]] = [
        [18, 19, 20, 26, 28, 25],
        [12, 13, 14, 34, 31, 35],
        [0, 1, 2, 40, 37, 41],
        [6, 7, 8, 43, 47, 44]
    ]

    // Final click uses the outer keys of the refinement grid.
    // On the current ES layout these keycodes resolve to:
    // ["1", "2", "0"], ["q", "w", "p"], ["a", "s", "ñ"], ["z", "x", "'"]
    private let finalClickTemplate: [[UInt16]] = [
        [18, 19, 29],
        [12, 13, 35],
        [0, 1, 41],
        [6, 7, 27]
    ]

    func resolveLayouts() -> (root: GridLayout, refinement: GridLayout, finalClick: GridLayout) {
        (
            root: GridLayout(id: "full", rows: resolveRows(from: rootTemplate, fallback: GridLayout.full.rows)),
            refinement: GridLayout(id: "refinement", rows: resolveRows(from: refinementTemplate, fallback: GridLayout.refinement.rows)),
            finalClick: GridLayout(id: "finalClick", rows: resolveRows(from: finalClickTemplate, fallback: GridLayout.finalClick.rows))
        )
    }

    private func resolveRows(from template: [[UInt16]], fallback: [[Character]]) -> [[Character]] {
        guard let keyboardLayout else { return fallback }

        let resolved = template.enumerated().map { rowIndex, row in
            row.enumerated().compactMap { columnIndex, keyCode in
                translatedCharacter(for: keyCode, keyboardLayout: keyboardLayout)
                    ?? fallback[safe: rowIndex]?[safe: columnIndex]
            }
        }

        let hasEmptyRows = resolved.contains(where: { $0.isEmpty })
        return hasEmptyRows ? fallback : resolved
    }

    private var keyboardLayout: UnsafePointer<UCKeyboardLayout>? {
        guard let source = TISCopyCurrentKeyboardLayoutInputSource()?.takeRetainedValue(),
              let rawLayoutData = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData) else {
            return nil
        }

        let layoutData = unsafeBitCast(rawLayoutData, to: CFData.self)
        guard let bytes = CFDataGetBytePtr(layoutData) else { return nil }
        return UnsafePointer<UCKeyboardLayout>(OpaquePointer(bytes))
    }

    private func translatedCharacter(for keyCode: UInt16, keyboardLayout: UnsafePointer<UCKeyboardLayout>) -> Character? {
        var deadKeyState: UInt32 = 0
        var length = 0
        var chars = [UniChar](repeating: 0, count: 4)

        let result = UCKeyTranslate(
            keyboardLayout,
            UInt16(keyCode),
            UInt16(kUCKeyActionDisplay),
            0,
            UInt32(LMGetKbdType()),
            OptionBits(kUCKeyTranslateNoDeadKeysBit),
            &deadKeyState,
            chars.count,
            &length,
            &chars
        )

        guard result == noErr, length > 0 else { return nil }

        let string = String(utf16CodeUnits: chars, count: Int(length))
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()

        guard string.count == 1, let character = string.first else { return nil }
        return character
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
