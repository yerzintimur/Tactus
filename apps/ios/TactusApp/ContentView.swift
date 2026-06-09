import SwiftUI
import Tactus

/// The accessible MVP screen. Every control is labelled for VoiceOver and usable
/// eyes-closed; spoken feedback on changes comes from the core via SpeechService.
///
/// MVP scope: connection status, kit navigation (previous/next), and kit rename —
/// the flows the core verifies end-to-end. Parameter editing (tempo, …) follows
/// once the core surfaces current values.
struct ContentView: View {
    @EnvironmentObject private var session: CoreSession
    @State private var showingRename = false

    /// UI-test mode: auto-connect and hide the DEBUG developer controls so the
    /// accessibility audit runs against the shipping UI.
    static let isUITest = ProcessInfo.processInfo.arguments.contains("--uitest")

    var body: some View {
        NavigationStack {
            List {
                connectionSection
                if session.connection == .ready {
                    kitSection
                }
                #if DEBUG
                if !Self.isUITest {
                    developerSection
                }
                #endif
            }
            .navigationTitle("Tactus")
            .task {
                session.startMidi()
                if Self.isUITest {
                    // Drive the pipeline so the audit runs on the real ready-state
                    // UI without the DEBUG developer controls.
                    session.connected()
                    session.receive(CoreSession.sampleV31IdentityReply)
                }
            }
            .sheet(isPresented: $showingRename) {
                RenameKitView(
                    number: session.currentKitNumber ?? 0,
                    currentName: session.currentKit ?? ""
                )
                .environmentObject(session)
            }
        }
    }

    // MARK: - Connection

    @ViewBuilder private var connectionSection: some View {
        Section("Connection") {
            LabeledContent("Status", value: connectionText)
            if let device = session.device {
                LabeledContent("Device", value: device.name)
                LabeledContent("Firmware", value: device.firmware)
                if let warning = firmwareWarning(device.firmwareSupport) {
                    Label(warning, systemImage: "exclamationmark.triangle")
                        .accessibilityLabel(warning)
                }
            } else if session.connection != .ready {
                Text("Connect your Roland V31 with a USB-C cable.")
            }
        }
    }

    // MARK: - Kit

    @ViewBuilder private var kitSection: some View {
        Section("Kit") {
            LabeledContent("Current kit", value: kitText)
                .accessibilityLabel("Current kit: \(kitText)")

            // Full-width prominent buttons: high contrast (white on accent),
            // large eyes-closed targets, and no label clipping.
            // Full-width prominent buttons: large eyes-closed targets with no
            // label clipping.
            Button {
                session.previousKit()
            } label: {
                Text("Previous kit").frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled((session.currentKitNumber ?? 0) == 0)

            Button {
                session.nextKit()
            } label: {
                Text("Next kit").frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)

            Button("Rename kit…") { showingRename = true }
                .accessibilityHint("Edit the name of the current kit")
        }
    }

    // MARK: - Developer (DEBUG only)

    #if DEBUG
    @ViewBuilder private var developerSection: some View {
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
    #endif

    // MARK: - Derived text

    private var connectionText: String {
        switch session.connection {
        case .disconnected: "Disconnected"
        case .identifying: "Identifying…"
        case .ready: "Ready"
        }
    }

    private var kitText: String {
        let name = session.currentKit ?? "—"
        if let number = session.currentKitNumber {
            return "\(Int(number) + 1) · \(name)"
        }
        return name
    }

    private func firmwareWarning(_ support: FirmwareSupport) -> String? {
        switch support {
        case .tested:
            nil
        case .untestedNewer:
            "This firmware is newer than we've tested. Everything should still work."
        case .untestedOlder:
            "This firmware is older than we've tested. Everything should still work."
        case .unknown:
            "This firmware hasn't been tested. Everything should still work."
        }
    }
}

/// Sheet for renaming the current kit. The core verifies the write and announces
/// the actual stored name (no blind writes), so we just submit and dismiss.
struct RenameKitView: View {
    @EnvironmentObject private var session: CoreSession
    @Environment(\.dismiss) private var dismiss

    let number: UInt32
    @State private var name: String

    init(number: UInt32, currentName: String) {
        self.number = number
        _name = State(initialValue: currentName)
    }

    var body: some View {
        NavigationStack {
            Form {
                TextField("Kit name", text: $name)
                    .accessibilityLabel("Kit name")
                    .submitLabel(.done)
                    .onSubmit(save)
            }
            .navigationTitle("Rename kit")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save", action: save)
                }
            }
        }
    }

    private func save() {
        session.renameKit(number, to: name)
        dismiss()
    }
}

#Preview {
    ContentView().environmentObject(CoreSession())
}
