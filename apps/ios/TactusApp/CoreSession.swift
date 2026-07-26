import Foundation
import Tactus

/// The single bridge between SwiftUI and the sans-I/O Rust core.
///
/// The core is pure logic: every call returns a list of `Effect`s that the host
/// must perform (send MIDI, schedule a tick, emit an event). This class drains
/// that list — forwarding outbound MIDI to the transport (CoreMIDI, task #13),
/// scheduling ticks, and projecting emitted events into `@Published` UI state.
///
/// The core's `.speak` events carry localized announcement text; this class posts
/// them to the system screen reader via `AnnouncementService` — the app has no TTS
/// of its own (ADR-0014). `.earcon` events become haptics/sounds via `EarconService`.
@MainActor
final class CoreSession: ObservableObject {
    @Published private(set) var connection: ConnectionState = .disconnected
    @Published private(set) var device: DeviceInfo?
    @Published private(set) var currentKit: String?
    @Published private(set) var currentKitNumber: UInt32?
    /// The active kit's tempo parameter (value + range/scale), projected from the
    /// core's snapshot. `nil` when no profile exposes it (e.g. unknown device).
    /// The value is the last value the device confirmed — never edit intent.
    @Published private(set) var tempo: ParameterView?
    /// The set list currently open, as the module reports it: the kits in playing
    /// order, ending at the module's own terminator. `nil` until one is opened.
    @Published private(set) var setlist: SetlistView?
    /// True while a tempo edit is in flight (written, not yet device-confirmed).
    /// The UI presents the edit as *in-progress* so the screen reader never voices
    /// the stale value as current (ADR-0014 edge case); the displayed number stays
    /// the last device-confirmed value — never intent (no blind writes).
    @Published private(set) var tempoEditInFlight = false
    /// Most recent announcement text, mirrored for the UI/debug.
    @Published private(set) var lastAnnouncement: String = ""
    @Published private(set) var log: [String] = []
    /// Debug MIDI diagnostics: the endpoint names CoreMIDI currently reports.
    @Published private(set) var midiSources: [String] = []
    @Published private(set) var midiDestinations: [String] = []

    /// The language the core is currently speaking and labelling in. Published
    /// so a change re-renders every view that asks for interface text.
    @Published private(set) var locale: String

    private let core: TactusSession
    private let transport: any MidiTransporting
    private let announcements = AnnouncementService()
    private let earcons = EarconService()

    /// Set by `startMidi()` to the transport's sender. When nil (e.g. before
    /// startup, or in previews), outbound MIDI is logged instead of sent.
    var sendMidi: ((Data) -> Void)?

    /// `transport` defaults to the real CoreMIDI transport; tests and the
    /// `--simulated-device` launch path inject a `SimulatedTransport` instead.
    init(
        locale: String = CoreSession.preferredLanguage(),
        transport: (any MidiTransporting)? = nil
    ) {
        core = TactusSession(locale: locale)
        self.locale = locale
        self.transport = transport ?? MidiTransport()
    }

    /// Wire up CoreMIDI and start listening. Call once when the app appears.
    /// Endpoint availability drives connect/disconnect; inbound bytes are fed to
    /// the core; the core's outbound MIDI is sent through the transport.
    func startMidi() {
        transport.onReceive = { [weak self] bytes in self?.receive(bytes) }
        transport.onConnectionChange = { [weak self] available in
            if available { self?.connected() } else { self?.disconnected() }
        }
        transport.onDevices = { [weak self] sources, destinations in
            self?.midiSources = sources
            self?.midiDestinations = destinations
        }
        sendMidi = { [weak self] bytes in self?.transport.send(bytes) }
        transport.start()
    }

    /// Manually re-scan MIDI endpoints (debug affordance).
    func rescanMidi() { transport.rescanNow() }

    // MARK: - Inbound events (call these from the transport / UI)

