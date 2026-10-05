# ADR-0016: A profile that has not run on a real module is read-only

**Status:** Accepted · **Date:** 2026-10-05

## Context
A device profile is data derived from Roland's documents
([ADR-0007](0007-device-profile-abstraction.md)). The V31 taught us how much the
documents leave out: the `-INF` floor's wire value, the `END` terminator, the
instrument banks, the Program Change on a kit change, the Transmit Edit Data
default — every one of them came from the bench, not the PDF
([docs/PROTOCOL.md](../PROTOCOL.md)). The TD-17 profile is the first we write
**without the module**: a blind drum teacher has one, we do not, and we will not
before release ([docs/devices/roland-td-17.md](../devices/roland-td-17.md)).

Reading from a map that is slightly wrong is harmless: a wrong offset reads a
wrong value, and the cross-check tests and other projects' hardware-tested
schemas make even that unlikely. **Writing is different.** Our "no blind writes"
rule — write, read back, verify, speak the stored value — reads the address it
wrote. A wrong offset therefore passes its own verification while changing a
neighbouring parameter on someone else's instrument, silently and persistently.
No read-back can catch that; only the module can.

## Decision
Every profile declares whether its map has been exercised on hardware:

```jsonc
"verification": {
  "on_hardware": false,
  "basis": "what the map rests on — bench sessions with dates, or the sources it was checked against"
}
```

Absent means `false`. For a profile that is not verified on hardware the core:

1. **Recognises the module and reads everything** the profile describes — kit
   names, values, panel edits with Transmit Edit Data — and announces it, with
   the same speech as for any other module.
2. **Sends no DT1, ever.** Every edit, kit selection and rename is refused at
   the one gate all writes pass, with a spoken reason that names the profile
   and says why. The refusal is an `EditFailed` like any other, so a UI can
   present its editors as read-only and the screen reader hears the same line.
3. **Says so at connect**, in place of the firmware caveat: the unverified map
   is the larger uncertainty and implies the smaller one.

The flag flips to `true` in data, after a bench session has confirmed a write,
its read-back and its persistence — or after a dump diffed against a changed
value on the module's screen, the way PulsoKit confirms a map. Nothing in code
changes when it does.

## Consequences
- The TD-17 ships recognised and readable from the first release, which is
  already worth having: a blind drummer hears their kit names, values and what
  a knob on the panel just did. Editing arrives with the first verified module.
- A contributor with a module can verify a profile without touching code: run
  the checklist in [HARDWARE_TESTING.md](../HARDWARE_TESTING.md), flip the flag,
  record the evidence in `basis`.
- The gate sits in the core, not the UI, so both platforms inherit it and no
  screen can forget it.
- Kit navigation on an unverified module means changing kits on the module
  itself; the app follows. That is the one feature a read-only profile
  visibly lacks, and it is the right one to lack: the current-kit byte is a
  write like any other.
