// Raw SysEx probe for hardware sessions (macOS, CoreMIDI): send Roland messages to
// the connected module and print every SysEx it sends back, with timestamps. This
// is how set-list reads/writes were verified on the V31 (PROTOCOL §5) when the app
// itself could not be driven. Not part of the app; nothing here is device-specific
// except the Roland framing (the model id below is the V31's).
//
//   swiftc -O -o midiprobe tools/midiprobe.swift
//   midiprobe <wait_ms> [<message> ...]
//
// A message is "RQ1 aa aa aa aa ss ss ss ss" or "DT1 aa aa aa aa dd dd ..." (the
// checksum is added), or raw hex bytes. Messages go out 60 ms apart; the tool keeps
// listening for <wait_ms> after the last one, so `midiprobe 60000` alone is a
// one-minute monitor for unsolicited traffic (Transmit Edit Data).
import CoreMIDI
import Foundation

let model: [UInt8] = [0x41, 0x10, 0x01, 0x06, 0x01]

func checksum(_ body: [UInt8]) -> UInt8 {
    let sum = body.reduce(0) { ($0 + Int($1)) % 128 }
    return UInt8((128 - sum) % 128)
}

func build(_ spec: String) -> [UInt8] {
    let parts = spec.split(separator: " ").map(String.init)
    guard let first = parts.first else { return [] }
    let bytes = parts.dropFirst().compactMap { UInt8($0, radix: 16) }
    switch first.uppercased() {
    case "RQ1": return [0xF0] + model + [0x11] + bytes + [checksum(bytes), 0xF7]
    case "DT1": return [0xF0] + model + [0x12] + bytes + [checksum(bytes), 0xF7]
    default: return parts.compactMap { UInt8($0, radix: 16) }
    }
}

func hex(_ b: [UInt8]) -> String { b.map { String(format: "%02X", $0) }.joined(separator: " ") }

func name(_ obj: MIDIObjectRef) -> String {
    var s: Unmanaged<CFString>?
    MIDIObjectGetStringProperty(obj, kMIDIPropertyDisplayName, &s)
    return s?.takeRetainedValue() as String? ?? "?"
}

var client = MIDIClientRef()
MIDIClientCreateWithBlock("midiprobe" as CFString, &client) { _ in }

func stamp() -> String {
    String(format: "%.3f", Date().timeIntervalSince1970.truncatingRemainder(dividingBy: 1000))
}

// SysEx is reassembled across packets; channel messages (program change, bank
// select, …) are printed as they come, so a kit change shows up even when the
// module pushes no SysEx for it. Real-time bytes (F8–FF) are dropped.
var buffer: [UInt8] = []
var inSysex = false
var inPort = MIDIPortRef()
MIDIInputPortCreateWithBlock(client, "in" as CFString, &inPort) { packetList, _ in
    for packet in packetList.unsafeSequence() {
        let bytes = Array(packet.bytes())
        var channel: [UInt8] = []
        for b in bytes {
            if b >= 0xF8 { continue }
            if b == 0xF0 { buffer = []; inSysex = true }
            if inSysex {
                buffer.append(b)
                if b == 0xF7 {
                    print("\(stamp()) ← \(hex(buffer))")
                    buffer = []
                    inSysex = false
                }
            } else {
                channel.append(b)
            }
        }
        if !channel.isEmpty { print("\(stamp()) ← \(hex(channel))   (channel)") }
        fflush(stdout)
    }
}

var source: MIDIEndpointRef = 0
var dest: MIDIEndpointRef = 0
for i in 0..<MIDIGetNumberOfSources() where name(MIDIGetSource(i)).contains("V31") { source = MIDIGetSource(i) }
for i in 0..<MIDIGetNumberOfDestinations() where name(MIDIGetDestination(i)).contains("V31") { dest = MIDIGetDestination(i) }
guard source != 0, dest != 0 else { print("V31 endpoints not found"); exit(1) }
MIDIPortConnectSource(inPort, source, nil)

var outPort = MIDIPortRef()
MIDIOutputPortCreate(client, "out" as CFString, &outPort)

let args = Array(CommandLine.arguments.dropFirst())
let waitMs = Int(args.first ?? "500") ?? 500
for spec in args.dropFirst() {
    let msg = build(spec)
    var list = MIDIPacketList()
    let packet = MIDIPacketListInit(&list)
    _ = MIDIPacketListAdd(&list, 1024, packet, 0, msg.count, msg)
    MIDISend(outPort, dest, &list)
    let t = String(format: "%.3f", Date().timeIntervalSince1970.truncatingRemainder(dividingBy: 1000))
    print("\(t) → \(hex(msg))")
    fflush(stdout)
    RunLoop.current.run(until: Date().addingTimeInterval(0.06))
}
RunLoop.current.run(until: Date().addingTimeInterval(Double(waitMs) / 1000))
