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
}
