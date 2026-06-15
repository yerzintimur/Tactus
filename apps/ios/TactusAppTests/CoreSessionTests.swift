import CoreMIDI
import Tactus
import XCTest

@testable import TactusApp

/// Verifies the Swift↔Rust integration and how CoreSession projects the core's
/// effects into UI state. The protocol itself is covered by the Rust test suite;
/// this pins the boundary.
@MainActor
final class CoreSessionTests: XCTestCase {
    func testIdentityReplyIdentifiesV31() {
        let session = CoreSession(locale: "en")

        session.connected()
        XCTAssertEqual(session.connection, .identifying)

        session.receive(CoreSession.sampleV31IdentityReply)

        XCTAssertEqual(session.device?.name, "Roland V31")
        XCTAssertEqual(session.device?.recognized, true)
        XCTAssertEqual(session.connection, .ready)
    }

    func testDisconnectClearsConnection() {
        let session = CoreSession(locale: "en")
        session.connected()
        session.receive(CoreSession.sampleV31IdentityReply)
        XCTAssertEqual(session.connection, .ready)

        session.disconnected()
        XCTAssertEqual(session.connection, .disconnected)
    }

    func testPreviousKitClampsAtZero() {
        let session = CoreSession(locale: "en")
        // No current kit yet → previousKit is a no-op (no crash, stays nil).
        session.previousKit()
        XCTAssertNil(session.currentKitNumber)
    }

    func testTempoIsProjectedFromTheSnapshot() {
        let session = CoreSession(locale: "en")
        session.connected()
        session.receive(CoreSession.sampleV31IdentityReply)

        // The V31 profile exposes tempo with its metadata even before a value has
        // been read back over MIDI (no transport responds in this harness).
        let tempo = try? XCTUnwrap(session.tempo)
        XCTAssertEqual(tempo?.label, "Tempo")
        XCTAssertEqual(tempo?.kind, .numeric)
        let range = tempo?.numeric?.range
        XCTAssertEqual(range?.rawMin, 200)
        XCTAssertEqual(range?.rawMax, 2600)
        XCTAssertEqual(range?.displayStep, 0.1)
        // No value yet → adjust is a safe no-op (no crash, nothing reported).
        XCTAssertNil(session.tempoRawValue)
        session.adjustTempo(rawSteps: 1)
    }

    // MARK: - MIDI destination selection policy

    private func endpoint(
        _ ref: MIDIEndpointRef, _ name: String, device: MIDIDeviceRef, offline: Bool = false
    ) -> MidiTransport.EndpointInfo {
        MidiTransport.EndpointInfo(ref: ref, name: name, device: device, offline: offline)
    }

    func testDestinationPrefersBidirectionalPairedDevice() {
        let iac = endpoint(1, "IAC Driver Bus 1", device: 99)
        let synth = endpoint(2, "Other Synth", device: 20)
        let module = endpoint(3, "V31", device: 10)
        // We receive from device 10, so its destination (the module) wins.
        let chosen = MidiTransport.selectDestination(
            from: [iac, synth, module], sourceDevices: [10])
        XCTAssertEqual(chosen, module)
    }

    func testDestinationSkipsOfflineAndDeprioritizesSoftwareBus() {
        let offlineModule = endpoint(1, "V31", device: 10, offline: true)
        let iac = endpoint(2, "IAC Driver Bus 1", device: 99)
        let synth = endpoint(3, "Other Synth", device: 20)
        // Offline V31 dropped; with no known source device, real hardware beats
        // the software bus.
        let chosen = MidiTransport.selectDestination(
            from: [iac, synth, offlineModule], sourceDevices: [])
        XCTAssertEqual(chosen, synth)
    }

    func testDestinationFallsBackToOfflineRatherThanNothing() {
        let only = endpoint(1, "V31", device: 10, offline: true)
        let chosen = MidiTransport.selectDestination(from: [only], sourceDevices: [10])
        XCTAssertEqual(chosen, only)
    }

    func testDestinationIsNilWhenNoneExist() {
        XCTAssertNil(MidiTransport.selectDestination(from: [], sourceDevices: [10]))
    }

    func testSoftwareBusNameDetection() {
        XCTAssertTrue(MidiTransport.isSoftwareBusName("IAC Driver Bus 1"))
        XCTAssertTrue(MidiTransport.isSoftwareBusName("Network Session 1"))
        XCTAssertFalse(MidiTransport.isSoftwareBusName("Roland V31"))
    }
}
