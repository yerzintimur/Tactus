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
    /// Which set list is open (0-based). The names live in the module; the core
    /// reads them in the background after connect, so a row says "Set list 3 ·
    /// Rock Night" once its name is known and "Set list 3" until then.
    @State private var number: UInt32 = 0

    var body: some View {
        List {
            Section {
                // Not a Picker: its row keeps the chosen value on one short
                // trailing line, and "Set list 3 · Rock Night" gets cut off
                // there. A two-line row gives the name the full width.
                NavigationLink {
                    SetlistChooser(selection: $number)
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(session.text(.sectionSetlist))
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        Text(session.text(.valueSetlistNumber, Self.rowValue(current)))
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("setlist-picker")

                if let setlist = session.setlist, !setlist.name.isEmpty {
                    LabeledContent(session.text(.labelSetlistName), value: setlist.name)
                }
                Button(session.text(.buttonRenameSetlist)) { showingRename = true }
                    .disabled(session.setlist == nil)
            }

            // Playing the set: the module keeps no list position of its own, so
            // these drive it — each press is a verified kit selection, announced
            // as "Step 2, Kit 5: Jazz" once the module confirms.
            Section {
                Button(session.text(.buttonPreviousStep)) { session.previousSetlistStep() }
                    .disabled(session.setlist == nil)
                Button(session.text(.buttonNextStep)) { session.nextSetlistStep() }
                    .disabled(session.setlist == nil)
            }

            Section {
                if let setlist = session.setlist, !setlist.steps.isEmpty {
                    ForEach(Array(setlist.steps.enumerated()), id: \.offset) { position, kit in
                        stepRow(
                            position: position, kit: kit, count: setlist.steps.count,
                            isCurrent: setlist.position.map(Int.init) == position)
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
    @ViewBuilder private func stepRow(
        position: Int, kit: KitRef, count: Int, isCurrent: Bool
    ) -> some View {
        let step = UInt32(position)
        let text = label(position: position, kit: kit)
        // The step being played reads "…, current step" — the one orientation a
        // drummer needs mid-set, carried by the row itself rather than a marker.
        let spoken = isCurrent ? session.text(.valueSetlistCurrentStep, text) : text
        Text(spoken)
            .fontWeight(isCurrent ? .bold : .regular)
            .accessibilityElement(children: .combine)
            .accessibilityLabel(spoken)
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

    /// The chosen list as the core lists it, or a bare number before the core
    /// has a profile to count lists with.
    private var current: SetlistRef {
        session.setlists.first { $0.number == number }
            ?? SetlistRef(number: number, displayNumber: number + 1, name: "")
    }

    /// The number and, once read, the name — the same shape as a kit row.
    static func rowValue(_ list: SetlistRef) -> String {
        list.name.isEmpty
            ? "\(list.displayNumber)"
            : "\(list.displayNumber) · \(list.name)"
    }

    /// A set list holds a fixed number of steps; a full one can't take another.
    private var canAdd: Bool {
        guard let setlist = session.setlist else { return false }
        return setlist.steps.count < Int(setlist.capacity)
    }
}

/// Choose which set list to open: every list the module holds, named once the
/// core has read its name (in the background after connect), the open one
/// marked. One full-width row per list, so a name is never cut off.
struct SetlistChooser: View {
    @EnvironmentObject private var session: CoreSession
    @Environment(\.dismiss) private var dismiss
    @Binding var selection: UInt32

    var body: some View {
        List(session.setlists, id: \.number) { list in
            Button {
                selection = list.number
                dismiss()
            } label: {
                HStack {
                    Text(session.text(.valueSetlistNumber, SetlistScreen.rowValue(list)))
                    Spacer()
                    if list.number == selection {
                        Image(systemName: "checkmark").accessibilityHidden(true)
                    }
                }
            }
            .tint(.primary)
            .accessibilityAddTraits(list.number == selection ? .isSelected : [])
        }
        .navigationTitle(session.text(.sectionSetlist))
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
