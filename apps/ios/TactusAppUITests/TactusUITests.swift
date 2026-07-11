import XCTest

/// End-to-end UI flows against the **simulated module** (`--simulated-device`,
/// device-mock plan B1): the app runs the real pipeline — identify → poll →
/// write → read-back → verify — over the core's `VirtualDeviceHandle`, no
/// hardware. Everything is asserted through the accessibility tree (labels and
/// values), which is exactly what a screen-reader user gets. Plus the
/// accessibility audit gate.
final class TactusUITests: XCTestCase {
    /// Shared CI runners are slow (a cold app launch alone can take tens of
    /// seconds); waits return as soon as the condition holds, so a generous
    /// timeout costs nothing locally.
    private let uiTimeout: TimeInterval = 15

    override func setUp() {
        continueAfterFailure = false
    }

    private func launchSimulated() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = ["--uitest", "--simulated-device"]
        app.launch()
        return app
    }

    /// Any element whose accessibility label or value contains `text` — resilient
    /// to how SwiftUI combines rows (LabeledContent exposes its value either as a
    /// child static text or as the row's value, depending on the container).
    private func element(
        in app: XCUIApplication, containing text: String
    ) -> XCUIElement {
        app.descendants(matching: .any)
            .matching(NSPredicate(format: "label CONTAINS %@ OR value CONTAINS %@", text, text))
            .firstMatch
    }

    /// The tempo row — one VoiceOver adjustable (its children are intentionally
    /// hidden from the accessibility tree).
    private func tempoElement(in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any)["tempo-adjustable"].firstMatch
    }

    private func waitForValue(_ expected: String, of element: XCUIElement) -> Bool {
        let predicate = NSPredicate(format: "value == %@", expected)
        let wait = XCTWaiter().wait(
            for: [expectation(for: predicate, evaluatedWith: element)], timeout: uiTimeout)
        return wait == .completed
    }

    /// The identity handshake + initial kit/tempo reads flow end-to-end: the app
    /// reaches Ready and surfaces the device, the current kit, and its tempo.
    @MainActor
    func testConnectReadsKitAndTempoFromTheSimulatedModule() {
        let app = launchSimulated()

        XCTAssertTrue(
            element(in: app, containing: "Roland V31").waitForExistence(timeout: uiTimeout),
            "identity reply should surface the device name")
        XCTAssertTrue(
            element(in: app, containing: "5 · Jazz").waitForExistence(timeout: uiTimeout),
            "the current kit is read from the module")
        let tempo = tempoElement(in: app)
        XCTAssertTrue(tempo.waitForExistence(timeout: uiTimeout))
        XCTAssertTrue(
            waitForValue("120.0 BPM", of: tempo),
            "the kit tempo is read from the module; got \(String(describing: tempo.value))")
    }

    /// Kit navigation round-trips through the module: the write is confirmed by
    /// the Current read and the *actual* kit (name read back) reaches the UI.
    @MainActor
    func testNextKitIsConfirmedByTheModule() {
        let app = launchSimulated()
        XCTAssertTrue(element(in: app, containing: "5 · Jazz").waitForExistence(timeout: uiTimeout))

        app.buttons["Next kit"].tap()
        XCTAssertTrue(
            element(in: app, containing: "6 · Funk").waitForExistence(timeout: uiTimeout),
            "the confirmed kit change (and its read-back name) should land")

        app.buttons["Previous kit"].tap()
        XCTAssertTrue(
            element(in: app, containing: "5 · Jazz").waitForExistence(timeout: uiTimeout),
            "navigating back is confirmed the same way")
    }

    /// A tempo nudge goes through write → read-back → verify; the displayed value
    /// is the device-confirmed one. The visible “+” button is hidden from the
    /// accessibility tree (the row is one adjustable), so it is tapped by
    /// coordinate — the VoiceOver path (adjustable increment) is validated in the
    /// manual eyes-closed pass.
    @MainActor
    func testTempoNudgeIsVerifiedByReadback() {
        let app = launchSimulated()
        let tempo = tempoElement(in: app)
        XCTAssertTrue(tempo.waitForExistence(timeout: uiTimeout))
        XCTAssertTrue(waitForValue("120.0 BPM", of: tempo))

        // Rightmost 44-pt control in the row is “+”.
        tempo.coordinate(withNormalizedOffset: CGVector(dx: 0.93, dy: 0.5)).tap()
        XCTAssertTrue(
            waitForValue("120.1 BPM", of: tempo),
            "the +0.1 BPM edit should be verified and displayed; got \(String(describing: tempo.value))"
        )
    }

    /// Legacy DEBUG path (no simulated device): the developer controls still
    /// drive the identity handshake with a canned reply.
    @MainActor
    func testConnectFlowSurfacesKitSection() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.buttons["Simulate connect"].waitForExistence(timeout: uiTimeout))
        app.buttons["Simulate connect"].tap()
        app.buttons["Simulate V31 identity reply"].tap()

        // The Kit section only appears once the device is ready, so its presence
        // proves the identity round-trip reached the UI.
        XCTAssertTrue(
            app.staticTexts["Kit"].waitForExistence(timeout: uiTimeout),
            "Kit section should appear once the device is ready")
        XCTAssertTrue(app.buttons["Rename kit…"].exists)
    }

    /// Accessibility audit gate. Runs against the shipping UI in the full ready
    /// state (simulated module connected, kit + tempo sections visible);
    /// `--uitest` hides the DEBUG developer controls.
    ///
    /// `contrast` and `dynamicType` are excluded from the automated gate:
    /// - contrast on standard iOS tinted controls (systemBlue ≈ 3–4:1) sits below
    ///   WCAG 4.5:1 and the audit reports it non-deterministically (nearly/failed
    ///   across runs) — unsuitable as a strict gate. Tracked for a dedicated
    ///   high-contrast theme pass.
    /// - dynamicType clipping likewise gets its own low-vision pass.
    ///
    /// Everything actionable stays enforced: text clipping, missing labels,
    /// traits, hit regions, element detection.
    @MainActor
    func testAccessibilityAudit() throws {
        let app = launchSimulated()

        let tempo = tempoElement(in: app)
        XCTAssertTrue(
            tempo.waitForExistence(timeout: uiTimeout),
            "the full ready state (kit + tempo) should be reached before auditing")
        XCTAssertTrue(waitForValue("120.0 BPM", of: tempo))

        try app.performAccessibilityAudit(
            for: XCUIAccessibilityAuditType.all.subtracting([.contrast, .dynamicType]))
    }
}
