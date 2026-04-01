import Carbon

final class GlobalHotKeyController {
    private var hotKeyRef: EventHotKeyRef?
    private var eventHandlerRef: EventHandlerRef?
    private let handler: () -> Void

    init(handler: @escaping () -> Void) {
        self.handler = handler
        register()
    }

    deinit {
        unregister()
    }

    private func register() {
        var hotKeyID = EventHotKeyID()
        hotKeyID.signature = OSType(0x4D547270) // MTrp
        hotKeyID.id = 1

        var eventType = EventTypeSpec()
        eventType.eventClass = OSType(kEventClassKeyboard)
        eventType.eventKind = UInt32(kEventHotKeyPressed)

        let selfPointer = Unmanaged.passUnretained(self).toOpaque()
        let callback: EventHandlerUPP = { _, _, userData in
            guard let userData else { return OSStatus(eventNotHandledErr) }
            let controller = Unmanaged<GlobalHotKeyController>.fromOpaque(userData).takeUnretainedValue()
            controller.handler()
            return noErr
        }

        InstallEventHandler(GetApplicationEventTarget(), callback, 1, &eventType, selfPointer, &eventHandlerRef)

        let modifiers = UInt32(cmdKey | shiftKey)
        let keyCode: UInt32 = 49 // space
        RegisterEventHotKey(keyCode, modifiers, hotKeyID, GetApplicationEventTarget(), 0, &hotKeyRef)
    }

    private func unregister() {
        if let hotKeyRef {
            UnregisterEventHotKey(hotKeyRef)
            self.hotKeyRef = nil
        }

        if let eventHandlerRef {
            RemoveEventHandler(eventHandlerRef)
            self.eventHandlerRef = nil
        }
    }
}
