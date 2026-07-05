import SwiftUI

private enum PanelTab: String, CaseIterable {
    case settings = "Settings"
    case info = "About"

    var icon: String {
        switch self {
        case .settings: return "gear"
        case .info: return "info.circle"
        }
    }
}

private let panelWidth: CGFloat = 320

struct AppPanelView: View {
    let permissionManager: PermissionManager
    let onQuit: () -> Void
    @State private var selectedTab: PanelTab = .settings

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                ForEach(PanelTab.allCases, id: \.self) { tab in
                    Button {
                        withAnimation(.snappy(duration: 0.25)) {
                            selectedTab = tab
                        }
                    } label: {
                        HStack(spacing: 5) {
                            Image(systemName: tab.icon)
                                .font(.system(size: 12, weight: .medium))
                            if selectedTab == tab {
                                Text(tab.rawValue)
                                    .font(.system(size: 11, weight: .semibold))
                                    .lineLimit(1)
                                    .fixedSize()
                            }
                        }
                        .padding(.horizontal, 10)
                        .padding(.vertical, 7)
                        .frame(maxWidth: .infinity)
                        .contentShape(Rectangle())
                        .background {
                            if selectedTab == tab {
                                Capsule().fill(.tint.opacity(0.15))
                            }
                        }
                        .clipShape(Capsule())
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(selectedTab == tab ? .primary : .secondary)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)

            Group {
                switch selectedTab {
                case .settings:
                    MenuBarContentView(
                        permissionManager: permissionManager,
                        onQuit: onQuit
                    )
                case .info:
                    InfoView(onQuit: onQuit)
                }
            }
            .transition(.opacity.combined(with: .move(edge: .bottom)))
        }
        .frame(width: panelWidth)
    }
}
