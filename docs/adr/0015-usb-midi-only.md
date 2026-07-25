# ADR-0015: USB MIDI is the only supported transport (BLE-MIDI deferred)

**Status:** Accepted · **Date:** 2026-07-25

## Context
Both modules we target can be reached two ways. **USB MIDI** is class-compliant on
each: plug the module into the phone (through Apple's camera/USB adapter) or a
computer and it appears as an ordinary MIDI endpoint, with no pairing step and no
permission prompt. **Bluetooth LE MIDI** also exists — the TD-17 documents it
explicitly (Bluetooth 4.2, GATT), and it is attractive on paper because it needs
no adapter at all ([docs/devices/roland-td-17.md](../devices/roland-td-17.md)).

Earlier drafts of the spec called USB "primary" and BLE "secondary" — a hedge that
left BLE permanently half-planned: it appeared in the requirements, the platform
notes, the permission lists and the backlog, without anyone owning it. That is the
worst of both worlds, because every one of those mentions is a promise we would
have to test on hardware, for two platforms, before a blind user could rely on it.

## Decision
**USB MIDI is the only transport Tactus builds, tests, documents and promises**,
on both iOS and Android, for both the V31 and the TD-17. BLE-MIDI is **deferred**,
not planned-but-unbuilt.

Concretely:

1. **No BLE code and no Bluetooth permissions.** iOS ships no
   `CABTMIDICentralViewController` and **no `NSBluetoothAlwaysUsageDescription`**;
   Android declares no `bluetooth_le` feature and no `BLUETOOTH_SCAN` /
   `BLUETOOTH_CONNECT` permissions.
2. **We do not block BLE endpoints.** CoreMIDI (and `android.media.midi`) present
   an already-paired BLE-MIDI device as an ordinary endpoint. If a user pairs one
   in the OS settings it will show up in our device list and will most likely
   work. That is *unsupported and untested*, not forbidden — writing code to
   filter it out would cost effort to make a working setup stop working.
3. **The docs say "USB", not "USB first".** No transport ranking, no
   nice-to-have BLE row in a requirements list.

## Rationale
- **Nonvisual-first cuts against BLE here** ([ADR-0006](0006-nonvisual-first.md)).
  A cable that is either in or out is a state you can feel. Pairing screens,
  permission dialogs, a device that silently drops mid-session and re-pairs to
  something else — those are exactly the failure modes that are hardest to
  diagnose without sight, and they sit in front of the user before our
  accessible UI ever gets a chance to help.
- **USB needs no permission at all**, so the first run has one fewer dialog to
  navigate — and a permission we never request is a permission that can never be
  denied by accident.
- **Scope.** BLE means two platform implementations, two permission stories, and
  hardware verification that SysEx survives BLE's fragmentation, all before it
  helps anyone. USB already works on the V31 today.
- Tactus edits parameters between songs; it is not a performance instrument where
  a cable would be in the way.

## Consequences
- **On the TD-17, USB is the only remaining path**, because the module has **no
  MIDI IN jack** at all. In practice that is a cable, not a dongle: USB-C→USB-B
  from an Android phone or a USB-C iPhone/iPad; only Lightning devices need
  Apple's camera adapter. The module must be set to **USB Driver Mode = GENERIC**
  so the OS class-compliant driver is used — the same setting the V31 needs.
- The hardware-test list loses "does SysEx survive BLE fragmentation"; the
  remaining TD-17 questions are all about the module, not the link.
- If we revisit: the transport seam is already the right shape — the core is
  sans-I/O and the native layer owns the endpoint
  ([ADR-0008](0008-sans-io-core-and-i18n.md)), so BLE would be a native-side
  addition, not a core change. Nothing in this decision paints us into a corner.
- **Revisit when** a blind user tells us the cable is the barrier — that evidence
  outranks the reasoning above.
