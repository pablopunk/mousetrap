import SwiftUI
import AppKit

@main
struct MousetrapApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @AppStorage("showMenuBarIcon") private var showMenuBarIcon = true
    private let permissionManager = PermissionManager()

    var body: some Scene {
        MenuBarExtra(isInserted: $showMenuBarIcon) {
            AppPanelView(
                permissionManager: permissionManager,
                onQuit: { NSApplication.shared.terminate(nil) }
            )
        } label: {
            Image(nsImage: .menuBarIcon)
        }
        .menuBarExtraStyle(.window)
        .windowResizability(.contentMinSize)
        .defaultSize(width: 320, height: 250)

    }
}
