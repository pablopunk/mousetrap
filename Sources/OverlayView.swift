import AppKit
import QuartzCore

@MainActor
final class OverlayView: NSView {
    private static let pulsePeriod: CFTimeInterval = 1.5

    var state: GridState = GridState(screenRect: .zero, currentRect: .zero, history: [], layout: .full) {
        didSet {
            needsDisplay = true
            updatePulse()
        }
    }

    var pressedKeys = Set<Character>() {
        didSet { needsDisplay = true }
    }

    var previewKeys = Set<Character>() {
        didSet { needsDisplay = true }
    }

    var previewPoint: CGPoint? {
        didSet { needsDisplay = true }
    }

    private var pulseTimer: Timer?
    private var pulseStartTime: CFTimeInterval = 0
    private var pulseOpacity: CGFloat = 1.0

    override var isOpaque: Bool { false }

    override func draw(_ dirtyRect: NSRect) {
        guard let context = NSGraphicsContext.current?.cgContext else { return }

        context.setFillColor(NSColor.black.withAlphaComponent(0.18).cgColor)
        context.fill(bounds)

        let localCurrentRect = CGRect(
            x: state.currentRect.minX - state.screenRect.minX,
            y: state.currentRect.minY - state.screenRect.minY,
            width: state.currentRect.width,
            height: state.currentRect.height
        )

        let isFinalClickLayout = state.layout.id == "finalClick"
        let currentRectOverlayOpacity: CGFloat = shouldPulse ? 0 : (isFinalClickLayout ? 0.03 : 0.10)
        context.setFillColor(NSColor.black.withAlphaComponent(currentRectOverlayOpacity).cgColor)
        context.fill(localCurrentRect)

        drawKeys(in: localCurrentRect, opacity: pulseOpacity, isPulsing: shouldPulse)
    }

