# Hardware testing & safety (Roland V31)

> Short answer: **testing Tactus against your V31 is safe — you will not brick it.**
> This page explains why (with sources) and gives a per-session safety protocol.

## Why it's safe

- Tactus speaks only **documented parameter SysEx** — `RQ1` (read) and `DT1`
  (write) plus the universal Identity Request/Reply. Per the *V31 MIDI
  Implementation* (v2.00): the module **does not receive System Reset** (`x/x` in
  the implementation chart), and the SysEx surface is **only the parameter address
  map** — there is **no firmware command**. The protocol literally cannot reach
  firmware.
- **Firmware is updated by a separate procedure** (a file on a USB stick + a
  startup key combo), confirmed by Roland for the sibling V51/V71. The one real
  "brick" risk — an interrupted *firmware* update — is unrelated to our protocol;
  Tactus never invokes it.
- **Worst realistic case = wrong parameter values or an overwritten kit.** Both are
  fully recoverable: the V31 has **BACKUP** (ALL 1–99, per-kit 1–999) and
  **FACTORY RESET** (V31 Reference Manual p.59).
- **Reading is risk-free.** `RQ1`/Identity cannot change anything on the module.
- **Prior art:** Jon Skeet's open-source *V-Drum Explorer* reads and writes Roland
  V-Drums (TD-17/27/50, …) parameters over exactly this `RQ1`/`DT1` + 7-bit
  address protocol — long-standing evidence this is safe.
- The only "caution" in Roland's own docs is about *very loud signals distorting
  the ambience/effects* — audio quality, not hardware damage.

## Per-session safety protocol

1. **Back up first**, on the module: `BACKUP` → ALL (and/or to USB). Factory Reset
   is the ultimate undo.
2. **Read-only first.** Identity + `RQ1` only — validates the round-trip and reads
   the current kit / name / tempo with zero risk.
3. **First writes go to a scratch kit** (duplicate one you don't care about), never
   your favourites; confirm every write with read-back.
4. **Throttle writes.** Roland paces SysEx as ≤256-byte packets ~20 ms apart; the
   engine does the same, which avoids buffer overload on the module.
5. **Never invoke a firmware update** — separate USB process; the app can't reach it.
6. Keep monitoring/headphone volume sane.

## How we debug

- **Cheapest first contact (Mac, read-only):** a generic MIDI monitor (e.g. *MIDI
  Monitor*) or a Web MIDI page in Chrome, with the V31 over USB. Send `RQ1`, watch
  the `DT1` replies — no app build, no writes.
- **On iPhone (the target):** the iOS transport (task #13) opens the V31 as a
  CoreMIDI endpoint (class-compliant USB MIDI — the only supported transport,
  [ADR-0015](adr/0015-usb-midi-only.md)). Start read-only to
  validate the protocol on real hardware (task #14), then enable writes behind the
  **write → read-back → verify** pipeline.
- Log raw SysEx as hex and compare against the golden vectors in
  [PROTOCOL.md](PROTOCOL.md).

## Verifying a profile written without the module

A profile built from the documents alone ships **read-only**: `verification.on_hardware`
is `false` and the core refuses every write
([ADR-0016](adr/0016-unverified-profiles-are-read-only.md)). The TD-17 is the
first such profile ([docs/devices/roland-td-17.md](devices/roland-td-17.md)).
To lift the flag, one session with the module, in this order — reads first, so
a wrong map is caught before anything is written:

1. **Identity.** Connect; the app must name the module and its firmware as the
   module's own screen does. Note the Identity Reply bytes in the log.
2. **Reads against the screen.** For kit 1 and one user kit: the kit name, the
   kit volume, a pad's instrument and layer volume, the reverb type — every one
   read by the app must match what the module shows. A mismatch is a map error:
   stop, fix the profile, re-read.
3. **Panel edits.** Turn Transmit Edit Data on; turn one knob; the app must name
   the pad, the layer and the parameter and speak the new value.
4. **One write.** Flip `on_hardware` to `true` locally, change one value on a
   user kit (kit volume by one step), read it back, confirm it on the module's
   screen, power-cycle, confirm it survived. Restore the original value.
5. **Record it.** Put the date, firmware and what was checked into
   `verification.basis`; commit the flag with the evidence.

If step 4 cannot be done — no user kit to spare, no time — the profile stays
read-only. That is a feature missing, not a module at risk.

## Sources

- Roland — *V31 MIDI Implementation* v2.00 (System Reset `x/x`; SysEx = RQ1/DT1 +
  Identity only) and *V31 Data List* (BACKUP, FACTORY RESET). Obtain via
  [docs/vendor/README.md](vendor/README.md).
- [V31 Reference Manual / error & function list (Roland)](https://static.roland.com/manuals/v31_reference_v200/en-US/411152779413004811.html)
- [V71: Restoring the factory settings (Factory Reset) — Roland](https://support.roland.com/hc/en-us/articles/30725770782491-V71-Restoring-the-factory-settings-Factory-Reset)
- [V51: System Program Update (firmware via USB) — Roland](https://support.roland.com/hc/en-us/articles/43879871340955-V51-System-Program-Update)
- [V-Drum Explorer — memory & 7-bit addressing (Jon Skeet)](https://codeblog.jonskeet.uk/2020/02/25/v-drum-explorer-memory-and-7-bit-addressing/) ·
  [docs](https://jskeet.github.io/DemoCode/Drums/)
- [Roland TD-50 MIDI Implementation (RQ1/DT1; ≤256-byte packets ~20 ms)](https://static.roland.com/assets/media/pdf/TD-50_MIDI_Imple_e03_W.pdf)
