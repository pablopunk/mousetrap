import AppKit
import Combine
import ServiceManagement
import KeyboardShortcuts

enum SettingsKeys {
    static let freeMouseStep = "freeMouseStep"
    static let defaultFreeMouseStep: Double = 20
}

@MainActor
final class SettingsStore: ObservableObject {
    static let shared = SettingsStore()

    @Published private(set) var launchAtLoginEnabled: Bool

    private init() {
        self.launchAtLoginEnabled = SMAppService.mainApp.status == .enabled

        if KeyboardShortcuts.getShortcut(for: .activateMousetrap) == nil {
            KeyboardShortcuts.setShortcut(.init(.space, modifiers: [.command, .shift]), for: .activateMousetrap)
        }

        if UserDefaults.standard.object(forKey: SettingsKeys.freeMouseStep) == nil {
            UserDefaults.standard.set(SettingsKeys.defaultFreeMouseStep, forKey: SettingsKeys.freeMouseStep)
        }
    }

    func refreshLaunchAtLoginState() {
        launchAtLoginEnabled = SMAppService.mainApp.status == .enabled
    }

    func setLaunchAtLogin(_ enabled: Bool) {
        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
        } catch {
            print("[Mousetrap] launch at login update failed: \(error)")
        }
        refreshLaunchAtLoginState()
    }

    var shortcutDisplay: String {
        guard let shortcut = KeyboardShortcuts.getShortcut(for: .activateMousetrap) else { return "Not set" }
        return shortcut.description
    }
}