    func connected() { perform(core.onConnected()) }
    func disconnected() { perform(core.onDisconnected()) }
    func receive(_ bytes: Data) { perform(core.handleMidiInput(bytes: bytes)) }
    func selectKit(_ number: UInt32) { perform(core.selectKit(number: number)) }
    func renameKit(_ number: UInt32, to name: String) { perform(core.renameKit(number: number, name: name)) }

    /// Nudge the active kit's tempo by `rawSteps` smallest increments (1 step =
    /// 0.1 BPM). Clamped to the parameter's range; routed through the core's
    /// write→read-back→verify pipeline, so the displayed value and the spoken
    /// confirmation are the **actual stored** value, never the intended one.
    func adjustTempo(rawSteps: Int) {
        guard let tempo,
            case let .int(raw)? = tempo.value,
            let range = tempo.numeric?.range,
            let kit = currentKitNumber
        else { return }
        let target = max(range.rawMin, min(range.rawMax, raw + Int64(rawSteps) * range.rawStep))
        guard target != raw else { return }
        // Mark in-flight *before* performing: a synchronously failing edit clears
        // it again via the emitted EditFailed.
        tempoEditInFlight = true
        perform(core.setParameter(paramId: tempo.paramId, indices: [kit], value: target))
    }

    /// The active kit's current raw tempo, if the device has reported it.
    var tempoRawValue: Int64? {
        if case let .int(value)? = tempo?.value { return value }
        return nil
    }

    var tempoAtMinimum: Bool {
        guard let raw = tempoRawValue, let range = tempo?.numeric?.range else { return false }
        return raw <= range.rawMin
    }

    var tempoAtMaximum: Bool {
        guard let raw = tempoRawValue, let range = tempo?.numeric?.range else { return false }
        return raw >= range.rawMax
    }

    /// Open a set list (0-based) for reading and rearranging. Its contents arrive
    /// from the module; `setlist` is republished as they land.
    func readSetlist(_ number: UInt32) { perform(core.readSetlist(number: number)) }
    /// Add the kit the module is currently on to the end of the open set list —
    /// the eyes-closed way to build one: play a kit, keep it.
    func appendSetlistStep(kit: UInt32) { perform(core.appendSetlistStep(kit: kit)) }
    func removeSetlistStep(_ step: UInt32) { perform(core.removeSetlistStep(step: step)) }
    func swapSetlistSteps(_ a: UInt32, _ b: UInt32) {
        perform(core.swapSetlistSteps(a: a, b: b))
    }
    func renameSetlist(to name: String) { perform(core.renameSetlist(name: name)) }

    /// Step to the adjacent kit. The core knows both the current kit and how many
    /// the module has, so the bounds live there — at the first/last kit it writes
    /// nothing and announces the edge, and every platform gets that for free.
    func nextKit() { perform(core.nextKit()) }
    func previousKit() { perform(core.previousKit()) }
    /// Switch the language of everything the core produces — announcements and
    /// the interface text below. Publishing the change re-renders the views, so
    /// the whole UI switches language at once.
    func setLocale(_ locale: String) {
        core.setLocale(locale: locale)
        self.locale = locale
    }

    /// The user's explicit language choice, or nil to follow the device.
    /// Persisted, because a blind user should not have to re-pick it every launch.
    var languageOverride: String? {
        get { UserDefaults.standard.string(forKey: Self.languageOverrideKey) }
        set {
            let defaults = UserDefaults.standard
            if let newValue {
                defaults.set(newValue, forKey: Self.languageOverrideKey)
            } else {
                defaults.removeObject(forKey: Self.languageOverrideKey)
            }
            setLocale(newValue ?? Self.currentLanguage())
        }
    }

    /// The languages the core can render, each under its own name.
    var availableLocales: [LocaleOption] { core.availableLocales() }

    private static let languageOverrideKey = "languageOverride"

    /// The app's own interface text, localized by the core rather than by
    /// platform string files (ADR-0008): in a nonvisual app a control label is
    /// read aloud, so it belongs in the same tested source as the announcements.
    func text(_ string: UiString, _ value: String? = nil) -> String {
        core.uiString(string: string, value: value)
    }
    func tick() { perform(core.tick(nowMs: Self.nowMs())) }

