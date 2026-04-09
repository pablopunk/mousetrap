import AppKit
import Combine
import KeyboardShortcuts

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let overlayController = OverlayWindowController()
    private let screenResolver = FocusedScreenResolver()
    private let permissionManager = PermissionManager()
    private let settings = SettingsStore.shared
    private var settingsCancellables = Set<AnyCancellable>()
    private var previouslyFocusedApp: NSRunningApplication?
    private let keyboardLayoutResolver = KeyboardLayoutResolver()
    private let overlayKeyboardInterceptor = KeyboardInterceptor()
    private let freeMouseKeyboardInterceptor = KeyboardInterceptor()
    private let mouseMovementInterceptor = MouseMovementInterceptor()
    private let freeMouseIndicatorController = FreeMouseIndicatorController()
    private lazy var navigator = GridNavigator()
    private var pendingFreeMouseClickWorkItem: DispatchWorkItem?
    private var pendingFreeMouseClickPoint: CGPoint?
    private let freeMouseDoubleTapWindow: TimeInterval = 0.25
    private let finalClickChordGracePeriod: TimeInterval = 0.08
    private var finalClickHeldKeys = Set<Character>()
    private var finalClickSessionKeys = Set<Character>()
    private var finalClickKeyOrder = [Character]()
    private var pendingFinalClickCommitWorkItem: DispatchWorkItem?
    private var finalClickPreviewKeys = Set<Character>()
    private var finalClickPreviewPoint: CGPoint?
    private var unsafeStateTimer: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        overlayKeyboardInterceptor.onKey = { [weak self] key in
            self?.handleInterceptedKey(key)
        }
        overlayKeyboardInterceptor.onKeyUp = { [weak self] key in
            self?.handleInterceptedKeyUp(key)
        }
        overlayController.onKey = { [weak self] key in
            self?.handleInterceptedKey(key)
        }
        freeMouseKeyboardInterceptor.onKey = { [weak self] key in
            self?.handleFreeMouseKey(key)
        }
        mouseMovementInterceptor.onMouseMovement = { [weak self] event in
            self?.handleObservedMouseMovement(event)
        }
        mouseMovementInterceptor.start()

        KeyboardShortcuts.onKeyDown(for: .activateMousetrap) { [weak self] in
            self?.toggleOverlay()
        }
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        restoreMenuBarIconIfNeeded()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        restoreMenuBarIconIfNeeded()
        return true
    }

    func applicationWillTerminate(_ notification: Notification) {
        cancelUnsafeStateTimeout()
        mouseMovementInterceptor.stop()
    }

    private var isInUnsafeState: Bool {
        overlayController.isVisible || freeMouseKeyboardInterceptor.isActive
    }

    private func toggleOverlay() {
        if freeMouseKeyboardInterceptor.isActive {
            deactivateFreeMouseMode()
        } else if overlayController.isVisible {
            deactivateOverlay()
        } else {
            activateOverlay()
            noteUnsafeActivity()
        }
    }

    private func activateOverlay() {
        guard permissionManager.hasAccessibilityPermission else {
            permissionManager.openAccessibilitySettings()
            return
        }

        previouslyFocusedApp = NSWorkspace.shared.frontmostApplication

        let layouts = keyboardLayoutResolver.resolveLayouts()
        navigator.configureLayouts(root: layouts.root, refinement: layouts.refinement, finalClick: layouts.finalClick)

        let screen = screenResolver.resolveFocusedScreen() ?? NSScreen.main ?? NSScreen.screens.first
        guard let screen else { return }

        navigator.reset(to: screen.frame)
        resetFinalClickInteraction()
        overlayKeyboardInterceptor.start()
        overlayController.show(on: screen, state: navigator.state)
    }

    private func deactivateOverlay(restoreFocus: Bool = true) {
        cancelPendingFinalClickCommit()
        resetFinalClickInteraction()
        overlayKeyboardInterceptor.stop()
        overlayController.hide()

        if !isInUnsafeState {
            cancelUnsafeStateTimeout()
        }

        guard restoreFocus, let previouslyFocusedApp else {
            if !isInUnsafeState {
                self.previouslyFocusedApp = nil
            }
            return
        }

        self.previouslyFocusedApp = nil
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
            previouslyFocusedApp.activate()
        }
    }

    private func handleInterceptedKey(_ key: InterceptedKey) {
        noteUnsafeActivity()

        switch key {
        case .escape:
            deactivateOverlay()
        case .delete:
            cancelPendingFinalClickCommit()
            resetFinalClickInteraction()
            navigator.back()
            updateOverlayAndCursor()
        case .returnKey:
            MouseController.leftClick()
            deactivateOverlay()
        case .shiftReturnKey:
            MouseController.rightClick()
            deactivateOverlay()
        case .space:
            MouseController.leftClick()
            deactivateOverlay()
        case .upArrow, .downArrow, .leftArrow, .rightArrow, .shiftUpArrow, .shiftDownArrow, .shiftLeftArrow, .shiftRightArrow:
            startFreeMouseMode(withInitialMove: key)
        case .character(let character):
            handleGridSelectionKeyDown(character)
        }
    }

    private func handleInterceptedKeyUp(_ key: InterceptedKey) {
        noteUnsafeActivity()

        guard case .character(let character) = key else { return }
        handleGridSelectionKeyUp(character)
    }

    private func handleGridSelectionKeyDown(_ character: Character) {
        guard navigator.state.layout.rect(for: character, in: navigator.state.currentRect) != nil else { return }

        cancelPendingFinalClickCommit()
        finalClickHeldKeys.insert(character)
        finalClickSessionKeys.insert(character)
        finalClickKeyOrder.removeAll(where: { $0 == character })
        finalClickKeyOrder.append(character)
        updateGridSelectionPreview()
    }

    private func handleGridSelectionKeyUp(_ character: Character) {
        guard finalClickHeldKeys.contains(character) || finalClickSessionKeys.contains(character) else { return }

        finalClickHeldKeys.remove(character)
        overlayController.updateInteraction(
            pressedKeys: finalClickHeldKeys,
            previewKeys: finalClickPreviewKeys,
            previewPoint: finalClickPreviewPoint
        )

        guard finalClickHeldKeys.isEmpty else { return }
        scheduleGridSelectionCommit()
    }

    private func updateGridSelectionPreview() {
        let selection = bestFinalClickSelection(from: finalClickSessionKeys)
        finalClickPreviewKeys = selection?.keys ?? []
        finalClickPreviewPoint = selection?.target
        overlayController.updateInteraction(
            pressedKeys: finalClickHeldKeys,
            previewKeys: finalClickPreviewKeys,
            previewPoint: finalClickPreviewPoint
        )
    }

    private func scheduleGridSelectionCommit() {
        guard let selection = bestFinalClickSelection(from: finalClickSessionKeys) else {
            resetFinalClickInteraction()
            return
        }

        let shouldClick = navigator.expectsClickOnNextSelection
        let workItem = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.pendingFinalClickCommitWorkItem = nil

            if shouldClick {
                MouseController.leftClick(at: selection.target)
                self.deactivateOverlay()
            } else {
                self.navigator.select(rect: self.refinementRect(for: selection))
                self.updateOverlayAndCursor()
            }
        }

        pendingFinalClickCommitWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + finalClickChordGracePeriod, execute: workItem)
    }

    private func cancelPendingFinalClickCommit() {
        pendingFinalClickCommitWorkItem?.cancel()
        pendingFinalClickCommitWorkItem = nil
    }

    private func resetFinalClickInteraction() {
        finalClickHeldKeys = []
        finalClickSessionKeys = []
        finalClickKeyOrder = []
        finalClickPreviewKeys = []
        finalClickPreviewPoint = nil
        overlayController.updateInteraction(pressedKeys: [], previewKeys: [], previewPoint: nil)
    }

    private func bestFinalClickSelection(from keys: Set<Character>) -> (keys: Set<Character>, rect: CGRect, target: CGPoint, baseCellSize: CGSize)? {
        guard !keys.isEmpty else { return nil }

        let layout = navigator.state.layout
        let bounds = navigator.state.currentRect
        let keyOrderIndex = Dictionary(uniqueKeysWithValues: finalClickKeyOrder.enumerated().map { ($0.element, $0.offset) })
        let latestKey = finalClickKeyOrder.last

        var bestKeys = Set<Character>()
        var bestRect: CGRect?
        var bestScore = (-1, -1, -1)

        let positions = keys.compactMap { key -> (Character, Int, Int)? in
            guard let position = layout.position(for: key) else { return nil }
            return (key, position.row, position.column)
        }

        let rows = Array(Set(positions.map { $0.1 })).sorted()
        let columns = Array(Set(positions.map { $0.2 })).sorted()

        for minRow in rows {
            for maxRow in rows where maxRow >= minRow {
                for minColumn in columns {
                    for maxColumn in columns where maxColumn >= minColumn {
                        var candidateKeys = Set<Character>()
                        var candidateRects = [CGRect]()
                        var unionRect: CGRect?
                        var isValid = true

                        for row in minRow...maxRow {
                            for column in minColumn...maxColumn {
                                guard let key = layout.key(atRow: row, column: column),
                                      keys.contains(key),
                                      let rect = layout.rect(for: key, in: bounds) else {
                                    isValid = false
                                    break
                                }

                                candidateKeys.insert(key)
                                candidateRects.append(rect)
                                unionRect = unionRect.map { $0.union(rect) } ?? rect
                            }

                            if !isValid {
                                break
                            }
                        }

                        guard isValid, let unionRect else { continue }

                        let containsLatestKey = latestKey.map { candidateKeys.contains($0) } == true ? 1 : 0
                        let recencyScore = candidateKeys.reduce(0) { partialResult, key in
                            partialResult + (keyOrderIndex[key] ?? 0)
                        }
                        let score = (candidateKeys.count, containsLatestKey, recencyScore)

                        if score > bestScore {
                            bestScore = score
                            bestKeys = candidateKeys
                            bestRect = unionRect
                        }
                    }
                }
            }
        }

        guard !bestKeys.isEmpty, let bestRect else { return nil }

        let cellRects = bestKeys.compactMap { layout.rect(for: $0, in: bounds) }
        let averageWidth = cellRects.reduce(CGFloat.zero) { $0 + $1.width } / CGFloat(max(cellRects.count, 1))
        let averageHeight = cellRects.reduce(CGFloat.zero) { $0 + $1.height } / CGFloat(max(cellRects.count, 1))
        let baseCellSize = CGSize(width: averageWidth, height: averageHeight)

        return (keys: bestKeys, rect: bestRect, target: bestRect.center, baseCellSize: baseCellSize)
    }

    private func refinementRect(for selection: (keys: Set<Character>, rect: CGRect, target: CGPoint, baseCellSize: CGSize)) -> CGRect {
        let bounds = navigator.state.currentRect
        let width = min(selection.baseCellSize.width, bounds.width)
        let height = min(selection.baseCellSize.height, bounds.height)

        let minX = bounds.minX
        let maxX = bounds.maxX - width
        let minY = bounds.minY
        let maxY = bounds.maxY - height

        let originX = min(max(selection.target.x - width / 2, minX), maxX)
        let originY = min(max(selection.target.y - height / 2, minY), maxY)

        return CGRect(x: originX, y: originY, width: width, height: height)
    }

    private func handleFreeMouseKey(_ key: InterceptedKey) {
        noteUnsafeActivity()

        switch key {
        case .escape:
            cancelPendingFreeMouseClick()
            deactivateFreeMouseMode()
        case .returnKey, .space:
            MouseController.endLeftDrag()
            handleFreeMouseReturn()
        case .shiftReturnKey:
            MouseController.endLeftDrag()
            let clickPoint = MouseController.currentCursorPositionAppKit
            cancelPendingFreeMouseClick()
            deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.rightClick(at: clickPoint)
            }
        case .upArrow, .downArrow, .leftArrow, .rightArrow:
            MouseController.endLeftDrag()
            cancelPendingFreeMouseClick()
            if MouseController.moveFreeCursor(direction: key) {
                freeMouseIndicatorController.updatePosition(to: MouseController.currentCursorPositionAppKit)
            }
        case .shiftUpArrow, .shiftDownArrow, .shiftLeftArrow, .shiftRightArrow:
            cancelPendingFreeMouseClick()
            MouseController.beginLeftDrag()
            if MouseController.moveFreeCursor(direction: baseArrowKey(for: key)) {
                freeMouseIndicatorController.updatePosition(to: MouseController.currentCursorPositionAppKit)
            }
        case .character, .delete:
            MouseController.endLeftDrag()
            break
        }
    }

    private func startFreeMouseMode(withInitialMove key: InterceptedKey) {
        freeMouseKeyboardInterceptor.start()
        deactivateOverlay(restoreFocus: true)

        switch key {
        case .shiftUpArrow, .shiftDownArrow, .shiftLeftArrow, .shiftRightArrow:
            MouseController.beginLeftDrag()
            if MouseController.moveFreeCursor(direction: baseArrowKey(for: key)) {
                freeMouseIndicatorController.show(at: MouseController.currentCursorPositionAppKit)
            }
        default:
            if MouseController.moveFreeCursor(direction: key) {
                freeMouseIndicatorController.show(at: MouseController.currentCursorPositionAppKit)
            }
        }

        noteUnsafeActivity()
    }

    private func deactivateFreeMouseMode() {
        cancelPendingFreeMouseClick()
        MouseController.endLeftDrag()
        freeMouseKeyboardInterceptor.stop()
        freeMouseIndicatorController.hide()

        if !isInUnsafeState {
            cancelUnsafeStateTimeout()
        }
    }

    private func handleFreeMouseReturn() {
        let clickPoint = MouseController.currentCursorPositionAppKit

        if pendingFreeMouseClickWorkItem != nil {
            cancelPendingFreeMouseClick()
            deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.doubleClick(at: clickPoint)
            }
            return
        }

        pendingFreeMouseClickPoint = clickPoint
        let workItem = DispatchWorkItem { [weak self] in
            guard let self, let clickPoint = self.pendingFreeMouseClickPoint else { return }
            self.pendingFreeMouseClickWorkItem = nil
            self.pendingFreeMouseClickPoint = nil
            self.deactivateFreeMouseMode()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                MouseController.leftClick(at: clickPoint)
            }
        }

        pendingFreeMouseClickWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + freeMouseDoubleTapWindow, execute: workItem)
    }

    private func cancelPendingFreeMouseClick() {
        pendingFreeMouseClickWorkItem?.cancel()
        pendingFreeMouseClickWorkItem = nil
        pendingFreeMouseClickPoint = nil
    }

    private func updateOverlayAndCursor() {
        resetFinalClickInteraction()
        overlayController.update(state: navigator.state)
        MouseController.moveCursor(to: navigator.state.currentRect.center)
    }

    private func baseArrowKey(for key: InterceptedKey) -> InterceptedKey {
        switch key {
        case .shiftUpArrow:
            .upArrow
        case .shiftDownArrow:
            .downArrow
        case .shiftLeftArrow:
            .leftArrow
        case .shiftRightArrow:
            .rightArrow
        default:
            key
        }
    }

    private func noteUnsafeActivity() {
        guard isInUnsafeState else { return }

        unsafeStateTimer?.invalidate()
        unsafeStateTimer = Timer.scheduledTimer(withTimeInterval: TimeInterval(settings.unsafeStateTimeoutSeconds), repeats: false) { [weak self] _ in
            Task { @MainActor in
                self?.resetToSafeStateDueToTimeout()
            }
        }
    }

    private func handleObservedMouseMovement(_ event: CGEvent) {
        guard !MouseController.shouldIgnoreObservedMouseMovement(event) else { return }
        MouseController.clearTrackedCursorPosition()
        guard isInUnsafeState else { return }

        print("[Mousetrap] mouse movement detected, resetting to safe state")
        deactivateFreeMouseMode()
        deactivateOverlay()
    }

    private func cancelUnsafeStateTimeout() {
        unsafeStateTimer?.invalidate()
        unsafeStateTimer = nil
    }

    private func restoreMenuBarIconIfNeeded() {
        guard UserDefaults.standard.bool(forKey: "showMenuBarIcon") == false else { return }
        UserDefaults.standard.set(true, forKey: "showMenuBarIcon")
    }

    private func resetToSafeStateDueToTimeout() {
        guard isInUnsafeState else {
            cancelUnsafeStateTimeout()
            return
        }

        print("[Mousetrap] inactivity timeout reached, resetting to safe state")
        deactivateFreeMouseMode()
        deactivateOverlay()
    }
}
