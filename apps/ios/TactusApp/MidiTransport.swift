import CoreMIDI
import Foundation
import os

/// CoreMIDI transport: the app's only MIDI I/O. The Rust core is sans-I/O, so
/// this layer owns the actual ports and endpoints.
///
/// - Inbound: every MIDI source is connected to one input port; raw bytes
///   (including fragmented SysEx) are forwarded verbatim to `onReceive`. The core
///   reassembles SysEx itself, so we don't interpret anything here.
/// - Outbound: bytes from the core's `SendMidi` effects are sent to the chosen
///   destination.
/// - Lifecycle: a CoreMIDI notification block re-scans on endpoint add/remove and
///   reports availability via `onConnectionChange`.
///
/// CoreMIDI invokes its blocks on its own threads; everything is hopped to the
/// main actor before touching `on*` handlers (which feed the @MainActor core).
///
/// USB-C class-compliant MIDI needs no entitlement on iOS. The Simulator has no
/// MIDI endpoints, so this stays idle there — real validation is on device (#14).
@MainActor
final class MidiTransport {
    /// Forwarded inbound MIDI bytes (one or more packets' worth).
    var onReceive: ((Data) -> Void)?
    /// `true` when at least one source and one destination are present.
    var onConnectionChange: ((Bool) -> Void)?
    /// Reports the current endpoint names (sources, destinations) after each scan
    /// — used for the debug MIDI diagnostics panel.
    var onDevices: (([String], [String]) -> Void)?

    private var client = MIDIClientRef()
    private var inputPort = MIDIPortRef()
    private var outputPort = MIDIPortRef()
    private var destination = MIDIEndpointRef()
    private var connectedSources = Set<MIDIEndpointRef>()
    private var available = false

    private let log = Logger(subsystem: "app.tactus", category: "midi")

    /// Create the client and ports, then scan existing endpoints. Idempotent.
    func start() {
        guard client == 0 else { rescan(); return }

        let notify: MIDINotifyBlock = { [weak self] message in
            let kind = message.pointee.messageID
            guard kind == .msgObjectAdded || kind == .msgObjectRemoved || kind == .msgSetupChanged
            else { return }
            Task { @MainActor in self?.rescan() }
        }
        check(MIDIClientCreateWithBlock("Tactus" as CFString, &client, notify), "MIDIClientCreate")

        let readBlock: MIDIReadBlock = { [weak self] packetList, _ in
            let data = Self.bytes(from: packetList)
            guard !data.isEmpty else { return }
            Task { @MainActor in self?.onReceive?(data) }
        }
        check(
            MIDIInputPortCreateWithBlock(client, "Tactus Input" as CFString, &inputPort, readBlock),
            "MIDIInputPortCreate")
        check(
            MIDIOutputPortCreate(client, "Tactus Output" as CFString, &outputPort),
            "MIDIOutputPortCreate")

        rescan()
    }

    /// Manually re-scan endpoints (debug affordance, in case a setup-change
    /// notification was missed).
    func rescanNow() { rescan() }

    /// Send raw bytes (a complete SysEx message) to the chosen destination.
    func send(_ bytes: Data) {
        guard destination != 0, outputPort != 0 else {
            log.warning("send dropped: no destination")
            return
        }
        let payload = [UInt8](bytes)
        // Our outbound messages (Identity Request, RQ1, small DT1 writes) are well
        // under one MIDIPacket's 256-byte data capacity.
        guard payload.count <= 256 else {
            log.error("send dropped: \(payload.count) bytes exceeds single-packet limit")
            return
        }
        var packet = MIDIPacket()
        packet.timeStamp = 0
        packet.length = UInt16(payload.count)
        withUnsafeMutableBytes(of: &packet.data) { raw in
            for (i, byte) in payload.enumerated() { raw[i] = byte }
        }
        var list = MIDIPacketList(numPackets: 1, packet: packet)
        check(MIDISend(outputPort, destination, &list), "MIDISend")
    }

    // MARK: - Endpoint scanning

    private func rescan() {
        // Sources: connect any newly-appeared ones (idempotent set) and note which
        // physical devices we receive from — used to pair the output endpoint.
        let sourceCount = MIDIGetNumberOfSources()
        var liveSources = Set<MIDIEndpointRef>()
        var sourceNames: [String] = []
        for i in 0..<sourceCount {
            let source = MIDIGetSource(i)
            liveSources.insert(source)
            sourceNames.append(Self.displayName(source))
            if !connectedSources.contains(source) {
                if check(MIDIPortConnectSource(inputPort, source, nil), "MIDIPortConnectSource") {
                    connectedSources.insert(source)
                }
            }
        }
        connectedSources.formIntersection(liveSources)
        let sourceDevices = Set(liveSources.map { Self.deviceRef(of: $0) }.filter { $0 != 0 })

        // Destinations: choose a robust target instead of just destination[0].
        // Prefer a bidirectional port on a device we also receive from (the module
        // we both send to and hear from), then real hardware over software buses
        // (IAC / Network Session), skipping offline endpoints. Groundwork for
        // multi-device selection (M7).
        let destinationInfos = (0..<MIDIGetNumberOfDestinations()).map { i -> EndpointInfo in
            let ref = MIDIGetDestination(i)
            return EndpointInfo(
                ref: ref, name: Self.displayName(ref),
                device: Self.deviceRef(of: ref), offline: Self.isOffline(ref))
        }
        let chosen = Self.selectDestination(from: destinationInfos, sourceDevices: sourceDevices)
        destination = chosen?.ref ?? 0
        let destinationNames = destinationInfos.map(\.name)

        log.info(
            """
            MIDI scan: sources=\(sourceNames, privacy: .public) \
            destinations=\(destinationNames, privacy: .public) \
            → chosen=\(chosen?.name ?? "none", privacy: .public)
            """
        )
        onDevices?(sourceNames, destinationNames)

        // Available only when we have a real endpoint to send to.
        let nowAvailable = !liveSources.isEmpty && destination != 0
        if nowAvailable != available {
            available = nowAvailable
            log.info("MIDI availability: \(nowAvailable)")
            onConnectionChange?(nowAvailable)
        }
    }

