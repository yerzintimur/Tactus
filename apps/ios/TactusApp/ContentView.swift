import SwiftUI
import Tactus

/// Skeleton screen: proves the Rust core is wired up and running on-device.
/// This is scaffolding (task #12) — the accessible MVP UI comes in task #16.
/// The "Developer" section drives the pipeline without hardware until the
/// CoreMIDI transport (task #13) exists.
struct ContentView: View {
    @EnvironmentObject private var session: CoreSession

    var body: some View {
        NavigationStack {
            List {
                Section("Status") {
                    LabeledContent("Connection", value: statusText)
                    if let device = session.device {
                        LabeledContent("Device", value: device.name)
                        LabeledContent("Firmware", value: device.firmware)
                        LabeledContent("Firmware support", value: supportText(device.firmwareSupport))
                    }
                    if let kit = session.currentKit {
                        LabeledContent("Current kit", value: kit)
                    }
                }

                if !session.lastSpoken.isEmpty {
                    Section("Last announcement") {
                        Text(session.lastSpoken)
                            .accessibilityLabel("Last announcement: \(session.lastSpoken)")
                    }
                }

                Section("Developer") {
                    Button("Simulate connect") { session.connected() }
                    Button("Simulate V31 identity reply") {
                        session.receive(CoreSession.sampleV31IdentityReply)
                    }
                    Button("Simulate disconnect") { session.disconnected() }
                }

                Section("Event log") {
                    if session.log.isEmpty {
                        Text("No events yet.").foregroundStyle(.secondary)
                    } else {
                        ForEach(Array(session.log.enumerated()), id: \.offset) { _, line in
                            Text(line).font(.system(.footnote, design: .monospaced))
                        }
                    }
                }
            }
            .navigationTitle("Tactus")
            .task { session.startMidi() }
        }
    }

    private var statusText: String {
        switch session.connection {
        case .disconnected: "Disconnected"
        case .identifying: "Identifying…"
        case .ready: "Ready"
        }
    }

    private func supportText(_ support: FirmwareSupport) -> String {
        switch support {
        case .tested: "Tested"
        case .untestedNewer: "Untested (newer)"
        case .untestedOlder: "Untested (older)"
        case .unknown: "Unknown"
        }
    }
}

#Preview {
    ContentView().environmentObject(CoreSession())
}
