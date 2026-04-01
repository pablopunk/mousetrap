import SwiftUI
import AppKit

private extension NSImage {
    static let menuBarIcon: NSImage = {
        guard let resourceURL = Bundle.main.url(forResource: "minimal-icon", withExtension: "png"),
              let image = NSImage(contentsOf: resourceURL) else {
            return NSImage(systemSymbolName: "cursorarrow.click", accessibilityDescription: "Mousetrap") ?? NSImage()
        }

        image.isTemplate = true
        image.size = NSSize(width: 18, height: 18)
        return image
    }()
}

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
