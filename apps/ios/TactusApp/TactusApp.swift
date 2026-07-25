import SwiftUI

/// App entry point. Owns the single `CoreSession` (the bridge to the Rust core)
/// and injects it into the view tree.
@main
struct TactusApp: App {
    @StateObject private var session = TactusApp.makeSession()

    /// With `--simulated-device` (DEBUG; the UI tests pass it) the session talks
    /// to the core's simulated module instead of CoreMIDI, so the full pipeline
    /// runs with no hardware.
    private static func makeSession() -> CoreSession {
        #if DEBUG
        // The language override is deliberately persistent, which would otherwise
        // leak from whichever UI test set it into every test that runs after —
        // each `--uitest` launch starts from the device language.
        if ProcessInfo.processInfo.arguments.contains("--uitest") {
            CoreSession.clearLanguageOverride()
        }
        // `--language ru` starts in a given language, so a UI test can assert the
        // whole interface in it without driving the system picker widget.
        let arguments = ProcessInfo.processInfo.arguments
        let language = arguments.firstIndex(of: "--language").map { arguments[$0 + 1] }
        if ProcessInfo.processInfo.arguments.contains("--simulated-device") {
            return CoreSession(
                locale: language ?? CoreSession.preferredLanguage(),
                transport: SimulatedTransport())
        }
        if let language {
            return CoreSession(locale: language)
        }
        #endif
        return CoreSession()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(session)
        }
    }
}
