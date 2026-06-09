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
        // Connect any newly-appeared sources to our input port (idempotent set).
        let sourceCount = MIDIGetNumberOfSources()
        var liveSources = Set<MIDIEndpointRef>()
        for i in 0..<sourceCount {
            let source = MIDIGetSource(i)
            liveSources.insert(source)
            if !connectedSources.contains(source) {
                if check(MIDIPortConnectSource(inputPort, source, nil), "MIDIPortConnectSource") {
                    connectedSources.insert(source)
                }
            }
        }
        connectedSources.formIntersection(liveSources)

        // Pick the first destination as the output target. Multi-device selection
        // comes in task #20.
        let destinationCount = MIDIGetNumberOfDestinations()
        destination = destinationCount > 0 ? MIDIGetDestination(0) : 0

        let nowAvailable = sourceCount > 0 && destinationCount > 0
        if nowAvailable != available {
            available = nowAvailable
            log.info("MIDI availability: \(nowAvailable)")
            onConnectionChange?(nowAvailable)
        }
    }

    // MARK: - Helpers

    /// Flatten a CoreMIDI packet list into a single byte buffer.
    private static func bytes(from packetList: UnsafePointer<MIDIPacketList>) -> Data {
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
