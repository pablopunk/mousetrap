import Foundation
import CoreGraphics

struct GridLayout: Equatable {
    let id: String
    let rows: [[Character]]

    static let full = GridLayout(id: "full", rows: [
        Array("1234567890"),
        Array("qwertyuiop"),
        Array("asdfghjklñ"),
        Array("zxcvbnm,./")
    ])

    static let refinement = GridLayout(id: "refinement", rows: [
        Array("123789"),
        Array("qweiop"),
        Array("asdklñ"),
        Array("zxc,.-")
    ])

    static let finalClick = GridLayout(id: "finalClick", rows: [
        Array("qwp"),
        Array("asñ"),
        Array("zx-")
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
}

struct GridState {
    let screenRect: CGRect
    let currentRect: CGRect
    let history: [CGRect]
    let layout: GridLayout
}

final class GridNavigator {
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
        guard let nextRect = state.layout.rect(for: key, in: state.currentRect) else {
            return false
        }

        var history = state.history
        history.append(state.currentRect)
        state = GridState(
            screenRect: state.screenRect,
            currentRect: nextRect,
            history: history,
            layout: layout(forDepth: history.count)
        )
        return true
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
}