    // MARK: - Destination selection (pure policy + CoreMIDI lookups)

    /// A snapshot of one MIDI endpoint, decoupled from live CoreMIDI state so the
    /// selection policy below is pure and unit-testable (the Simulator has no
    /// endpoints, so this is the only way to cover the logic).
    struct EndpointInfo: Equatable, Sendable {
        let ref: MIDIEndpointRef
        let name: String
        /// The owning device (0 for virtual endpoints with no entity).
        let device: MIDIDeviceRef
        let offline: Bool
    }

    /// Pick the destination to send to. Higher score wins; ties keep CoreMIDI's
    /// order (first wins). Returns `nil` only when there's nothing to send to.
    ///
    /// Scoring: +4 if it shares a device with a connected source (a true
    /// bidirectional port — the module we also hear from), +2 if it's real
    /// hardware (has an owning device), +1 if it isn't a software bus. Offline
    /// endpoints are dropped unless that would leave nothing.
    nonisolated static func selectDestination(
        from destinations: [EndpointInfo],
        sourceDevices: Set<MIDIDeviceRef>
    ) -> EndpointInfo? {
        let online = destinations.filter { !$0.offline }
        let pool = online.isEmpty ? destinations : online
        guard let first = pool.first else { return nil }

        func score(_ d: EndpointInfo) -> Int {
            var s = 0
            if d.device != 0 && sourceDevices.contains(d.device) { s += 4 }
            if d.device != 0 { s += 2 }
            if !isSoftwareBusName(d.name) { s += 1 }
            return s
        }

        var best = first
        var bestScore = score(first)
        for d in pool.dropFirst() {
            let s = score(d)
            if s > bestScore {
                best = d
                bestScore = s
            }
        }
        return best
    }

    /// Whether an endpoint name looks like a software bus (macOS IAC / Network
    /// Session) we'd rather avoid when real hardware is present.
    nonisolated static func isSoftwareBusName(_ name: String) -> Bool {
        let n = name.lowercased()
        return n.contains("iac") || n.contains("network session") || n.contains("network midi")
    }

    /// The device that owns an endpoint, or 0 for a virtual endpoint with no
    /// entity (e.g. a software-created port).
    private static func deviceRef(of endpoint: MIDIEndpointRef) -> MIDIDeviceRef {
        var entity = MIDIEntityRef()
        guard MIDIEndpointGetEntity(endpoint, &entity) == noErr, entity != 0 else { return 0 }
        var device = MIDIDeviceRef()
        guard MIDIEntityGetDevice(entity, &device) == noErr else { return 0 }
        return device
    }

    /// Whether CoreMIDI currently reports the endpoint as offline (disconnected).
    private static func isOffline(_ endpoint: MIDIEndpointRef) -> Bool {
        var value: Int32 = 0
        let status = MIDIObjectGetIntegerProperty(endpoint, kMIDIPropertyOffline, &value)
        return status == noErr && value != 0
    }

    // MARK: - Helpers

    /// Human-readable endpoint name (e.g. the module's USB MIDI port name).
    private static func displayName(_ endpoint: MIDIEndpointRef) -> String {
        var value: Unmanaged<CFString>?
        let status = MIDIObjectGetStringProperty(endpoint, kMIDIPropertyDisplayName, &value)
        if status == noErr, let name = value?.takeRetainedValue() {
            return name as String
        }
        return "endpoint \(endpoint)"
    }

    /// Flatten a CoreMIDI packet list into a single byte buffer. `nonisolated`:
    /// CoreMIDI invokes the read block on its own thread, so this pure parsing
    /// must not be main-actor-isolated (the result is hopped to the main actor by
    /// the caller). Without this, the isolation check traps (EXC_BREAKPOINT).
    nonisolated private static func bytes(from packetList: UnsafePointer<MIDIPacketList>) -> Data {
        var data = Data()
        var packet = packetList.pointee.packet
        for _ in 0..<packetList.pointee.numPackets {
            let length = Int(packet.length)
            withUnsafeBytes(of: packet.data) { raw in
                data.append(contentsOf: raw.prefix(length))
            }
            packet = MIDIPacketNext(&packet).pointee
        }
        return data
    }

    @discardableResult
    private func check(_ status: OSStatus, _ what: String) -> Bool {
        if status != noErr {
            log.error("\(what) failed: \(status)")
            return false
        }
        return true
    }
}
