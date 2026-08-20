import AppKit

@MainActor
final class FreeMouseIndicatorController {
    private var window: NSPanel?
    private var indicatorView: IndicatorView?

    func show(at point: CGPoint) {
        if window == nil {
            createWindow()
        }

        updatePosition(to: point)
        window?.orderFrontRegardless()
        indicatorView?.startAnimating()
    }

    func updatePosition(to point: CGPoint) {
        guard let window else { return }
        let origin = CGPoint(x: point.x + 14, y: point.y - 8)
        window.setFrameOrigin(origin)
    }

    func hide() {
        indicatorView?.stopAnimating()
        window?.orderOut(nil)
    }

    private func createWindow() {
        let rect = CGRect(x: 0, y: 0, width: 20, height: 20)
        let window = NSPanel(
            contentRect: rect,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        window.level = .screenSaver
        window.backgroundColor = .clear
        window.isOpaque = false
        window.hasShadow = false
        window.ignoresMouseEvents = true
        window.hidesOnDeactivate = false
        window.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]

        let view = IndicatorView(frame: CGRect(origin: .zero, size: rect.size))
        window.contentView = view

        self.window = window
        self.indicatorView = view
    }
}

private final class IndicatorView: NSView {
    private let sparkleLayer = CATextLayer()

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.masksToBounds = false
        setUpLayers()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func draw(_ dirtyRect: NSRect) {}

    func startAnimating() {
        guard sparkleLayer.animation(forKey: "pulse") == nil else { return }

        let pulse = CABasicAnimation(keyPath: "transform.scale")
        pulse.fromValue = 0.9
        pulse.toValue = 1.15
        pulse.duration = 0.55
        pulse.autoreverses = true
        pulse.repeatCount = .infinity
        pulse.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        sparkleLayer.add(pulse, forKey: "pulse")

        let opacity = CABasicAnimation(keyPath: "opacity")
        opacity.fromValue = 0.7
        opacity.toValue = 1.0
        opacity.duration = 0.55
        opacity.autoreverses = true
        opacity.repeatCount = .infinity
        opacity.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        sparkleLayer.add(opacity, forKey: "opacityPulse")
    }

    func stopAnimating() {
        sparkleLayer.removeAllAnimations()
    }

    private func setUpLayers() {
        sparkleLayer.string = "✦"
        sparkleLayer.fontSize = 18
        sparkleLayer.alignmentMode = .center
        sparkleLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        sparkleLayer.foregroundColor = NSColor.systemYellow.cgColor
        sparkleLayer.frame = bounds

        layer?.addSublayer(sparkleLayer)
    }
}
