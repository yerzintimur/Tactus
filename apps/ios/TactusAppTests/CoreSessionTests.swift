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
}