    private func drawKeys(in rect: CGRect, opacity: CGFloat, isPulsing: Bool) {
        let rows = state.layout.rows
        let rowHeight = rect.height / CGFloat(rows.count)
        let cellInset: CGFloat = state.history.isEmpty ? 4 : 0
        let isFinalClickLayout = state.layout.id == "finalClick"
        let baseFill: CGFloat = isFinalClickLayout ? 0.10 : 0.08
        let baseStroke: CGFloat = isFinalClickLayout ? 0.24 : 0.20
        let baseText: CGFloat = isFinalClickLayout ? 0.80 : 0.94
        let cellFillOpacity: CGFloat = (isPulsing ? 0.82 : baseFill) * opacity
        let cellStrokeOpacity: CGFloat = (isPulsing ? 0.88 : baseStroke) * opacity
        let textOpacity: CGFloat = (isPulsing ? 1.0 : baseText) * opacity
        let fontScale: CGFloat = isFinalClickLayout ? 0.10 : 0.32

        for (rowIndex, row) in rows.enumerated() {
            let cellWidth = rect.width / CGFloat(row.count)

            for (columnIndex, key) in row.enumerated() {
                let cell = CGRect(
                    x: rect.minX + CGFloat(columnIndex) * cellWidth,
                    y: rect.maxY - CGFloat(rowIndex + 1) * rowHeight,
                    width: cellWidth,
                    height: rowHeight
                ).insetBy(dx: cellInset, dy: cellInset)

                let lowercasedKey = Character(String(key).lowercased())
                let isPreviewed = previewKeys.contains(lowercasedKey)
                let isPressed = pressedKeys.contains(lowercasedKey)
                let accentColor = NSColor.controlAccentColor

                let rounded = NSBezierPath(roundedRect: cell, xRadius: 8, yRadius: 8)
                let fillColor: NSColor
                let strokeColor: NSColor
                let strokeWidth: CGFloat

                if isPreviewed {
                    fillColor = accentColor.withAlphaComponent((isPressed ? 0.42 : 0.22) * opacity)
                    strokeColor = accentColor.withAlphaComponent((isPressed ? 0.92 : 0.70) * opacity)
                    strokeWidth = isPressed ? 2 : 1.5
                } else if isPulsing {
                    fillColor = NSColor.black.withAlphaComponent(cellFillOpacity)
                    strokeColor = NSColor.white.withAlphaComponent(cellStrokeOpacity)
                    strokeWidth = 1
                } else {
                    fillColor = NSColor.white.withAlphaComponent(cellFillOpacity)
                    strokeColor = NSColor.white.withAlphaComponent(cellStrokeOpacity)
                    strokeWidth = 1
                }

                fillColor.setFill()
                rounded.fill()
                strokeColor.setStroke()
                rounded.lineWidth = strokeWidth
                rounded.stroke()

                let paragraph = NSMutableParagraphStyle()
                paragraph.alignment = .center

                let minimumFontSize: CGFloat = isFinalClickLayout ? 8 : 16
                let textColor: NSColor
                if isPreviewed {
                    textColor = accentColor.blended(withFraction: 0.15, of: .white) ?? accentColor
                } else {
                    textColor = .white
                }

                let attrs: [NSAttributedString.Key: Any] = [
                    .font: NSFont.monospacedSystemFont(ofSize: max(minimumFontSize, min(cell.width, cell.height) * fontScale), weight: .bold),
                    .foregroundColor: textColor.withAlphaComponent(textOpacity),
                    .paragraphStyle: paragraph
                ]

                let text = String(key).uppercased() as NSString
                let size = text.size(withAttributes: attrs)
                let textRect = CGRect(
                    x: cell.midX - size.width / 2,
                    y: cell.midY - size.height / 2,
                    width: size.width,
                    height: size.height
                )
                text.draw(in: textRect, withAttributes: attrs)
            }
        }

        if let previewPoint {
            drawPreviewPoint(previewPoint, opacity: opacity)
        }

        let hint = "⌘⇧Space toggle · Esc cancel · Delete back · Return click"
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center
        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 16, weight: .medium),
            .foregroundColor: NSColor.white.withAlphaComponent(0.9),
            .paragraphStyle: paragraph
        ]
        let hintText = hint as NSString
        hintText.draw(in: CGRect(x: 24, y: 24, width: bounds.width - 48, height: 24), withAttributes: attrs)
    }

    private func drawPreviewPoint(_ point: CGPoint, opacity: CGFloat) {
        let localPoint = CGPoint(x: point.x - state.screenRect.minX, y: point.y - state.screenRect.minY)
        let accentColor = NSColor.controlAccentColor

        let outerRect = CGRect(x: localPoint.x - 7, y: localPoint.y - 7, width: 14, height: 14)
        let innerRect = CGRect(x: localPoint.x - 3, y: localPoint.y - 3, width: 6, height: 6)

        let outer = NSBezierPath(ovalIn: outerRect)
        accentColor.withAlphaComponent(0.95 * opacity).setStroke()
        outer.lineWidth = 2
        outer.stroke()

        let inner = NSBezierPath(ovalIn: innerRect)
        accentColor.withAlphaComponent(0.95 * opacity).setFill()
        inner.fill()
    }

    // MARK: - Pulse animation

    private var currentGridLevel: Int {
        min(state.history.count + 1, 3)
    }

    private var shouldPulse: Bool {
        switch currentGridLevel {
        case 1: return UserDefaults.standard.bool(forKey: SettingsKeys.pulseGrid1)
        case 2: return UserDefaults.standard.bool(forKey: SettingsKeys.pulseGrid2)
        default: return UserDefaults.standard.bool(forKey: SettingsKeys.pulseGrid3)
        }
    }

    private func updatePulse() {
        if shouldPulse {
            startPulse()
        } else {
            stopPulse()
        }
    }

    private func startPulse() {
        guard pulseTimer == nil else { return }
        pulseStartTime = CACurrentMediaTime()
        pulseTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                let elapsed = CACurrentMediaTime() - self.pulseStartTime
                self.pulseOpacity = CGFloat((cos(elapsed * 2.0 * .pi / Self.pulsePeriod) + 1.0) / 2.0)
                self.needsDisplay = true
            }
        }
    }

    func stopPulse() {
        pulseTimer?.invalidate()
        pulseTimer = nil
        pulseOpacity = 1.0
        needsDisplay = true
    }
}
