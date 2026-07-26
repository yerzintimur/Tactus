import SwiftUI
import Tactus

/// Read and rearrange a set list — the module's own ordering of kits, which on the
/// V31 is otherwise reachable only through the screen.
///
/// Built eyes-closed first. Every step is one accessibility element that reads
/// "Step 3: 12 · Funk", and its edits are **custom actions** (move up, move down,
/// remove) rather than separate buttons: a screen-reader user reaches them from
/// the row itself, without hunting for controls that belong to the row they just
/// left. Adding uses the kit the module is already on, so building a set list is
/// "play a kit, keep it" — no kit picker to navigate blind.
///
/// Nothing here is announced: the user is reading this list, so the screen reader
/// voices the result itself (ADR-0014). Failures still speak — they come from the
/// core as errors.
struct SetlistScreen: View {
    @EnvironmentObject private var session: CoreSession
    @State private var showingRename = false
    /// Which set list is open (0-based). The module holds 32; their names live in
    /// the module, so the picker offers numbers and the name appears once read.
    @State private var number: UInt32 = 0

    private static let setlistCount: UInt32 = 32

    var body: some View {
        List {
            Section {
                Picker(session.text(.sectionSetlist), selection: $number) {
                    ForEach(0..<Self.setlistCount, id: \.self) { index in
                        Text(session.text(.valueSetlistNumber, "\(index + 1)")).tag(index)
                    }
                }
                .accessibilityIdentifier("setlist-picker")

                if let setlist = session.setlist, !setlist.name.isEmpty {
                    LabeledContent(session.text(.labelSetlistName), value: setlist.name)
                }
                Button(session.text(.buttonRenameSetlist)) { showingRename = true }
                    .disabled(session.setlist == nil)
            }

            Section {
                if let setlist = session.setlist, !setlist.steps.isEmpty {
                    ForEach(Array(setlist.steps.enumerated()), id: \.offset) { position, kit in
                        stepRow(position: position, kit: kit, count: setlist.steps.count)
                    }
                } else {
                    Text(session.text(.valueSetlistEmpty))
                }

                if let kit = session.currentKitNumber, canAdd {
                    Button(session.text(.buttonAddCurrentKit)) {
                        session.appendSetlistStep(kit: kit)
                    }
                }
            }
        }
        .navigationTitle(session.text(.sectionSetlist))
        .task(id: number) { session.readSetlist(number) }
        .sheet(isPresented: $showingRename) {
            RenameSetlistView(currentName: session.setlist?.name ?? "")
                .environmentObject(session)
        }
    }

    /// One step: a single element reading "Step 1: 5 · Jazz", carrying its own
    /// edits as custom actions. `.swipeActions` gives sighted users the gesture and
    /// VoiceOver the same three actions from the rotor — one definition, both.
    @ViewBuilder private func stepRow(position: Int, kit: KitRef, count: Int) -> some View {
        let step = UInt32(position)
        Text(label(position: position, kit: kit))
            .accessibilityElement(children: .combine)
            .accessibilityLabel(label(position: position, kit: kit))
            .swipeActions(edge: .leading) {
                if position > 0 {
                    Button(session.text(.buttonMoveStepUp)) {
                        session.swapSetlistSteps(step, step - 1)
                    }
                }
                if position + 1 < count {
                    Button(session.text(.buttonMoveStepDown)) {
                        session.swapSetlistSteps(step, step + 1)
                    }
                }
            }
            .swipeActions(edge: .trailing) {
                Button(session.text(.buttonRemoveStep), role: .destructive) {
                    session.removeSetlistStep(step)
                }
            }
    }

    /// "Step 1: 5 · Jazz" — the position the user counts from 1, then the kit as it
    /// reads everywhere else in the app.
    private func label(position: Int, kit: KitRef) -> String {
        let name = kit.name.isEmpty ? "" : " · \(kit.name)"
        return session.text(.valueSetlistStep, "\(position + 1): \(kit.displayNumber)\(name)")
    }

    /// A set list holds a fixed number of steps; a full one can't take another.
    private var canAdd: Bool {
        guard let setlist = session.setlist else { return false }
        return setlist.steps.count < Int(setlist.capacity)
    }
}

/// Rename the open set list. Mirrors `RenameKitView` — the same shape, so the
/// gesture a user learned on kits works here too.
struct RenameSetlistView: View {
    @EnvironmentObject private var session: CoreSession
    @Environment(\.dismiss) private var dismiss

    @State private var name: String

    init(currentName: String) {
        _name = State(initialValue: currentName)
    }

    var body: some View {
        NavigationStack {
            Form {
                TextField(session.text(.labelSetlistName), text: $name)
                    .accessibilityLabel(session.text(.labelSetlistName))
                    .submitLabel(.done)
                    .onSubmit(save)
            }
            .navigationTitle(session.text(.titleRenameSetlist))
            #if os(iOS)
                .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(session.text(.buttonCancel)) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(session.text(.buttonSave), action: save)
                }
            }
        }
    }

    private func save() {
        session.renameSetlist(to: name)
        dismiss()
    }
}
