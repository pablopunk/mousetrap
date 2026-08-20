import SwiftUI
import KeyboardShortcuts

struct ShortcutEditorView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Shortcut")
                .font(.system(size: 18, weight: .semibold))

            Text("Choose the global shortcut that activates Mousetrap.")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)

            HStack(spacing: 10) {
                Text("Activate")
                    .font(.system(size: 13, weight: .semibold))
                    .frame(width: 60, alignment: .leading)

                KeyboardShortcuts.Recorder(for: .activateMousetrap)
                    .frame(height: 24)

                Button("Reset") {
                    KeyboardShortcuts.reset(.activateMousetrap)
                }
                .buttonStyle(.borderless)
            }

            Spacer()
        }
        .padding(16)
        .frame(width: 360, height: 140)
    }
}
