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
        if ProcessInfo.processInfo.arguments.contains("--simulated-device") {
            return CoreSession(transport: SimulatedTransport())
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
