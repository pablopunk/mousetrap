import SwiftUI
import ServiceManagement
import KeyboardShortcuts

struct SectionHeaderView: View {
    let title: String
    let icon: String

    var body: some View {
        Label(title, systemImage: icon)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(.secondary)
            .textCase(.uppercase)
    }
}

struct PreferenceToggleRow: View {
    let title: String
    let subtitle: String
    let icon: String
    @Binding var isOn: Bool

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 13))
                .foregroundStyle(.tint)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.system(size: 13, weight: .medium))
                Text(subtitle)
                    .font(.system(size: 11))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
            }

            Spacer()

            Toggle("", isOn: $isOn)
                .toggleStyle(.switch)
                .controlSize(.mini)
                .labelsHidden()
        }
        .frame(minHeight: 34)
    }
}

struct LaunchAtLoginRow: View {
    @State private var isEnabled = SMAppService.mainApp.status == .enabled

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "play.circle")
                .font(.system(size: 13))
                .foregroundStyle(.tint)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 1) {
                Text("Launch at login")
                    .font(.system(size: 13, weight: .medium))
                Text("Start Mousetrap automatically")
                    .font(.system(size: 11))
                    .foregroundStyle(.tertiary)
            }

            Spacer()

            Toggle("", isOn: Binding(
                get: { isEnabled },
                set: { newValue in
                    do {
                        if newValue {
                            try SMAppService.mainApp.register()
                        } else {
                            try SMAppService.mainApp.unregister()
                        }
                        isEnabled = (SMAppService.mainApp.status == .enabled)
                    } catch {
                        print("[Mousetrap] launch at login update failed: \(error)")
                        isEnabled = (SMAppService.mainApp.status == .enabled)
                    }
                }
            ))
            .toggleStyle(.switch)
            .controlSize(.mini)
            .labelsHidden()
        }
        .frame(minHeight: 34)
    }
}

private struct GridLevelPill: View {
    let label: String
    @AppStorage var isOn: Bool

    init(label: String, key: String, defaultValue: Bool = false) {
        self.label = label
        self._isOn = AppStorage(wrappedValue: defaultValue, key)
    }

    var body: some View {
        Button { isOn.toggle() } label: {
            Text(label)
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundStyle(isOn ? .white : .secondary)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(isOn ? Color.accentColor : Color.secondary.opacity(0.15))
                .clipShape(Capsule())
        }
        .buttonStyle(.plain)
    }
}

struct ShortcutRowView: View {
    @AppStorage(SettingsKeys.freeMouseStep) private var freeMouseStep = SettingsKeys.defaultFreeMouseStep
    @AppStorage(SettingsKeys.unsafeStateTimeoutSeconds) private var unsafeStateTimeoutSeconds = SettingsKeys.defaultUnsafeStateTimeoutSeconds

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                Image(systemName: "command")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.tint)
                    .frame(width: 18)

                Text("Activate")
                    .font(.system(size: 13, weight: .semibold))
                    .frame(width: 92, alignment: .leading)

                Spacer(minLength: 0)

                KeyboardShortcuts.Recorder(for: .activateMousetrap)
                    .frame(width: 130, alignment: .trailing)
            }
            .frame(minHeight: 34)

            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .center, spacing: 10) {
                    Image(systemName: "timer")
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(.tint)
                        .frame(width: 18)

                    Text("Global timeout")
                        .font(.system(size: 13, weight: .medium))

                    Spacer(minLength: 0)

                    HStack(spacing: 6) {
                        Text("\(unsafeStateTimeoutSeconds)")
                            .font(.system(size: 12, weight: .medium, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .frame(width: 24, alignment: .trailing)

                        Text("secs")
                            .font(.system(size: 12))
                            .foregroundStyle(.tertiary)

                        Stepper("", value: $unsafeStateTimeoutSeconds, in: 3...60)
                            .labelsHidden()
                            .controlSize(.small)
                    }
                }

                Text("Dismiss the UI when no action is taken")
                    .font(.system(size: 11))
                    .foregroundStyle(.tertiary)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 28)
            }

            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .center, spacing: 10) {
                    Image(systemName: "sparkles")
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(.tint)
                        .frame(width: 18)

                    Text("Travel")
                        .font(.system(size: 13, weight: .medium))

                    Spacer(minLength: 0)

                    HStack(spacing: 8) {
                        Slider(value: $freeMouseStep, in: 1...20, step: 1)
                            .controlSize(.small)
                            .frame(width: 90)

                        Text("\(Int(freeMouseStep))")
                            .font(.system(size: 12, weight: .medium, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .frame(width: 24, alignment: .trailing)
                    }
                }

                Text("↩ to click (twice to double click)\n⇧↩ to right click · ⇧←↑↓→ to drag")
                    .font(.system(size: 11))
                    .foregroundStyle(.tertiary)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 28)
            }

            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .center, spacing: 10) {
                    Image(systemName: "waveform")
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(.tint)
                        .frame(width: 18)

                    Text("High contrast pulse")
                        .font(.system(size: 13, weight: .medium))

                    Spacer(minLength: 0)

                    HStack(spacing: 6) {
                        GridLevelPill(label: "1st", key: SettingsKeys.pulseGrid1)
                        GridLevelPill(label: "2nd", key: SettingsKeys.pulseGrid2)
                        GridLevelPill(label: "3rd", key: SettingsKeys.pulseGrid3, defaultValue: true)
                    }
                }

                Text("Pulse the grid to reveal content behind it")
                    .font(.system(size: 11))
                    .foregroundStyle(.tertiary)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 28)
            }
        }
    }
}

struct MenuBarContentView: View {
    @AppStorage("showMenuBarIcon") private var showMenuBarIcon = true
    let permissionManager: PermissionManager
    let onQuit: () -> Void
    @State private var hasAccessibilityPermission = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(alignment: .leading, spacing: 6) {
                SectionHeaderView(title: "Preferences", icon: "slider.horizontal.3")
                LaunchAtLoginRow()
                PreferenceToggleRow(
                    title: "Show menu bar icon",
                    subtitle: "Reopen app to re-enable",
                    icon: "menubar.rectangle",
                    isOn: $showMenuBarIcon
                )
            }

            Divider().opacity(0.5)

            VStack(alignment: .leading, spacing: 8) {
                ShortcutRowView()

                if !hasAccessibilityPermission {
                    Button {
                        permissionManager.openAccessibilitySettings()
                    } label: {
                        Label("Accessibility permissions needed", systemImage: "exclamationmark.triangle")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundStyle(.orange)
                    }
                    .buttonStyle(.plain)
                }
            }

            Divider().opacity(0.5)

            Button(action: onQuit) {
                HStack(spacing: 4) {
                    Image(systemName: "power")
                    Text("Quit")
                    Text("⌘Q")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                .font(.system(size: 12))
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(width: 320)
        .onAppear {
            refreshAccessibilityPermission()
        }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            refreshAccessibilityPermission()
        }
    }

    private func refreshAccessibilityPermission() {
        hasAccessibilityPermission = permissionManager.hasAccessibilityPermission
    }
}
