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
        let currentRectOverlayOpacity: CGFloat = (isFinalClickLayout ? 0.03 : 0.10) * pulseOpacity
        context.setFillColor(NSColor.black.withAlphaComponent(currentRectOverlayOpacity).cgColor)
        context.fill(localCurrentRect)

        drawKeys(in: localCurrentRect, opacity: pulseOpacity)
    }

    private func drawKeys(in rect: CGRect, opacity: CGFloat) {
        let rows = state.layout.rows
        let rowHeight = rect.height / CGFloat(rows.count)
        let cellInset: CGFloat = state.history.isEmpty ? 4 : 0
        let isFinalClickLayout = state.layout.id == "finalClick"
        let cellFillOpacity: CGFloat = (isFinalClickLayout ? 0.10 : 0.08) * opacity
        let cellStrokeOpacity: CGFloat = (isFinalClickLayout ? 0.24 : 0.20) * opacity
        let textOpacity: CGFloat = (isFinalClickLayout ? 0.80 : 0.94) * opacity
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

                let rounded = NSBezierPath(roundedRect: cell, xRadius: 8, yRadius: 8)
                NSColor.white.withAlphaComponent(cellFillOpacity).setFill()
                rounded.fill()

                NSColor.white.withAlphaComponent(cellStrokeOpacity).setStroke()
                rounded.lineWidth = 1
                rounded.stroke()

                let paragraph = NSMutableParagraphStyle()
                paragraph.alignment = .center

                let minimumFontSize: CGFloat = isFinalClickLayout ? 8 : 16
                let attrs: [NSAttributedString.Key: Any] = [
                    .font: NSFont.monospacedSystemFont(ofSize: max(minimumFontSize, min(cell.width, cell.height) * fontScale), weight: .bold),
                    .foregroundColor: NSColor.white.withAlphaComponent(textOpacity),
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