    // MARK: - Effect handling

    private func perform(_ effects: [Effect]) {
        for effect in effects {
            switch effect {
            case .sendMidi(let bytes):
                if let sendMidi {
                    sendMidi(bytes)
                } else {
                    append("→ MIDI \(Self.hex(bytes))")
                }
            case .scheduleTick(let afterMs):
                scheduleTick(afterMs: afterMs)
            case .emit(let event):
                apply(event)
            }
        }
        refreshViewModel()
    }

    /// The parameter id of the tempo control (also used by `refreshViewModel`).
    private static let tempoParamId = "kit.common.tempo"

    /// Re-pull the core's snapshot and project the bits the UI binds to. Cheap
    /// (a small in-memory build); the snapshot holds the last device-confirmed
    /// values, so the UI never shows unverified edit intent.
    private func refreshViewModel() {
        let snapshot = core.snapshot()
        tempo = snapshot.parameters.first { $0.paramId == Self.tempoParamId }
        setlist = snapshot.setlist
    }

    private func apply(_ event: CoreEvent) {
        switch event {
        case .connectionChanged(let state):
            connection = state
            if state != .ready { tempoEditInFlight = false }
        case .deviceIdentified(let info):
            device = info
            append("device: \(info.name) — fw \(info.firmware)")
        case .currentKitChanged(let number, let name):
            // A kit switch abandons any in-flight tempo edit UI-wise (the engine
            // resolves the pipeline itself; the new kit's tempo read repopulates).
            tempoEditInFlight = false
            currentKitNumber = number
            currentKit = name
        case .setlistChanged(let number):
            append("set list \(number + 1) updated")
        case .editConfirmed(let paramId, let display):
            if paramId == Self.tempoParamId { tempoEditInFlight = false }
            append("✓ \(display)")
        case .editFailed(let paramId, let reason):
            if paramId == Self.tempoParamId { tempoEditInFlight = false }
            append("✗ \(reason)")
        case .speak(let speech):
            lastAnnouncement = speech.text
            announcements.announce(speech)
            append("🔊 \(speech.text)")
        case .earcon(let earcon):
            earcons.play(earcon)
            append("🔔 \(earcon)")
        case .error(let message):
            append("error: \(message)")
        }
    }

    /// The core asks us to call `tick` again after a delay (polling, retries).
    private func scheduleTick(afterMs: UInt64) {
        Task { [weak self] in
            try? await Task.sleep(nanoseconds: afterMs * 1_000_000)
            self?.tick()
        }
    }

    // MARK: - Helpers

    private func append(_ line: String) {
        log.append(line)
        if log.count > 200 { log.removeFirst(log.count - 200) }
    }

    private static func hex(_ data: Data) -> String {
        data.map { String(format: "%02X", $0) }.joined(separator: " ")
    }

    /// Monotonic millisecond clock for the engine's timers.
    private static func nowMs() -> UInt64 {
        UInt64(DispatchTime.now().uptimeNanoseconds / 1_000_000)
    }

    /// The language to start in: the user's saved choice, otherwise the device's.
    static func preferredLanguage() -> String {
        UserDefaults.standard.string(forKey: languageOverrideKey) ?? currentLanguage()
    }

    /// Forget a saved language choice (UI tests, which must start from a known
    /// state — the override survives app launches by design).
    static func clearLanguageOverride() {
        UserDefaults.standard.removeObject(forKey: languageOverrideKey)
    }

    /// Core localisation expects a bare language code ("en"/"ru"), not "en_US".
    private static func currentLanguage() -> String {
        Locale.current.language.languageCode?.identifier ?? "en"
    }

    /// A canned V31 Identity Reply (family 01 06, member 03 00) for running the
    /// pipeline in the Simulator before the CoreMIDI transport (task #13) lands.
    static let sampleV31IdentityReply = Data([
        0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x01, 0x06, 0x03, 0x00, 0x00, 0x02, 0x00, 0x00, 0xF7,
    ])
}
