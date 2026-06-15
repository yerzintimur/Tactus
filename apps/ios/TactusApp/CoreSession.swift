import Foundation
import Tactus

/// The single bridge between SwiftUI and the sans-I/O Rust core.
///
/// The core is pure logic: every call returns a list of `Effect`s that the host
/// must perform (send MIDI, schedule a tick, emit an event). This class drains
/// that list — forwarding outbound MIDI to the transport (CoreMIDI, task #13),
/// scheduling ticks, and projecting emitted events into `@Published` UI state.
///
/// Speech (`.speak`) is captured here for now; task #15 routes it to
/// `AVSpeechSynthesizer`. Until the transport exists, outbound MIDI is logged.
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
    /// Most recent spoken announcement, mirrored for the UI.
    @Published private(set) var lastSpoken: String = ""
    @Published private(set) var log: [String] = []
    /// Debug MIDI diagnostics: the endpoint names CoreMIDI currently reports.
    @Published private(set) var midiSources: [String] = []
    @Published private(set) var midiDestinations: [String] = []

    private let core: TactusSession
    private let transport = MidiTransport()
    private let speech: SpeechService
    private let earcons = EarconService()

    /// Set by `startMidi()` to the transport's sender. When nil (e.g. before
    /// startup, or in previews), outbound MIDI is logged instead of sent.
    var sendMidi: ((Data) -> Void)?

    init(locale: String = CoreSession.currentLanguage()) {
        core = TactusSession(locale: locale)
        speech = SpeechService(locale: locale)
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

    /// Step to the adjacent kit. The core verifies the result and announces the
    /// actual kit; an out-of-range request just fails (reported via EditFailed).
    func nextKit() { selectKit((currentKitNumber ?? 0) + 1) }
    func previousKit() {
        guard let number = currentKitNumber, number > 0 else { return }
        selectKit(number - 1)
    }
    func setLocale(_ locale: String) {
        core.setLocale(locale: locale)
        speech.setLocale(locale)
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

    /// Re-pull the core's snapshot and project the bits the UI binds to. Cheap
    /// (a small in-memory build); the snapshot holds the last device-confirmed
    /// values, so the UI never shows unverified edit intent.
    private func refreshViewModel() {
        tempo = core.snapshot().parameters.first { $0.paramId == "kit.common.tempo" }
    }

    private func apply(_ event: CoreEvent) {
        switch event {
        case .connectionChanged(let state):
            connection = state
        case .deviceIdentified(let info):
            device = info
            append("device: \(info.name) — fw \(info.firmware)")
        case .currentKitChanged(let number, let name):
            currentKitNumber = number
            currentKit = name
        case .editConfirmed(_, let display):
            append("✓ \(display)")
        case .editFailed(_, let reason):
            append("✗ \(reason)")
        case .speak(let speech):
            lastSpoken = speech.text
            self.speech.speak(speech)
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
