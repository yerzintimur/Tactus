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
    /// Most recent spoken announcement (the future speech layer reads this).
    @Published private(set) var lastSpoken: String = ""
    @Published private(set) var log: [String] = []

    private let core: TactusSession

    /// Injected by the transport layer (task #13). Until then, outbound MIDI is
    /// logged instead of sent.
    var sendMidi: ((Data) -> Void)?

    init(locale: String = CoreSession.currentLanguage()) {
        core = TactusSession(locale: locale)
    }

    // MARK: - Inbound events (call these from the transport / UI)

    func connected() { perform(core.onConnected()) }
    func disconnected() { perform(core.onDisconnected()) }
    func receive(_ bytes: Data) { perform(core.handleMidiInput(bytes: bytes)) }
    func selectKit(_ number: UInt32) { perform(core.selectKit(number: number)) }
    func setLocale(_ locale: String) { core.setLocale(locale: locale) }
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
    }

    private func apply(_ event: CoreEvent) {
        switch event {
        case .connectionChanged(let state):
            connection = state
        case .deviceIdentified(let info):
            device = info
            append("device: \(info.name) — fw \(info.firmware)")
        case .currentKitChanged(_, let name):
            currentKit = name
        case .editConfirmed(_, let display):
            append("✓ \(display)")
        case .editFailed(_, let reason):
            append("✗ \(reason)")
        case .speak(let speech):
            lastSpoken = speech.text
            append("🔊 \(speech.text)")
        case .earcon(let earcon):
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
