import AppKit

extension NSImage {
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
