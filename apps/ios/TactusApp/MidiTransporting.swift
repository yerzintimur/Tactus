import Foundation

/// The transport surface `CoreSession` drives — the seam between the sans-I/O
/// core and whatever moves the bytes. Two implementations:
///
/// - `MidiTransport` — the real CoreMIDI I/O (USB module);
/// - `SimulatedTransport` (DEBUG) — the core's `VirtualDeviceHandle`, so the full
///   pipeline runs with no hardware (device-mock plan, B1).
///
/// The core never knows which one it is talking to: both deliver complete inbound
/// MIDI via `onReceive` and take complete outbound messages via `send`.
@MainActor
protocol MidiTransporting: AnyObject {
    /// Inbound MIDI bytes (one or more packets' worth).
    var onReceive: ((Data) -> Void)? { get set }
    /// `true` when a device is reachable (sources + a destination present).
    var onConnectionChange: ((Bool) -> Void)? { get set }
    /// Current endpoint names (sources, destinations) after each scan.
    var onDevices: (([String], [String]) -> Void)? { get set }

    /// Open the transport and report the initial state. Idempotent.
    func start()
    /// Send one complete outbound MIDI message.
    func send(_ bytes: Data)
    /// Re-scan endpoints (debug affordance).
    func rescanNow()
}

extension MidiTransport: MidiTransporting {}
