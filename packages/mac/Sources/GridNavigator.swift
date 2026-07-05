import Foundation
import CoreGraphics

struct GridLayout: Equatable {
    let id: String
    let rows: [[Character]]

    // Fallback only. In normal app flow this is overridden by KeyboardLayoutResolver.
    // Keep this in sync conceptually with KeyboardLayoutResolver.rootTemplate.
    static let full = GridLayout(id: "full", rows: [
        Array("1234567890"),
        Array("qwertyuiop"),
        Array("asdfghjklñ"),
        Array("zxcvbnm,./")
    ])

    // Fallback only. In normal app flow this is overridden by KeyboardLayoutResolver.
    // Keep this in sync conceptually with KeyboardLayoutResolver.refinementTemplate.
    static let refinement = GridLayout(id: "refinement", rows: [
        Array("1234567890"),
        Array("qwertyuiop"),
        Array("asdfghjklñ"),
        Array("zxcvbnm,./")
    ])

    // Fallback only. In normal app flow this is overridden by KeyboardLayoutResolver.
    // Keep this in sync conceptually with KeyboardLayoutResolver.finalClickTemplate.
    static let finalClick = GridLayout(id: "finalClick", rows: [
        Array("1234567890"),
        Array("qwertyuiop"),
        Array("asdfghjklñ"),
        Array("zxcvbnm,./")
    ])

    var maxColumns: Int {
        rows.map(\.count).max() ?? 1
    }

    func rect(for key: Character, in bounds: CGRect) -> CGRect? {
        let lowercased = Character(String(key).lowercased())
        let totalRows = CGFloat(rows.count)
        let cellHeight = bounds.height / totalRows

        for (rowIndex, row) in rows.enumerated() {
            guard let columnIndex = row.firstIndex(of: lowercased) else { continue }

            let cellWidth = bounds.width / CGFloat(row.count)
            let x = bounds.minX + CGFloat(columnIndex) * cellWidth
            let y = bounds.maxY - CGFloat(rowIndex + 1) * cellHeight

            return CGRect(x: x, y: y, width: cellWidth, height: cellHeight)
        }

        return nil
    }

    func position(for key: Character) -> (row: Int, column: Int)? {
        let lowercased = Character(String(key).lowercased())

        for (rowIndex, row) in rows.enumerated() {
            guard let columnIndex = row.firstIndex(of: lowercased) else { continue }
            return (row: rowIndex, column: columnIndex)
        }

        return nil
    }

    func key(atRow row: Int, column: Int) -> Character? {
        guard rows.indices.contains(row), rows[row].indices.contains(column) else { return nil }
        return rows[row][column]
    }
}

struct GridState {
    let screenRect: CGRect
    let currentRect: CGRect
    let history: [CGRect]
    let layout: GridLayout
}

final class GridNavigator {
    private let refinementWidthExpansionRatio: CGFloat = 0.5
    private let finalClickBaseWidthExpansionRatio: CGFloat = 0.5
    private let finalClickLaptopHeightExpansionRatio: CGFloat = 0.08
    private let finalClickDesktopHeightExpansionRatio: CGFloat = 0.03
    private let finalClickLaptopTargetKeyWidth: CGFloat = 22
    private let finalClickDesktopTargetKeyWidth: CGFloat = 19
    private let finalClickMaxScreenWidthFraction: CGFloat = 0.4
    private let compactScreenWidthThreshold: CGFloat = 1600

    private var rootLayout: GridLayout
    private var refinementLayout: GridLayout
    private var finalClickLayout: GridLayout
    private(set) var state: GridState

    init(rootLayout: GridLayout = .full, refinementLayout: GridLayout = .refinement, finalClickLayout: GridLayout = .finalClick) {
        self.rootLayout = rootLayout
        self.refinementLayout = refinementLayout
        self.finalClickLayout = finalClickLayout
        let zero = CGRect.zero
        self.state = GridState(screenRect: zero, currentRect: zero, history: [], layout: rootLayout)
    }

    func configureLayouts(root: GridLayout, refinement: GridLayout, finalClick: GridLayout) {
        self.rootLayout = root
        self.refinementLayout = refinement
        self.finalClickLayout = finalClick
    }

    func reset(to screenRect: CGRect) {
        state = GridState(screenRect: screenRect, currentRect: screenRect, history: [], layout: layout(forDepth: 0))
    }

    @discardableResult
    func select(_ key: Character) -> Bool {
        guard let selectedRect = state.layout.rect(for: key, in: state.currentRect) else {
            return false
        }

        select(rect: selectedRect)
        return true
    }

    func select(rect: CGRect) {
        var history = state.history
        history.append(state.currentRect)

        let nextDepth = history.count
        let nextRect = expandedRectIfNeeded(rect, forDepth: nextDepth)

        state = GridState(
            screenRect: state.screenRect,
            currentRect: nextRect,
            history: history,
            layout: layout(forDepth: nextDepth)
        )
    }

    func back() {
        guard let previous = state.history.last else { return }
        var history = state.history
        history.removeLast()
        state = GridState(
            screenRect: state.screenRect,
            currentRect: previous,
            history: history,
            layout: layout(forDepth: history.count)
        )
    }

    var expectsClickOnNextSelection: Bool {
        state.layout == finalClickLayout
    }

    private func layout(forDepth depth: Int) -> GridLayout {
        switch depth {
        case 0: rootLayout
        case 1: refinementLayout
        default: finalClickLayout
        }
    }

    private func expandedRectIfNeeded(_ rect: CGRect, forDepth depth: Int) -> CGRect {
        let widthExpansionRatio: CGFloat
        let heightExpansionRatio: CGFloat

        switch depth {
        case 1:
            widthExpansionRatio = refinementWidthExpansionRatio
            heightExpansionRatio = 0
        case 2:
            let compactScreenFactor = max(0, min(1, (compactScreenWidthThreshold - state.screenRect.width) / 500))
            let targetKeyWidth = finalClickDesktopTargetKeyWidth + ((finalClickLaptopTargetKeyWidth - finalClickDesktopTargetKeyWidth) * compactScreenFactor)
            let heightBoost = finalClickDesktopHeightExpansionRatio + ((finalClickLaptopHeightExpansionRatio - finalClickDesktopHeightExpansionRatio) * compactScreenFactor)
            let baseWidth = rect.width * (1 + 2 * finalClickBaseWidthExpansionRatio)
            let targetWidthFromKeys = targetKeyWidth * CGFloat(finalClickLayout.maxColumns)
            let maxAllowedWidth = state.screenRect.width * finalClickMaxScreenWidthFraction
            let desiredWidth = min(max(baseWidth, targetWidthFromKeys), maxAllowedWidth)
            widthExpansionRatio = max(0, (desiredWidth / max(rect.width, 1) - 1) / 2)
            heightExpansionRatio = heightBoost
        default:
            widthExpansionRatio = 0
            heightExpansionRatio = 0
        }

        guard widthExpansionRatio > 0 || heightExpansionRatio > 0 else { return rect }

        let expanded = rect.insetBy(
            dx: -rect.width * widthExpansionRatio,
            dy: -rect.height * heightExpansionRatio
        )

        return expanded.intersection(state.screenRect)
    }
}
