import SwiftUI

/// App entry point. Owns the single `CoreSession` (the bridge to the Rust core)
/// and injects it into the view tree.
@main
struct TactusApp: App {
    @StateObject private var session = CoreSession()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(session)
        }
    }
}
