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

        let isFinalClickLayout = state.layout.id == "finalClick"
        let currentRectOverlayOpacity: CGFloat = isFinalClickLayout ? 0.03 : 0.10
        context.setFillColor(NSColor.black.withAlphaComponent(currentRectOverlayOpacity).cgColor)
        context.fill(localCurrentRect)

        drawKeys(in: localCurrentRect)
    }

    private func drawKeys(in rect: CGRect) {
        let rows = state.layout.rows
        let rowHeight = rect.height / CGFloat(rows.count)
        let cellInset: CGFloat = state.history.isEmpty ? 4 : 0
        let isFinalClickLayout = state.layout.id == "finalClick"
        let cellFillOpacity: CGFloat = isFinalClickLayout ? 0.04 : 0.08
        let cellStrokeOpacity: CGFloat = isFinalClickLayout ? 0.10 : 0.20
        let textOpacity: CGFloat = isFinalClickLayout ? 0.55 : 0.94
        let fontScale: CGFloat = isFinalClickLayout ? 0.20 : 0.32

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

                let attrs: [NSAttributedString.Key: Any] = [
                    .font: NSFont.monospacedSystemFont(ofSize: max(16, min(cell.width, cell.height) * fontScale), weight: .bold),
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
}
