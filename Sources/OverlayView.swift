import AppKit

@MainActor
final class OverlayView: NSView {
    var state: GridState = GridState(screenRect: .zero, currentRect: .zero, history: [], layout: .full) {
        didSet { needsDisplay = true }
    }

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

        context.setFillColor(NSColor.black.withAlphaComponent(0.10).cgColor)
        context.fill(localCurrentRect)

        let borderPath = NSBezierPath(rect: localCurrentRect)
        NSColor.systemGreen.withAlphaComponent(0.95).setStroke()
        borderPath.lineWidth = 3
        borderPath.stroke()

        drawKeys(in: localCurrentRect)
    }

    private func drawKeys(in rect: CGRect) {
        let rows = state.layout.rows
        let rowHeight = rect.height / CGFloat(rows.count)

        for (rowIndex, row) in rows.enumerated() {
            let cellWidth = rect.width / CGFloat(row.count)

            for (columnIndex, key) in row.enumerated() {
                let cell = CGRect(
                    x: rect.minX + CGFloat(columnIndex) * cellWidth,
                    y: rect.maxY - CGFloat(rowIndex + 1) * rowHeight,
                    width: cellWidth,
                    height: rowHeight
                ).insetBy(dx: 4, dy: 4)

                let rounded = NSBezierPath(roundedRect: cell, xRadius: 8, yRadius: 8)
                NSColor.white.withAlphaComponent(0.08).setFill()
                rounded.fill()

                NSColor.white.withAlphaComponent(0.20).setStroke()
                rounded.lineWidth = 1
                rounded.stroke()

                let paragraph = NSMutableParagraphStyle()
                paragraph.alignment = .center

                let attrs: [NSAttributedString.Key: Any] = [
                    .font: NSFont.monospacedSystemFont(ofSize: max(16, min(cell.width, cell.height) * 0.32), weight: .bold),
                    .foregroundColor: NSColor.white,
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

        let hint = "⌘⇧Space toggle · Esc cancel · Delete back · Space/Return click"
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
}
