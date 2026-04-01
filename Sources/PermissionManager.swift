import AppKit
import ApplicationServices

@MainActor
final class PermissionManager {
    var hasAccessibilityPermission: Bool {
        AXIsProcessTrusted()
    }

    func openAccessibilitySettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
    }
}
