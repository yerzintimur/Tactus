#if DEBUG
import Foundation
import Tactus

/// A drop-in `MidiTransporting` backed by the core's `VirtualDeviceHandle` — the
/// same profile-driven simulated module the Rust e2e harness drives (device-mock
/// plan, B1). The full pipeline (identify → poll → edit → verify) runs for real:
/// writes persist in the simulated state and read-backs verify them; replies are
/// delivered asynchronously after the device's modelled latency, so timing
/// behaviour (in-progress edits, announcement interruption) is exercised too.
///
/// Selected at launch with `--simulated-device` (the UI tests pass it). DEBUG
/// only, and the sim object exists only in dev bindings (`just build-ios`);
/// shipping bindings (`just build-ios-release`) contain no simulated device.
@MainActor
final class SimulatedTransport: MidiTransporting {
    var onReceive: ((Data) -> Void)?
    var onConnectionChange: ((Bool) -> Void)?
    var onDevices: (([String], [String]) -> Void)?

    private let device: VirtualDeviceHandle
    private let endpointName = "Simulated V31"

    init() {
        device = VirtualDeviceHandle.v31()
        // Neighbours of the boot kit (index 4), so kit navigation has real,
        // named places to land during tests and eyes-closed dev runs.
        device.seedKit(index: 3, name: "Blues", tempoRaw: 900)
        device.seedKit(index: 5, name: "Funk", tempoRaw: 1300)
    }

    func start() {
        onDevices?([endpointName], [endpointName])
        onConnectionChange?(true)
    }

    func rescanNow() {
        onDevices?([endpointName], [endpointName])
    }

    func send(_ bytes: Data) {
        let replies = device.respond(hostMsg: bytes)
        guard !replies.isEmpty else { return }
        deliver(replies, afterMs: device.replyDelayMs(request: bytes))
    }

    // MARK: - Hardware-side actions (drive from tests / dev UI)

    /// The user turns the kit dial on the (simulated) module.
    func hardwareSelectKit(_ index: UInt32) {
        deliver([device.hardwareSelectKit(index: index)], afterMs: device.pushDelayMs())
    }

    /// The user turns a knob on the (simulated) module.
    func hardwareEdit(_ paramId: String, indices: [UInt32], value: Int64) {
        deliver(
            [device.hardwareEditInt(paramId: paramId, indices: indices, value: value)],
            afterMs: device.pushDelayMs())
    }

    /// Deliver device→host messages after the modelled latency, like a real
    /// module answering over USB.
    private func deliver(_ messages: [Data], afterMs delay: UInt64) {
        Task { [weak self] in
            try? await Task.sleep(nanoseconds: delay * 1_000_000)
            for message in messages { self?.onReceive?(message) }
        }
    }
}
#endif
