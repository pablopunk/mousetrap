import SwiftUI
import AppKit

private extension Bundle {
    var buildNumber: String? {
        infoDictionary?["CFBundleVersion"] as? String
    }
}

struct InfoView: View {
    let onQuit: () -> Void
    private let version = Bundle.main.buildNumber

    var body: some View {
        VStack(spacing: 14) {
            HStack(spacing: 12) {
                Image(nsImage: .menuBarIcon)
                    .renderingMode(.template)
                    .resizable()
                    .interpolation(.high)
                    .foregroundStyle(.tint)
                    .frame(width: 28, height: 28)
                    .padding(6)

                VStack(alignment: .leading, spacing: 2) {
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text("Mousetrap")
                            .font(.system(size: 16, weight: .semibold))
                        if let version {
                            Text("v\(version)")
                                .font(.system(size: 10, weight: .medium, design: .monospaced))
                                .foregroundStyle(.tertiary)
                        }
                    }
                    HStack(spacing: 4) {
                        Text("Made with 🩵 by")
                            .font(.system(size: 11))
                            .foregroundStyle(.secondary)
                        Link("Pablo Varela", destination: URL(string: "https://pablopunk.com")!)
                            .font(.system(size: 11, weight: .medium))
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            Divider().opacity(0.5)

            HStack(spacing: 8) {
                Link(destination: URL(string: "https://github.com/pablopunk/mousetrap")!) {
                    HStack(spacing: 5) {
                        Image(systemName: "swift")
                            .font(.system(size: 11))
                        Text("GitHub")
                            .font(.system(size: 11, weight: .medium))
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 7)
                    .background(.quaternary.opacity(0.5))
                    .clipShape(RoundedRectangle(cornerRadius: 7))
                }
                .buttonStyle(.plain)

                Button(action: onQuit) {
                    HStack(spacing: 4) {
                        Image(systemName: "power")
                            .font(.system(size: 11))
                        Text("Quit")
                            .font(.system(size: 11, weight: .medium))
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 7)
                    .background(.quaternary.opacity(0.5))
                    .clipShape(RoundedRectangle(cornerRadius: 7))
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .keyboardShortcut("Q", modifiers: .command)
            }
        }
        .padding(14)
    }
}
